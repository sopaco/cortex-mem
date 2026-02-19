use serde::{Deserialize, Serialize};

/// 存储记忆请求
#[derive(Debug, Clone, Deserialize)]
pub struct StoreMemoryRequest {
    /// 语音转录后的文本内容
    pub content: String,
    /// 固定值 "audio_listener"，标识来源为语音旁听服务
    #[allow(dead_code)]
    pub source: String,
    /// 语音识别的时间戳，RFC 3339 格式
    #[allow(dead_code)]
    pub timestamp: String,
    /// 说话人类型："user"（本人）或 "other"（他人）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_type: Option<String>,
    /// 说话人识别的置信度（0-1）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    pub speaker_confidence: Option<f32>,
}

/// 存储记忆响应
#[derive(Debug, Clone, Serialize)]
pub struct StoreMemoryResponse {
    /// 是否成功存储
    pub success: bool,
    /// 存储的记忆唯一标识
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_id: Option<String>,
    /// 成功或错误消息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// 健康检查响应
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    /// API 状态
    pub status: String,
    /// 当前时间戳
    pub timestamp: String,
}

/// 错误响应
#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    /// 是否成功
    pub success: bool,
    /// 错误类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    /// 错误信息
    pub error: String,
}

/// 记忆项（用于查询和列表响应）
#[derive(Debug, Clone, Serialize)]
pub struct MemoryItem {
    /// 记忆 ID
    pub id: String,
    /// 记忆内容
    pub content: String,
    /// 来源
    pub source: String,
    /// 时间戳
    pub timestamp: String,
    /// 说话人类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_type: Option<String>,
    /// 说话人置信度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_confidence: Option<f32>,
    /// 相关性分数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevance: Option<f32>,
}

/// 查询记忆响应
#[derive(Debug, Clone, Serialize)]
pub struct RetrieveMemoryResponse {
    /// 记忆列表
    pub memories: Vec<MemoryItem>,
}

/// 列出记忆响应
#[derive(Debug, Clone, Serialize)]
pub struct ListMemoryResponse {
    /// 记忆列表
    pub memories: Vec<MemoryItem>,
    /// 总数
    pub total: usize,
}