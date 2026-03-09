use crate::layers::generator::{AbstractGenerator, OverviewGenerator};
use crate::llm::LLMClient;
use crate::{CortexFilesystem, FilesystemOperations, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// 层级生成配置
#[derive(Debug, Clone)]
pub struct LayerGenerationConfig {
    /// 每批生成数量
    pub batch_size: usize,
    /// 批次间延迟（毫秒）
    pub delay_ms: u64,
    /// 启动时自动生成
    pub auto_generate_on_startup: bool,
    /// Abstract 配置
    pub abstract_config: AbstractConfig,
    /// Overview 配置
    pub overview_config: OverviewConfig,
}

#[derive(Debug, Clone)]
pub struct AbstractConfig {
    /// 最大 Token 数
    pub max_tokens: usize,
    /// 最大字符数
    pub max_chars: usize,
    /// 目标句子数
    pub target_sentences: usize,
}

#[derive(Debug, Clone)]
pub struct OverviewConfig {
    /// 最大 Token 数
    pub max_tokens: usize,
    /// 最大字符数
    pub max_chars: usize,
}

impl Default for LayerGenerationConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            delay_ms: 2000,
            auto_generate_on_startup: false,
            abstract_config: AbstractConfig {
                max_tokens: 400,
                max_chars: 2000,
                target_sentences: 2,
            },
            overview_config: OverviewConfig {
                max_tokens: 1500,
                max_chars: 6000,
            },
        }
    }
}

/// 层级生成统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerationStats {
    pub total: usize,
    pub generated: usize,
    pub failed: usize,
}

/// 层级生成器
///
/// 负责扫描文件系统，检测缺失的 L0/L1 文件，并渐进式生成
pub struct LayerGenerator {
    filesystem: Arc<CortexFilesystem>,
    abstract_gen: AbstractGenerator,
    overview_gen: OverviewGenerator,
    llm_client: Arc<dyn LLMClient>,
    config: LayerGenerationConfig,
}

impl LayerGenerator {
    pub fn new(
        filesystem: Arc<CortexFilesystem>,
        llm_client: Arc<dyn LLMClient>,
        config: LayerGenerationConfig,
    ) -> Self {
        Self {
            filesystem,
            abstract_gen: AbstractGenerator::new(),
            overview_gen: OverviewGenerator::new(),
            llm_client,
            config,
        }
    }

    /// 扫描所有目录
    pub async fn scan_all_directories(&self) -> Result<Vec<String>> {
        let mut directories = Vec::new();

        // 扫描四个核心维度
        for scope in &["session", "user", "agent", "resources"] {
            let scope_uri = format!("cortex://{}", scope);

            // Check if scope exists
            match self.filesystem.exists(&scope_uri).await {
                Ok(true) => {
                    debug!("Scanning scope: {}", scope);
                    match self.scan_scope(&scope_uri).await {
                        Ok(dirs) => {
                            debug!("Scope {} found {} directories", scope, dirs.len());
                            directories.extend(dirs);
                        }
                        Err(e) => {
                            warn!("Failed to scan scope {}: {}", scope, e);
                        }
                    }
                }
                Ok(false) => {
                    debug!("Scope {} does not exist, skipping", scope);
                }
                Err(e) => {
                    warn!("Failed to check scope {} existence: {}", scope, e);
                }
            }
        }

        Ok(directories)
    }

    /// Scan a single scope
    async fn scan_scope(&self, scope_uri: &str) -> Result<Vec<String>> {
        let mut directories = Vec::new();
        
        // First check if scope exists
        match self.filesystem.exists(scope_uri).await {
            Ok(true) => {
                debug!("Scope directory exists: {}", scope_uri);
            }
            Ok(false) => {
                debug!("Scope directory does not exist: {}", scope_uri);
                return Ok(directories);
            }
            Err(e) => {
                warn!("Failed to check scope existence: {} - {}", scope_uri, e);
                return Ok(directories);
            }
        }
        
        // Try to list directory contents
        match self.filesystem.list(scope_uri).await {
            Ok(entries) => {
                debug!("Scope {} has {} entries", scope_uri, entries.len());
            }
            Err(e) => {
                warn!("Failed to list scope directory: {} - {}", scope_uri, e);
                return Ok(directories);
            }
        }
        
        self.scan_recursive(scope_uri, &mut directories).await?;
        Ok(directories)
    }

    /// 递归扫描目录
    fn scan_recursive<'a>(
        &'a self,
        uri: &'a str,
        directories: &'a mut Vec<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            // 列出当前目录
            let entries = match self.filesystem.list(uri).await {
                Ok(entries) => entries,
                Err(e) => {
                    debug!("Failed to list {}: {}", uri, e);
                    return Ok(());
                }
            };

            for entry in entries {
                // 跳过隐藏文件
                if entry.name.starts_with('.') {
                    continue;
                }

                if entry.is_directory {
                    // 添加目录到列表
                    directories.push(entry.uri.clone());

                    // 递归扫描子目录
                    self.scan_recursive(&entry.uri, directories).await?;
                }
            }

            Ok(())
        })
    }

    /// 检测目录是否有 L0/L1 文件
    pub async fn has_layers(&self, uri: &str) -> Result<bool> {
        let abstract_path = format!("{}/.abstract.md", uri);
        let overview_path = format!("{}/.overview.md", uri);

        let has_abstract = self.filesystem.exists(&abstract_path).await?;
        let has_overview = self.filesystem.exists(&overview_path).await?;

        Ok(has_abstract && has_overview)
    }

    /// 过滤出缺失 L0/L1 的目录
    pub async fn filter_missing_layers(&self, dirs: &[String]) -> Result<Vec<String>> {
        let mut missing = Vec::new();

        for dir in dirs {
            match self.has_layers(dir).await {
                Ok(has) => {
                    if !has {
                        missing.push(dir.clone());
                    }
                }
                Err(e) => {
                    debug!("Failed to check layers for {}: {}", dir, e);
                }
            }
        }

        Ok(missing)
    }

    /// Ensure all directories have L0/L1
    pub async fn ensure_all_layers(&self) -> Result<GenerationStats> {
        info!("Scanning directories for missing L0/L1 layers...");
        let directories = self.scan_all_directories().await?;
        debug!("Found {} directories", directories.len());
        
        for dir in &directories {
            debug!("Scanned directory: {}", dir);
        }

        let missing = self.filter_missing_layers(&directories).await?;
        info!("Found {} directories missing L0/L1", missing.len());

        if missing.is_empty() {
            return Ok(GenerationStats {
                total: 0,
                generated: 0,
                failed: 0,
            });
        }

        let mut stats = GenerationStats {
            total: missing.len(),
            generated: 0,
            failed: 0,
        };

        // Generate in batches
        let total_batches = (missing.len() + self.config.batch_size - 1) / self.config.batch_size;

        for (batch_idx, batch) in missing.chunks(self.config.batch_size).enumerate() {
            debug!("Processing batch {}/{}", batch_idx + 1, total_batches);

            for dir in batch {
                match self.generate_layers_for_directory(dir).await {
                    Ok(_) => {
                        stats.generated += 1;
                        debug!("Generated: {}", dir);
                    }
                    Err(e) => {
                        stats.failed += 1;
                        warn!("Failed to generate for {}: {}", dir, e);
                    }
                }
            }

            // Delay between batches
            if batch_idx < total_batches - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(self.config.delay_ms)).await;
            }
        }

        info!("Layer generation completed: {} generated, {} failed", stats.generated, stats.failed);
        Ok(stats)
    }

    /// Ensure a specific timeline directory has L0/L1 layer files
    /// Used when session closes to trigger generation, avoiding frequent updates
    pub async fn ensure_timeline_layers(&self, timeline_uri: &str) -> Result<GenerationStats> {
        info!("Starting layer generation for timeline: {}", timeline_uri);

        // Scan all directories under timeline
        let mut directories = Vec::new();
        self.scan_recursive(timeline_uri, &mut directories).await?;

        info!("Found {} timeline directories", directories.len());

        // Detect missing L0/L1
        let missing = self.filter_missing_layers(&directories).await?;
        info!("Found {} directories missing L0/L1", missing.len());

        if missing.is_empty() {
            return Ok(GenerationStats {
                total: 0,
                generated: 0,
                failed: 0,
            });
        }

        let mut stats = GenerationStats {
            total: missing.len(),
            generated: 0,
            failed: 0,
        };

        // Generate layer files (no need to batch, timeline is usually small)
        for dir in missing {
            match self.generate_layers_for_directory(&dir).await {
                Ok(_) => {
                    stats.generated += 1;
                    info!("Generation succeeded: {}", dir);
                }
                Err(e) => {
                    stats.failed += 1;
                    warn!("Generation failed: {} - {}", dir, e);
                }
            }
        }

        info!(
            "Timeline layer generation completed: {} succeeded, {} failed",
            stats.generated, stats.failed
        );
        Ok(stats)
    }

    /// Generate L0/L1 for a single directory
    async fn generate_layers_for_directory(&self, uri: &str) -> Result<()> {
        debug!("Generating layer files for: {}", uri);

        // 1. Check if regeneration is needed (avoid generating unchanged content)
        if !self.should_regenerate(uri).await? {
            debug!("Directory content unchanged, skipping generation: {}", uri);
            return Ok(());
        }

        // 2. Read directory content (aggregate all sub-files)
        let content = self.aggregate_directory_content(uri).await?;

        if content.is_empty() {
            debug!("Directory is empty, skipping: {}", uri);
            return Ok(());
        }

        // 3. Use existing AbstractGenerator to generate L0 abstract
        let abstract_text = self
            .abstract_gen
            .generate_with_llm(&content, &self.llm_client, &[])
            .await?;

        // 4. Use existing OverviewGenerator to generate L1 overview
        let overview = self
            .overview_gen
            .generate_with_llm(&content, &self.llm_client)
            .await?;

        // 5. Enforce length limits
        let abstract_text = self.enforce_abstract_limit(abstract_text)?;
        let overview = self.enforce_overview_limit(overview)?;

        // 6. Add "Added" date marker (consistent with extraction.rs)
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let abstract_with_date = format!("{}\n\n**Added**: {}", abstract_text, timestamp);
        let overview_with_date = format!("{}\n\n---\n\n**Added**: {}", overview, timestamp);

        // 7. Write files
        let abstract_path = format!("{}/.abstract.md", uri);
        let overview_path = format!("{}/.overview.md", uri);

        self.filesystem
            .write(&abstract_path, &abstract_with_date)
            .await?;
        self.filesystem
            .write(&overview_path, &overview_with_date)
            .await?;

        debug!("Layer files generated for: {}", uri);
        Ok(())
    }

    /// Check if layer files need to be regenerated
    ///
    /// Check logic:
    /// 1. If .abstract.md or .overview.md doesn't exist → need to generate
    /// 2. If files in directory are newer than .abstract.md → need to regenerate
    /// 3. Otherwise → skip (avoid duplicate generation)
    async fn should_regenerate(&self, uri: &str) -> Result<bool> {
        let abstract_path = format!("{}/.abstract.md", uri);
        let overview_path = format!("{}/.overview.md", uri);

        // Check if layer files exist
        let abstract_exists = self.filesystem.exists(&abstract_path).await?;
        let overview_exists = self.filesystem.exists(&overview_path).await?;

        if !abstract_exists || !overview_exists {
            debug!("Layer files missing, need to generate: {}", uri);
            return Ok(true);
        }

        // Read timestamp from .abstract.md
        let abstract_content = match self.filesystem.read(&abstract_path).await {
            Ok(content) => content,
            Err(_) => {
                debug!("Cannot read .abstract.md, need to regenerate: {}", uri);
                return Ok(true);
            }
        };

        // Extract "Added" timestamp
        let abstract_timestamp = self.extract_added_timestamp(&abstract_content);

        if abstract_timestamp.is_none() {
            debug!(".abstract.md missing timestamp, need to regenerate: {}", uri);
            return Ok(true);
        }

        let abstract_time = abstract_timestamp.unwrap();

        // Check if files in directory have updates
        let entries = self.filesystem.list(uri).await?;
        for entry in entries {
            // Skip hidden files and directories
            if entry.name.starts_with('.') || entry.is_directory {
                continue;
            }

            // Only check .md and .txt files
            if entry.name.ends_with(".md") || entry.name.ends_with(".txt") {
                // Read file content, extract timestamp if any
                if let Ok(file_content) = self.filesystem.read(&entry.uri).await {
                    if let Some(file_time) = self.extract_added_timestamp(&file_content) {
                        // If file timestamp is later than abstract timestamp, need to regenerate
                        if file_time > abstract_time {
                            debug!("File {} has updates, need to regenerate: {}", entry.name, uri);
                            return Ok(true);
                        }
                    }
                }
            }
        }

        debug!("Directory content unchanged, no need to regenerate: {}", uri);
        Ok(false)
    }

    /// Extract "Added" timestamp from content
    fn extract_added_timestamp(&self, content: &str) -> Option<DateTime<Utc>> {
        // Find "**Added**: YYYY-MM-DD HH:MM:SS UTC" format
        if let Some(start) = content.find("**Added**: ") {
            let timestamp_str = &content[start + 11..];
            if let Some(end) = timestamp_str.find('\n') {
                let timestamp_str = &timestamp_str[..end].trim();
                // Parse timestamp
                if let Ok(dt) = DateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S UTC") {
                    return Some(dt.with_timezone(&Utc));
                }
            }
        }
        None
    }

    /// 聚合目录内容
    async fn aggregate_directory_content(&self, uri: &str) -> Result<String> {
        let entries = self.filesystem.list(uri).await?;
        let mut content = String::new();

        for entry in entries {
            // 跳过隐藏文件和目录
            if entry.name.starts_with('.') || entry.is_directory {
                continue;
            }

            // 只读取文本文件
            if entry.name.ends_with(".md") || entry.name.ends_with(".txt") {
                match self.filesystem.read(&entry.uri).await {
                    Ok(file_content) => {
                        content.push_str(&format!("\n\n=== {} ===\n\n", entry.name));
                        content.push_str(&file_content);
                    }
                    Err(e) => {
                        debug!("Failed to read {}: {}", entry.uri, e);
                    }
                }
            }
        }

        // 截断到合理长度（避免超出 LLM 上下文限制）
        let max_chars = 10000;
        if content.chars().count() > max_chars {
            let truncated: String = content.chars().take(max_chars).collect();
            let mut content = truncated;
            content.push_str("\n\n[内容已截断...]");
            return Ok(content);
        }

        Ok(content)
    }

    /// 强制执行 Abstract 长度限制
    fn enforce_abstract_limit(&self, text: String) -> Result<String> {
        let mut result = text.trim().to_string();
        let max_chars = self.config.abstract_config.max_chars;

        if result.chars().count() <= max_chars {
            return Ok(result);
        }

        // 找到 max_chars 字符对应的字节位置
        let byte_limit = result
            .char_indices()
            .nth(max_chars)
            .map(|(i, _)| i)
            .unwrap_or(result.len());

        // 截断到最后一个句号/问号/叹号
        if let Some(pos) = result[..byte_limit]
            .rfind(|c| c == '。' || c == '.' || c == '?' || c == '!' || c == '！' || c == '？')
        {
            result.truncate(pos + 1);
        } else {
            // 找到 max_chars - 3 字符对应的字节位置
            let truncate_pos = result
                .char_indices()
                .nth(max_chars.saturating_sub(3))
                .map(|(i, _)| i)
                .unwrap_or(result.len());
            result.truncate(truncate_pos);
            result.push_str("...");
        }

        Ok(result)
    }

    /// 强制执行 Overview 长度限制
    fn enforce_overview_limit(&self, text: String) -> Result<String> {
        let mut result = text.trim().to_string();
        let max_chars = self.config.overview_config.max_chars;

        if result.chars().count() <= max_chars {
            return Ok(result);
        }

        // 找到 max_chars 字符对应的字节位置
        let byte_limit = result
            .char_indices()
            .nth(max_chars)
            .map(|(i, _)| i)
            .unwrap_or(result.len());

        // 截断到最后一个段落
        if let Some(pos) = result[..byte_limit].rfind("\n\n") {
            result.truncate(pos);
            result.push_str("\n\n[内容已截断...]");
        } else {
            // 找到 max_chars - 3 字符对应的字节位置
            let truncate_pos = result
                .char_indices()
                .nth(max_chars.saturating_sub(3))
                .map(|(i, _)| i)
                .unwrap_or(result.len());
            result.truncate(truncate_pos);
            result.push_str("...");
        }

        Ok(result)
    }

    /// Regenerate all oversized .abstract files
    pub async fn regenerate_oversized_abstracts(&self) -> Result<RegenerationStats> {
        info!("Scanning for oversized .abstract files...");
        let directories = self.scan_all_directories().await?;
        let max_chars = self.config.abstract_config.max_chars;

        let mut stats = RegenerationStats {
            total: 0,
            regenerated: 0,
            failed: 0,
        };

        for dir in directories {
            let abstract_path = format!("{}/.abstract.md", dir);

            if let Ok(content) = self.filesystem.read(&abstract_path).await {
                // Remove "Added" marker before checking length
                let content_without_metadata = self.strip_metadata(&content);

                if content_without_metadata.len() > max_chars {
                    stats.total += 1;
                    info!(
                        "Found oversized .abstract: {} ({} chars)",
                        dir,
                        content_without_metadata.len()
                    );

                    match self.generate_layers_for_directory(&dir).await {
                        Ok(_) => {
                            stats.regenerated += 1;
                            info!("Regeneration succeeded: {}", dir);
                        }
                        Err(e) => {
                            stats.failed += 1;
                            warn!("Regeneration failed: {} - {}", dir, e);
                        }
                    }
                }
            }
        }

        info!(
            "Regeneration completed: total={}, succeeded={}, failed={}",
            stats.total, stats.regenerated, stats.failed
        );

        Ok(stats)
    }

    /// 移除元数据（Added、Confidence等）
    fn strip_metadata(&self, content: &str) -> String {
        let mut result = content.to_string();

        // 移除 **Added**: ... 行
        if let Some(pos) = result.find("\n\n**Added**:") {
            result.truncate(pos);
        } else if let Some(pos) = result.find("**Added**:") {
            result.truncate(pos);
        }

        result.trim().to_string()
    }
}

/// 重新生成统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegenerationStats {
    pub total: usize,
    pub regenerated: usize,
    pub failed: usize,
}
