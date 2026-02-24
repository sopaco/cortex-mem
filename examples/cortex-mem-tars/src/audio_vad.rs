/// VAD (Voice Activity Detection) 语音活动检测模块
/// 
/// 改进点:
/// 1. 使用更清晰的状态机模式
/// 2. 移除了不必要的声纹识别功能(简化为单用户场景)
/// 3. 更好的错误处理和日志
use anyhow::Result;

/// VAD 检测器配置
#[derive(Debug, Clone)]
pub struct VadConfig {
    /// 采样率
    pub sample_rate: u32,
    /// 最小语音持续时长(毫秒)
    pub min_speech_duration_ms: u32,
    /// 静音阈值(毫秒) - 检测到静音后多久结束录音
    pub silence_threshold_ms: u32,
    /// 最大录音时长(毫秒)
    pub max_recording_duration_ms: u32,
    /// 音量阈值 - RMS 低于此值视为静音
    pub volume_threshold: f32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            min_speech_duration_ms: 1000,
            silence_threshold_ms: 2000,
            max_recording_duration_ms: 60000,
            volume_threshold: 0.0010,
        }
    }
}

/// VAD 检测状态
#[derive(Debug, Clone, PartialEq)]
enum VadState {
    /// 静音状态，等待语音开始
    Idle,
    /// 检测到语音，正在录音
    Recording {
        /// 已录制的采样数
        samples_count: usize,
        /// 连续静音的采样数
        silence_samples: usize,
    },
}

/// 语音段 - VAD 检测到的完整语音片段
#[derive(Debug, Clone)]
pub struct SpeechSegment {
    /// 音频数据
    pub samples: Vec<f32>,
    /// 时长(毫秒)
    pub duration_ms: u32,
    /// 平均音量(RMS)
    pub avg_volume: f32,
}

impl SpeechSegment {
    /// 计算音频的 RMS 音量
    fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt()
    }

    /// 从样本创建语音段
    fn new(samples: Vec<f32>, sample_rate: u32) -> Self {
        let duration_ms = ((samples.len() as u64 * 1000) / sample_rate as u64) as u32;
        let avg_volume = Self::calculate_rms(&samples);
        
        Self {
            samples,
            duration_ms,
            avg_volume,
        }
    }
}

/// VAD 检测器
pub struct VadDetector {
    config: VadConfig,
    state: VadState,
    speech_buffer: Vec<f32>,
    total_samples: usize,
}

impl VadDetector {
    /// 创建新的 VAD 检测器
    pub fn new(config: VadConfig) -> Self {
        log::info!("VAD 检测器初始化:");
        log::info!("  - 采样率: {} Hz", config.sample_rate);
        log::info!("  - 最小语音时长: {} ms", config.min_speech_duration_ms);
        log::info!("  - 静音阈值: {} ms", config.silence_threshold_ms);
        log::info!("  - 最大录音时长: {} ms", config.max_recording_duration_ms);
        log::info!("  - 音量阈值: {:.4}", config.volume_threshold);

        Self {
            config,
            state: VadState::Idle,
            speech_buffer: Vec::new(),
            total_samples: 0,
        }
    }

    /// 处理音频样本，返回完整的语音段(如果有)
    /// 
    /// 改进点:
    /// 1. 使用状态机模式，逻辑更清晰
    /// 2. 减少了重复代码
    /// 3. 更好的日志输出
    pub fn process(&mut self, samples: &[f32]) -> Option<SpeechSegment> {
        self.total_samples += samples.len();
        let rms = SpeechSegment::calculate_rms(samples);
        
        // 检查是否为语音
        let is_speech = rms > self.config.volume_threshold;
        
        // 状态机转换
        match &mut self.state {
            VadState::Idle => {
                if is_speech {
                    log::debug!("检测到语音开始 (RMS: {:.4})", rms);
                    self.state = VadState::Recording {
                        samples_count: 0,
                        silence_samples: 0,
                    };
                    self.speech_buffer.clear();
                    self.speech_buffer.extend_from_slice(samples);
                }
            }
            VadState::Recording {
                samples_count,
                silence_samples,
            } => {
                if is_speech {
                    // 继续录音
                    *silence_samples = 0;
                    self.speech_buffer.extend_from_slice(samples);
                    *samples_count += samples.len();
                } else {
                    // 检测到静音
                    *silence_samples += samples.len();
                    
                    let silence_duration_ms = 
                        (*silence_samples as u64 * 1000 / self.config.sample_rate as u64) as u32;
                    
                    // 检查是否达到静音阈值
                    if silence_duration_ms >= self.config.silence_threshold_ms {
                        log::debug!("静音阈值达到，结束录音");
                        return self.finish_recording();
                    }
                }
                
                // 检查最大录音时长
                let recording_duration_ms = 
                    (*samples_count as u64 * 1000 / self.config.sample_rate as u64) as u32;
                
                if recording_duration_ms >= self.config.max_recording_duration_ms {
                    log::debug!("达到最大录音时长，结束录音");
                    return self.finish_recording();
                }
            }
        }
        
        None
    }

    /// 完成录音，返回语音段(如果满足最小时长要求)
    fn finish_recording(&mut self) -> Option<SpeechSegment> {
        let duration_ms = 
            (self.speech_buffer.len() as u64 * 1000 / self.config.sample_rate as u64) as u32;
        
        self.state = VadState::Idle;
        self.total_samples = 0;
        
        if duration_ms >= self.config.min_speech_duration_ms {
            let segment = SpeechSegment::new(
                std::mem::take(&mut self.speech_buffer),
                self.config.sample_rate,
            );
            
            log::info!(
                "语音段检测完成: {} ms, {} 采样, RMS: {:.4}",
                segment.duration_ms,
                segment.samples.len(),
                segment.avg_volume
            );
            
            Some(segment)
        } else {
            log::debug!("语音段过短 ({} ms)，丢弃", duration_ms);
            self.speech_buffer.clear();
            None
        }
    }

    /// 重置检测器状态
    pub fn reset(&mut self) {
        self.state = VadState::Idle;
        self.speech_buffer.clear();
        self.total_samples = 0;
        log::debug!("VAD 检测器已重置");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_basic() {
        let mut config = VadConfig::default();
        config.sample_rate = 16000;
        config.min_speech_duration_ms = 100;
        config.silence_threshold_ms = 200;
        config.volume_threshold = 0.01;

        let mut detector = VadDetector::new(config);

        // 模拟静音
        let silence: Vec<f32> = vec![0.0; 1600]; // 100ms
        assert!(detector.process(&silence).is_none());

        // 模拟语音
        let speech: Vec<f32> = vec![0.1; 3200]; // 200ms
        assert!(detector.process(&speech).is_none());

        // 模拟静音结束
        let silence: Vec<f32> = vec![0.0; 3200]; // 200ms
        let segment = detector.process(&silence);
        assert!(segment.is_some());
        
        if let Some(seg) = segment {
            assert!(seg.duration_ms >= 200);
        }
    }

    #[test]
    fn test_rms_calculation() {
        let samples = vec![0.1, -0.1, 0.2, -0.2];
        let rms = SpeechSegment::calculate_rms(&samples);
        assert!(rms > 0.0);
        assert!(rms < 0.2);
    }
}
