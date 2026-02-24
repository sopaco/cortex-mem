/// Whisper 语音转录模块
/// 
/// 改进点:
/// 1. 使用 Arc 共享 WhisperContext，避免重复加载模型
/// 2. 更好的错误处理和日志
/// 3. 支持音频重采样和格式转换
use anyhow::{Context, Result};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use std::sync::Arc;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

/// Whisper 要求的采样率
pub const WHISPER_SAMPLE_RATE: u32 = 16000;

/// Whisper 转录器配置
#[derive(Debug, Clone)]
pub struct TranscriptionConfig {
    /// Whisper 模型文件路径
    pub model_path: String,
    /// 使用的线程数
    pub num_threads: usize,
    /// 是否自动检测语言
    pub auto_detect_language: bool,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            model_path: "examples/废弃的录音项目参考/ggml-medium.bin".to_string(),
            num_threads: 4,
            auto_detect_language: false,  // 改为 false，强制使用中文
        }
    }
}

/// Whisper 转录器
/// 
/// 使用 Arc 包装以支持多线程共享
pub struct WhisperTranscriber {
    context: Arc<WhisperContext>,
    config: TranscriptionConfig,
}

impl WhisperTranscriber {
    /// 创建新的转录器
    pub fn new(config: TranscriptionConfig) -> Result<Self> {
        log::info!("加载 Whisper 模型: {}", config.model_path);
        
        // 🔇 禁用 Whisper 的控制台输出，避免干扰 TUI
        // 临时重定向 stderr 到 /dev/null
        #[cfg(unix)]
        let null_file = std::fs::File::create("/dev/null")?;
        #[cfg(windows)]
        let null_file = std::fs::File::create("NUL")?;
        
        #[cfg(unix)]
        let saved_stderr = unsafe {
            let stderr_fd = libc::dup(2);
            if stderr_fd >= 0 {
                libc::dup2(null_file.as_raw_fd(), 2);
                Some(stderr_fd)
            } else {
                None
            }
        };
        
        let context_result = WhisperContext::new_with_params(
            &config.model_path,
            whisper_rs::WhisperContextParameters::default(),
        );
        
        // 恢复 stderr
        #[cfg(unix)]
        if let Some(fd) = saved_stderr {
            unsafe {
                libc::dup2(fd, 2);
                libc::close(fd);
            }
        }
        
        let context = context_result
            .with_context(|| format!("无法加载 Whisper 模型: {}", config.model_path))?;
        
        log::info!("Whisper 模型加载成功");
        
        Ok(Self {
            context: Arc::new(context),
            config,
        })
    }

    /// 获取共享的 context (用于多线程)
    pub fn context(&self) -> Arc<WhisperContext> {
        Arc::clone(&self.context)
    }

    /// 转录音频
    /// 
    /// # 参数
    /// - `audio_data`: 音频采样数据 (f32 格式，单声道)
    /// - `sample_rate`: 音频采样率
    /// 
    /// # 返回
    /// 转录的文本
    pub async fn transcribe(&self, audio_data: &[f32], sample_rate: u32) -> Result<String> {
        // 预处理音频
        let processed_audio = self.preprocess_audio(audio_data, sample_rate)?;
        
        // 在阻塞线程池中执行转录
        let context = Arc::clone(&self.context);
        let num_threads = self.config.num_threads;
        let auto_detect = self.config.auto_detect_language;
        
        let text = tokio::task::spawn_blocking(move || {
            Self::transcribe_blocking(&context, &processed_audio, num_threads, auto_detect)
        })
        .await
        .context("转录任务失败")??;
        
        Ok(text)
    }

    /// 预处理音频: 重采样到 16kHz
    fn preprocess_audio(&self, audio_data: &[f32], sample_rate: u32) -> Result<Vec<f32>> {
        // 检查音频是否为静音
        let rms = (audio_data.iter().map(|&x| x * x).sum::<f32>() / audio_data.len() as f32).sqrt();
        
        log::debug!(
            "音频预处理: {} 采样, {} Hz, RMS: {:.4}",
            audio_data.len(),
            sample_rate,
            rms
        );
        
        if rms < 0.001 {
            log::warn!("音频过于安静 (RMS: {:.4})，可能是静音", rms);
        }
        
        // 如果已经是 16kHz，直接返回
        if sample_rate == WHISPER_SAMPLE_RATE {
            return Ok(audio_data.to_vec());
        }
        
        // 重采样到 16kHz
        log::debug!("重采样: {} Hz -> {} Hz", sample_rate, WHISPER_SAMPLE_RATE);
        Self::resample_audio(audio_data, sample_rate, WHISPER_SAMPLE_RATE)
    }

    /// 音频重采样
    fn resample_audio(audio: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let mut resampler = SincFixedIn::<f32>::new(
            to_rate as f64 / from_rate as f64,
            2.0,
            params,
            audio.len(),
            1, // 单声道
        )
        .context("无法创建重采样器")?;

        let resampled_waves = resampler
            .process(&[audio], None)
            .context("重采样失败")?;

        Ok(resampled_waves[0].clone())
    }

    /// 在阻塞线程中执行转录
    fn transcribe_blocking(
        context: &WhisperContext,
        audio_data: &[f32],
        num_threads: usize,
        auto_detect_language: bool,
    ) -> Result<String> {
        let mut state = context.create_state().context("无法创建 Whisper 状态")?;

        // 配置转录参数 - 优化中文识别
        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: 5,
            patience: 1.0,
        });

        params.set_n_threads(num_threads as i32);
        params.set_translate(false);
        params.set_language(if auto_detect_language {
            None
        } else {
            Some("zh") // 中文
        });
        
        // 🔧 优化中文识别的参数
        params.set_initial_prompt("以下是普通话的句子。"); // 引导模型使用简体中文
        params.set_temperature(0.0); // 降低随机性，提高准确性
        params.set_no_speech_thold(0.6); // 过滤无语音段
        
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_single_segment(false);

        // 执行转录
        state
            .full(params, audio_data)
            .context("Whisper 转录失败")?;

        // 收集所有段落
        let num_segments = state.full_n_segments().context("无法获取段落数量")?;
        log::debug!("Whisper 识别出 {} 个段落", num_segments);

        let mut transcribed_text = String::new();
        for i in 0..num_segments {
            if let Ok(segment) = state.full_get_segment_text(i) {
                let segment_text = segment.trim();
                
                if !segment_text.is_empty() {
                    log::debug!("段落 {}: '{}'", i, segment_text);
                    
                    // 在段落之间添加空格
                    if !transcribed_text.is_empty() {
                        transcribed_text.push(' ');
                    }
                    transcribed_text.push_str(segment_text);
                }
            }
        }

        log::info!("转录完成: {} 字符", transcribed_text.len());
        
        // 🔧 繁体转简体
        let simplified_text = convert_traditional_to_simplified(&transcribed_text);
        
        Ok(simplified_text)
    }
}

/// 将多声道音频转换为单声道
pub fn convert_to_mono(audio: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return audio.to_vec();
    }

    audio
        .chunks_exact(channels)
        .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// 检查转录文本是否有意义
pub fn is_meaningful_text(text: &str, audio_volume: f32) -> bool {
    let text = text.trim();

    // 1. 检查音频音量
    if audio_volume < 0.003 {
        log::debug!("音频音量过低: {:.4}", audio_volume);
        return false;
    }

    // 2. 检查是否为空
    if text.is_empty() {
        return false;
    }

    // 3. 检查 Whisper 的特殊标记
    let meaningless_markers = [
        "[silence]", "[music]", "[noise]", "[background]",
        "[laughter]", "[applause]", "[pause]", "[cough]",
        "[sneeze]", "[breath]", "[click]", "[thump]",
        "[static]", "[echo]", "[no audio]", "[BLANK_AUDIO]",
        "[typing]", "[HUMMING]", "(歌詞)",
        "epic music", "upbeat music", "(epic music)", "(upbeat music)",
        "*epic music*", "*upbeat music*", "music playing", "background music",
    ];

    for marker in &meaningless_markers {
        if text.to_lowercase().contains(&marker.to_lowercase()) {
            log::debug!("检测到无意义标记: {}", marker);
            return false;
        }
    }

    // 4. 检查文本长度
    if text.len() < 3 {
        log::debug!("文本过短: {} 字符", text.len());
        return false;
    }

    // 5. 检查是否只包含标点符号
    let has_content = text.chars().any(|c| {
        c.is_alphanumeric() || c.is_whitespace() || (c as u32) > 0x4E00 // CJK 字符
    });

    if !has_content {
        log::debug!("文本不包含有意义的内容");
        return false;
    }

    true
}

/// 繁体转简体（简单映射）
/// 注意：这是一个简化版本，只处理常见的繁体字
fn convert_traditional_to_simplified(text: &str) -> String {
    // 常见繁体字到简体字的映射
    let traditional_to_simplified = [
        ("這", "这"), ("個", "个"), ("們", "们"), ("來", "来"),
        ("說", "说"), ("時", "时"), ("為", "为"), ("會", "会"),
        ("對", "对"), ("沒", "没"), ("過", "过"), ("還", "还"),
        ("點", "点"), ("開", "开"), ("關", "关"), ("見", "见"),
        ("聽", "听"), ("講", "讲"), ("認", "认"), ("識", "识"),
        ("間", "间"), ("問", "问"), ("題", "题"), ("應", "应"),
        ("該", "该"), ("當", "当"), ("現", "现"), ("樣", "样"),
        ("處", "处"), ("變", "变"), ("動", "动"), ("從", "从"),
        ("後", "后"), ("學", "学"), ("機", "机"), ("電", "电"),
        ("話", "话"), ("國", "国"), ("長", "长"), ("種", "种"),
        ("發", "发"), ("經", "经"), ("書", "书"), ("記", "记"),
        ("員", "员"), ("業", "业"), ("產", "产"), ("廠", "厂"),
        ("車", "车"), ("門", "门"), ("網", "网"), ("線", "线"),
        ("進", "进"), ("運", "运"), ("數", "数"), ("據", "据"),
        ("區", "区"), ("歷", "历"), ("報", "报"), ("場", "场"),
        ("幾", "几"), ("條", "条"), ("導", "导"), ("術", "术"),
        ("環", "环"), ("億", "亿"), ("萬", "万"), ("華", "华"),
        ("復", "复"), ("雙", "双"), ("協", "协"), ("實", "实"),
        ("體", "体"), ("內", "内"), ("總", "总"), ("達", "达"),
        ("極", "极"), ("標", "标"), ("確", "确"), ("較", "较"),
        ("組", "组"), ("統", "统"), ("級", "级"), ("獨", "独"),
        ("與", "与"), ("並", "并"), ("層", "层"), ("際", "际"),
        ("頭", "头"), ("漢", "汉"), ("測", "测"), ("態", "态"),
        ("費", "费"), ("約", "约"), ("術", "术"), ("備", "备"),
        ("劃", "划"), ("參", "参"), ("質", "质"), ("護", "护"),
        ("導", "导"), ("險", "险"), ("測", "测"), ("廣", "广"),
        ("農", "农"), ("響", "响"), ("類", "类"), ("語", "语"),
        ("兒", "儿"), ("師", "师"), ("節", "节"), ("藝", "艺"),
        ("錶", "表"), ("鐘", "钟"), ("鬧", "闹"), ("麼", "么"),
        ("樂", "乐"), ("聲", "声"), ("臺", "台"), ("灣", "湾"),
        ("礙", "碍"), ("愛", "爱"), ("罷", "罢"), ("筆", "笔"),
        ("邊", "边"), ("賓", "宾"), ("倉", "仓"), ("嘗", "尝"),
        ("塵", "尘"), ("遲", "迟"), ("蟲", "虫"), ("處", "处"),
        ("觸", "触"), ("詞", "词"), ("達", "达"), ("帶", "带"),
        ("單", "单"), ("擋", "挡"), ("島", "岛"), ("燈", "灯"),
        ("調", "调"), ("讀", "读"), ("獨", "独"), ("對", "对"),
        ("奪", "夺"), ("頓", "顿"), ("額", "额"), ("兒", "儿"),
        ("爾", "尔"), ("罰", "罚"), ("範", "范"), ("飛", "飞"),
        ("墳", "坟"), ("豐", "丰"), ("復", "复"), ("負", "负"),
    ];
    
    let mut result = text.to_string();
    for (traditional, simplified) in &traditional_to_simplified {
        result = result.replace(traditional, simplified);
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_to_mono() {
        let stereo = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
        let mono = convert_to_mono(&stereo, 2);
        assert_eq!(mono.len(), 3);
        assert!((mono[0] - 0.15).abs() < 0.01);
    }
    
    #[test]
    fn test_traditional_to_simplified() {
        assert_eq!(convert_traditional_to_simplified("這個"), "这个");
        assert_eq!(convert_traditional_to_simplified("時間"), "时间");
        assert_eq!(convert_traditional_to_simplified("開關"), "开关");
    }

    #[test]
    fn test_meaningful_text() {
        assert!(is_meaningful_text("你好世界", 0.1));
        assert!(is_meaningful_text("Hello world", 0.1));
        assert!(!is_meaningful_text("[silence]", 0.1));
        assert!(!is_meaningful_text("", 0.1));
        assert!(!is_meaningful_text("abc", 0.001));
    }
}
