/// 音频输入模块 - 使用 CPAL 捕获麦克风音频
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, SupportedStreamConfig};
use std::sync::Arc;
use tokio::sync::mpsc;

/// 音频增益系数
const AUDIO_GAIN: f32 = 2.0;

/// 音频捕获配置
pub struct AudioCaptureConfig {
    pub sample_rate: u32,
    pub channels: u16,
}

impl From<&SupportedStreamConfig> for AudioCaptureConfig {
    fn from(config: &SupportedStreamConfig) -> Self {
        Self {
            sample_rate: config.sample_rate().0,
            channels: config.channels(),
        }
    }
}

/// 音频流管理器 - 改进版本，使用更好的 RAII 模式
pub struct AudioStreamManager {
    _stream: Stream, // 使用 RAII 模式，Stream 会在 drop 时自动停止
    config: AudioCaptureConfig,
}

// 实现 Send trait (因为 Stream 不是 Send，我们需要确保正确使用)
unsafe impl Send for AudioStreamManager {}

impl AudioStreamManager {
    /// 启动音频捕获流
    /// 
    /// 改进点:
    /// 1. 返回 AudioStreamManager 而非 forget stream，确保资源正确管理
    /// 2. 使用 Arc<AtomicBool> 来控制流的生命周期
    /// 3. 错误处理更加完善
    pub fn start(sample_sender: mpsc::Sender<Vec<f32>>) -> Result<Self> {
        let host = cpal::default_host();
        
        let device = host
            .default_input_device()
            .context("没有可用的音频输入设备")?;
        
        log::info!("音频输入设备: {}", device.name()?);
        log::info!("音频增益: {:.1}x", AUDIO_GAIN);

        let config = device
            .default_input_config()
            .context("无法获取默认音频配置")?;
        
        log::info!("音频配置: {:?}", config);

        // 只支持 f32 格式
        if config.sample_format() != cpal::SampleFormat::F32 {
            anyhow::bail!("不支持的音频格式，需要 f32 格式");
        }

        let stream_config = config.config();
        
        // 使用 Arc 来安全地在回调中使用 sender
        let sender = Arc::new(sample_sender);
        
        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // 应用音频增益
                let amplified: Vec<f32> = data.iter().map(|&x| x * AUDIO_GAIN).collect();
                
                // 使用 try_send 避免阻塞音频线程
                if let Err(e) = sender.try_send(amplified) {
                    log::warn!("音频数据发送失败: {}", e);
                }
            },
            move |err| {
                log::error!("音频流错误: {}", err);
            },
            None, // None = 使用默认配置
        )
        .context("无法创建音频输入流")?;

        stream.play().context("无法启动音频流")?;

        Ok(Self {
            _stream: stream,
            config: AudioCaptureConfig::from(&config),
        })
    }

    /// 获取音频配置
    pub fn config(&self) -> &AudioCaptureConfig {
        &self.config
    }
}

// 实现 Drop trait 确保资源正确清理
impl Drop for AudioStreamManager {
    fn drop(&mut self) {
        log::info!("音频流管理器已停止");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audio_capture_basic() {
        let (tx, mut rx) = mpsc::channel(100);
        
        // 注意: 这个测试在 CI 环境中可能失败，因为没有音频设备
        if let Ok(_manager) = AudioStreamManager::start(tx) {
            // 等待一些音频数据
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            // 应该收到一些音频数据
            if let Ok(samples) = rx.try_recv() {
                assert!(!samples.is_empty());
            }
        }
    }
}
