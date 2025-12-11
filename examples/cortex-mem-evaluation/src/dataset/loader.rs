//! 数据集加载器
//! 
//! 加载和验证测试数据集

use anyhow::{Result, Context};
use std::path::Path;
use tracing::info;

use super::types::{RecallTestDataset, EffectivenessTestDataset};

/// 数据集加载器
pub struct DatasetLoader;

impl DatasetLoader {
    /// 加载召回率测试数据集
    pub fn load_recall_dataset(path: &Path) -> Result<RecallTestDataset> {
        info!("加载召回率测试数据集: {:?}", path);
        
        let content = std::fs::read_to_string(path)
            .context(format!("读取数据集文件失败: {:?}", path))?;
        
        let dataset: RecallTestDataset = serde_json::from_str(&content)
            .context("解析召回率数据集失败")?;
        
        info!("召回率数据集加载完成: {}个测试用例, {}个记忆",
            dataset.test_cases.len(), dataset.memories.len());
        
        Ok(dataset)
    }
    
    /// 加载有效性测试数据集
    pub fn load_effectiveness_dataset(path: &Path) -> Result<EffectivenessTestDataset> {
        info!("加载有效性测试数据集: {:?}", path);
        
        let content = std::fs::read_to_string(path)
            .context(format!("读取数据集文件失败: {:?}", path))?;
        
        let dataset: EffectivenessTestDataset = serde_json::from_str(&content)
            .context("解析有效性数据集失败")?;
        
        info!("有效性数据集加载完成: {}个测试用例, {}个现有记忆",
            dataset.test_cases.len(), dataset.existing_memories.len());
        
        Ok(dataset)
    }
    
    /// 验证数据集完整性
    pub fn validate_dataset<T: serde::de::DeserializeOwned>(path: &Path) -> Result<()> {
        info!("验证数据集: {:?}", path);
        
        let content = std::fs::read_to_string(path)
            .context(format!("读取数据集文件失败: {:?}", path))?;
        
        // 尝试解析以验证格式
        let _dataset: T = serde_json::from_str(&content)
            .context("数据集格式验证失败")?;
        
        info!("数据集验证通过: {:?}", path);
        Ok(())
    }
    
    /// 获取数据集统计信息
    pub fn get_dataset_stats(path: &Path) -> Result<DatasetStats> {
        let content = std::fs::read_to_string(path)
            .context(format!("读取数据集文件失败: {:?}", path))?;
        
        // 根据文件内容判断数据集类型
        if content.contains("relevant_memory_ids") {
            let dataset: RecallTestDataset = serde_json::from_str(&content)?;
            Ok(DatasetStats {
                dataset_type: "recall".to_string(),
                test_cases_count: dataset.test_cases.len(),
                memories_count: dataset.memories.len(),
                metadata: Some(dataset.metadata),
            })
        } else if content.contains("expected_facts") {
            let dataset: EffectivenessTestDataset = serde_json::from_str(&content)?;
            Ok(DatasetStats {
                dataset_type: "effectiveness".to_string(),
                test_cases_count: dataset.test_cases.len(),
                memories_count: dataset.existing_memories.len(),
                metadata: Some(dataset.metadata),
            })
        } else {
            anyhow::bail!("未知的数据集类型")
        }
    }
}

/// 数据集统计信息
#[derive(Debug)]
pub struct DatasetStats {
    /// 数据集类型
    pub dataset_type: String,
    /// 测试用例数量
    pub test_cases_count: usize,
    /// 记忆数量
    pub memories_count: usize,
    /// 元数据
    pub metadata: Option<super::types::DatasetMetadata>,
}