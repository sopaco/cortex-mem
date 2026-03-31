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

/// 解析 Whisper 模型路径
///
/// 按以下优先级查找模型文件：
/// 1. 可执行文件所在目录
/// 2. 当前工作目录
///
/// 如果传入的是绝对路径，直接返回原路径。
/// 如果传入的是相对路径，按优先级查找存在的文件；如果都不存在，返回可执行文件目录下的路径。
pub fn resolve_model_path(model_path: &str) -> String {
    // 如果是绝对路径，直接返回
    if std::path::Path::new(model_path).is_absolute() {
        return model_path.to_string();
    }

    // 获取可执行文件所在目录
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()));

    // 获取当前工作目录
    let cwd = std::env::current_dir().ok();

    // 优先级 1: 可执行文件所在目录
    if let Some(ref exe_dir) = exe_dir {
        let exe_path = exe_dir.join(model_path);
        if exe_path.exists() {
            log::info!("从可执行文件目录加载 Whisper 模型: {:?}", exe_path);
            return exe_path.to_string_lossy().to_string();
        }
    }

    // 优先级 2: 当前工作目录
    if let Some(ref cwd) = cwd {
        let cwd_path = cwd.join(model_path);
        if cwd_path.exists() {
            log::info!("从当前工作目录加载 Whisper 模型: {:?}", cwd_path);
            return cwd_path.to_string_lossy().to_string();
        }
    }

    // 如果都不存在，返回可执行文件目录下的路径（让后续加载时报错）
    if let Some(ref exe_dir) = exe_dir {
        let fallback_path = exe_dir.join(model_path);
        log::warn!(
            "Whisper 模型文件未找到，将尝试从可执行文件目录加载: {:?}",
            fallback_path
        );
        return fallback_path.to_string_lossy().to_string();
    }

    // 最后回退到原路径
    model_path.to_string()
}

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
            model_path: "whisper-ggml.bin".to_string(),
            num_threads: 4,
            auto_detect_language: false, // 改为 false，强制使用中文
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
        // 解析模型路径（优先从可执行文件目录查找）
        let resolved_model_path = resolve_model_path(&config.model_path);
        log::info!("加载 Whisper 模型: {} -> {}", config.model_path, resolved_model_path);

        // 🔇 禁用 Whisper 的控制台输出，避免干扰 TUI
        // 临时重定向 stderr 到 /dev/null
        #[cfg(unix)]
        let null_file = std::fs::File::create("/dev/null")?;
        #[cfg(windows)]
        let _null_file = std::fs::File::create("NUL")?;

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
            &resolved_model_path,
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
            .with_context(|| format!("无法加载 Whisper 模型: {}", resolved_model_path))?;

        log::info!("Whisper 模型加载成功");

        // 更新配置中的路径为解析后的路径
        let mut config = config;
        config.model_path = resolved_model_path;

        Ok(Self {
            context: Arc::new(context),
            config,
        })
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

        let resampled_waves = resampler.process(&[audio], None).context("重采样失败")?;

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
        state.full(params, audio_data).context("Whisper 转录失败")?;

        // 收集所有段落（使用 whisper-rs 0.15+ 的迭代器 API）
        let num_segments = state.full_n_segments();
        log::debug!("Whisper 识别出 {} 个段落", num_segments);

        let mut transcribed_text = String::new();
        for segment in state.as_iter() {
            let segment_text = segment.to_str_lossy()
                .map(|s| s.into_owned())
                .unwrap_or_default();
            let segment_text = segment_text.trim();

            if !segment_text.is_empty() {
                log::debug!("段落: '{}'", segment_text);

                // 在段落之间添加空格
                if !transcribed_text.is_empty() {
                    transcribed_text.push(' ');
                }
                transcribed_text.push_str(segment_text);
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
///
/// 改进点:
/// 1. 提高音量阈值到 0.01
/// 2. 增加更多 Whisper 特殊标记
/// 3. 检测重复字符/重复词模式
/// 4. 检测疑似噪音误识别的文本
pub fn is_meaningful_text(text: &str, audio_volume: f32) -> bool {
    let text = text.trim();

    // 1. 检查音频音量
    if audio_volume < 0.02 {
        log::debug!("音频音量过低: {:.4}", audio_volume);
        return false;
    }

    // 2. 检查是否为空
    if text.is_empty() {
        return false;
    }

    // 3. 检查 Whisper 的特殊标记（扩展列表）
    let meaningless_markers = [
        // 标准标记
        "[silence]",
        "[music]",
        "[noise]",
        "[background]",
        "[laughter]",
        "[applause]",
        "[pause]",
        "[cough]",
        "[sneeze]",
        "[breath]",
        "[click]",
        "[thump]",
        "[static]",
        "[echo]",
        "[no audio]",
        "[BLANK_AUDIO]",
        "[typing]",
        "[HUMMING]",
        // 音乐相关
        "(歌詞)",
        "epic music",
        "upbeat music",
        "(epic music)",
        "(upbeat music)",
        "*epic music*",
        "*upbeat music*",
        "music playing",
        "background music",
        // 更多噪音标记
        "[ringing]",
        "[beep]",
        "[ding]",
        "[buzz]",
        "[hiss]",
        "[whir]",
        "[crackle]",
        "[pop]",
        "[bang]",
        "[clap]",
        // 常见误识别
        "...",
        "…",
        "   ",
        "\n",
        "(audience laughter)",
        "(applause)",
        "(cheering)",
        "Subtitle",
        "字幕",
    ];

    for marker in &meaningless_markers {
        if text.to_lowercase().contains(&marker.to_lowercase()) {
            log::debug!("检测到无意义标记: {}", marker);
            return false;
        }
    }

    // 4. 检查文本长度（提高到 4 字符）
    if text.len() < 4 {
        log::debug!("文本过短: {} 字符", text.len());
        return false;
    }

    // 5. 检查是否只包含标点符号
    let has_content = text.chars().any(|c| {
        c.is_alphanumeric() || (c as u32) > 0x4E00 // CJK 字符
    });

    if !has_content {
        log::debug!("文本不包含有意义的内容");
        return false;
    }

    // 6. 检查是否为重复字符模式（如 "啊啊啊啊", "嗯嗯嗯"）
    if is_repetitive_pattern(text) {
        log::debug!("检测到重复字符模式: {}", text);
        return false;
    }

    // 7. 检查是否为疑似噪音误识别（单个音节重复或无意义组合）
    if is_likely_noise_misrecognition(text) {
        log::debug!("检测到疑似噪音误识别: {}", text);
        return false;
    }

    true
}

/// 检查文本是否为重复字符模式
///
/// 例如: "啊啊啊啊", "嗯嗯嗯", "呃呃", "......" 等
fn is_repetitive_pattern(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();

    // 如果只有一个字符类型，认为是重复模式
    if chars.len() >= 3 {
        let unique_chars: std::collections::HashSet<char> = chars.iter().copied().collect();
        if unique_chars.len() == 1 {
            return true;
        }

        // 检查是否只有 2 种字符交替出现（如 "啊呃啊呃"）
        if unique_chars.len() == 2 && chars.len() >= 4 {
            // 检查是否为交替模式
            let chars_vec: Vec<char> = unique_chars.into_iter().collect();
            let mut pattern1 = true;
            let mut pattern2 = true;
            for (i, &c) in chars.iter().enumerate() {
                if c != chars_vec[i % 2] {
                    pattern1 = false;
                }
                if c != chars_vec[(i + 1) % 2] {
                    pattern2 = false;
                }
            }
            if pattern1 || pattern2 {
                return true;
            }
        }
    }

    // 检查是否为重复词（如 "然后然后然后"）
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() >= 3 {
        let unique_words: std::collections::HashSet<&str> = words.iter().copied().collect();
        if unique_words.len() == 1 {
            return true;
        }
    }

    false
}

/// 检查文本是否为疑似噪音误识别
///
/// Whisper 有时会将噪音误识别为一些常见的音节或组合
fn is_likely_noise_misrecognition(text: &str) -> bool {
    // 常见的噪音误识别模式
    let noise_patterns = [
        // 单音节重复
        "嗯",
        "呃",
        "啊",
        "哦",
        "呃",
        "额",
        "唔",
        "嗯",
        "uh",
        "um",
        "ah",
        "oh",
        "er",
        "hm",
        // 无意义组合
        "谢谢收看", // 常见误识别
        "请继续",   // 常见误识别
        "谢谢观看", // 常见误识别
        "下期再见", // 常见误识别
        "感谢收看", // 常见误识别
        "谢谢大家", // 常见误识别
    ];

    let text_lower = text.to_lowercase();
    let text_trimmed = text.trim();

    // 检查是否只包含噪音模式
    for pattern in &noise_patterns {
        // 如果文本完全匹配或主要由这个模式组成
        if text_trimmed == *pattern || text_lower == *pattern {
            return true;
        }
        // 如果文本是模式的重复（如 "嗯嗯嗯"）
        if pattern.len() <= 3 && text_trimmed.chars().all(|c| pattern.contains(c)) {
            // 检查是否只包含这个模式的字符
            let pattern_chars: std::collections::HashSet<char> = pattern.chars().collect();
            let text_chars: std::collections::HashSet<char> = text_trimmed.chars().collect();
            if text_chars.is_subset(&pattern_chars) && text_trimmed.len() >= 3 {
                return true;
            }
        }
    }

    // 检查文本是否太短且包含大量标点
    let alpha_count = text.chars().filter(|c| c.is_alphabetic()).count();
    let punct_count = text.chars().filter(|c| c.is_ascii_punctuation()).count();
    if text.len() < 10 && punct_count > alpha_count {
        return true;
    }

    // 检查是否为纯数字（可能是噪音误识别）
    if text
        .chars()
        .all(|c| c.is_ascii_digit() || c.is_whitespace())
    {
        return true;
    }

    false
}

/// 繁体转简体（简单映射）
/// 注意：这是一个简化版本，只处理常见的繁体字
fn convert_traditional_to_simplified(text: &str) -> String {
    // 常见繁体字到简体字的映射
    let traditional_to_simplified = [
        ("這", "这"),
        ("個", "个"),
        ("們", "们"),
        ("來", "来"),
        ("說", "说"),
        ("時", "时"),
        ("為", "为"),
        ("會", "会"),
        ("對", "对"),
        ("沒", "没"),
        ("過", "过"),
        ("還", "还"),
        ("點", "点"),
        ("開", "开"),
        ("關", "关"),
        ("見", "见"),
        ("聽", "听"),
        ("講", "讲"),
        ("認", "认"),
        ("識", "识"),
        ("間", "间"),
        ("問", "问"),
        ("題", "题"),
        ("應", "应"),
        ("該", "该"),
        ("當", "当"),
        ("現", "现"),
        ("樣", "样"),
        ("處", "处"),
        ("變", "变"),
        ("動", "动"),
        ("從", "从"),
        ("後", "后"),
        ("學", "学"),
        ("機", "机"),
        ("電", "电"),
        ("話", "话"),
        ("國", "国"),
        ("長", "长"),
        ("種", "种"),
        ("發", "发"),
        ("經", "经"),
        ("書", "书"),
        ("記", "记"),
        ("員", "员"),
        ("業", "业"),
        ("產", "产"),
        ("廠", "厂"),
        ("車", "车"),
        ("門", "门"),
        ("網", "网"),
        ("線", "线"),
        ("進", "进"),
        ("運", "运"),
        ("數", "数"),
        ("據", "据"),
        ("區", "区"),
        ("歷", "历"),
        ("報", "报"),
        ("場", "场"),
        ("幾", "几"),
        ("條", "条"),
        ("導", "导"),
        ("術", "术"),
        ("環", "环"),
        ("億", "亿"),
        ("萬", "万"),
        ("華", "华"),
        ("復", "复"),
        ("雙", "双"),
        ("協", "协"),
        ("實", "实"),
        ("體", "体"),
        ("內", "内"),
        ("總", "总"),
        ("達", "达"),
        ("極", "极"),
        ("標", "标"),
        ("確", "确"),
        ("較", "较"),
        ("組", "组"),
        ("統", "统"),
        ("級", "级"),
        ("獨", "独"),
        ("與", "与"),
        ("並", "并"),
        ("層", "层"),
        ("際", "际"),
        ("頭", "头"),
        ("漢", "汉"),
        ("測", "测"),
        ("態", "态"),
        ("費", "费"),
        ("約", "约"),
        ("術", "术"),
        ("備", "备"),
        ("劃", "划"),
        ("參", "参"),
        ("質", "质"),
        ("護", "护"),
        ("導", "导"),
        ("險", "险"),
        ("測", "测"),
        ("廣", "广"),
        ("農", "农"),
        ("響", "响"),
        ("類", "类"),
        ("語", "语"),
        ("兒", "儿"),
        ("師", "师"),
        ("節", "节"),
        ("藝", "艺"),
        ("錶", "表"),
        ("鐘", "钟"),
        ("鬧", "闹"),
        ("麼", "么"),
        ("樂", "乐"),
        ("聲", "声"),
        ("臺", "台"),
        ("灣", "湾"),
        ("礙", "碍"),
        ("愛", "爱"),
        ("罷", "罢"),
        ("筆", "笔"),
        ("邊", "边"),
        ("賓", "宾"),
        ("倉", "仓"),
        ("嘗", "尝"),
        ("塵", "尘"),
        ("遲", "迟"),
        ("蟲", "虫"),
        ("處", "处"),
        ("觸", "触"),
        ("詞", "词"),
        ("達", "达"),
        ("帶", "带"),
        ("單", "单"),
        ("擋", "挡"),
        ("島", "岛"),
        ("燈", "灯"),
        ("調", "调"),
        ("讀", "读"),
        ("獨", "独"),
        ("對", "对"),
        ("奪", "夺"),
        ("頓", "顿"),
        ("額", "额"),
        ("兒", "儿"),
        ("爾", "尔"),
        ("罰", "罚"),
        ("範", "范"),
        ("飛", "飞"),
        ("墳", "坟"),
        ("豐", "丰"),
        ("復", "复"),
        ("負", "负"),
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

    #[test]
    fn test_resolve_model_path_absolute() {
        // 绝对路径应该直接返回
        let path = "/absolute/path/to/model.bin";
        assert_eq!(resolve_model_path(path), path);
    }

    #[test]
    fn test_resolve_model_path_relative() {
        // 相对路径应该被解析（即使文件不存在，也应该返回一个路径）
        let resolved = resolve_model_path("model.bin");
        // 应该包含文件名
        assert!(resolved.ends_with("model.bin"));
    }
}
