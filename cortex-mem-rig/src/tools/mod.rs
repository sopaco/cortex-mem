// Rig Tool Implementations - OpenViking Style

use cortex_mem_tools::{
    MemoryOperations, SearchArgs, FindArgs, LsArgs, ExploreArgs, StoreArgs,
    AbstractResponse, OverviewResponse, ReadResponse, SearchResponse, FindResponse,
    LsResponse, ExploreResponse, StoreResponse, ToolsError,
};
use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

// ==================== Tiered Access Tools ====================

/// Abstract Tool - Get L0 abstract (~100 tokens)
pub struct AbstractTool {
    operations: Arc<MemoryOperations>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AbstractArgs {
    pub uri: String,
}

impl AbstractTool {
    pub fn new(operations: Arc<MemoryOperations>) -> Self {
        Self { operations }
    }
}

impl Tool for AbstractTool {
    const NAME: &'static str = "abstract";

    type Error = ToolsError;
    type Args = AbstractArgs;
    type Output = AbstractResponse;

    fn definition(
        &self,
        _prompt: String,
    ) -> impl std::future::Future<Output = ToolDefinition> + Send + Sync {
        async {
            ToolDefinition {
                name: Self::NAME.to_string(),
                description: "获取内容的 L0 抽象摘要（~100 tokens），用于快速判断相关性".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "uri": {
                            "type": "string",
                            "description": "内容的 URI"
                        }
                    },
                    "required": ["uri"]
                }),
            }
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(self.operations.get_abstract(&args.uri).await?)
    }
}

/// Overview Tool - Get L1 overview (~2000 tokens)
pub struct OverviewTool {
    operations: Arc<MemoryOperations>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OverviewArgs {
    pub uri: String,
}

impl OverviewTool {
    pub fn new(operations: Arc<MemoryOperations>) -> Self {
        Self { operations }
    }
}

impl Tool for OverviewTool {
    const NAME: &'static str = "overview";

    type Error = ToolsError;
    type Args = OverviewArgs;
    type Output = OverviewResponse;

    fn definition(
        &self,
        _prompt: String,
    ) -> impl std::future::Future<Output = ToolDefinition> + Send + Sync {
        async {
            ToolDefinition {
                name: Self::NAME.to_string(),
                description: "获取内容的 L1 概览（~2000 tokens），包含核心信息和使用场景".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "uri": {
                            "type": "string",
                            "description": "内容的 URI"
                        }
                    },
                    "required": ["uri"]
                }),
            }
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(self.operations.get_overview(&args.uri).await?)
    }
}

/// Read Tool - Get L2 complete content
pub struct ReadTool {
    operations: Arc<MemoryOperations>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadArgs {
    pub uri: String,
}

impl ReadTool {
    pub fn new(operations: Arc<MemoryOperations>) -> Self {
        Self { operations }
    }
}

impl Tool for ReadTool {
    const NAME: &'static str = "read";

    type Error = ToolsError;
    type Args = ReadArgs;
    type Output = ReadResponse;

    fn definition(
        &self,
        _prompt: String,
    ) -> impl std::future::Future<Output = ToolDefinition> + Send + Sync {
        async {
            ToolDefinition {
                name: Self::NAME.to_string(),
                description: "获取 L2 完整内容，仅在需要详细信息时使用".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "uri": {
                            "type": "string",
                            "description": "内容的 URI"
                        }
                    },
                    "required": ["uri"]
                }),
            }
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(self.operations.get_read(&args.uri).await?)
    }
}

// ==================== Search Tools ====================

/// Search Tool - Intelligent search with multiple engines
pub struct SearchTool {
    operations: Arc<MemoryOperations>,
}

impl SearchTool {
    pub fn new(operations: Arc<MemoryOperations>) -> Self {
        Self { operations }
    }
}

impl Tool for SearchTool {
    const NAME: &'static str = "search";

    type Error = ToolsError;
    type Args = SearchArgs;
    type Output = SearchResponse;

    fn definition(
        &self,
        _prompt: String,
    ) -> impl std::future::Future<Output = ToolDefinition> + Send + Sync {
        async {
            ToolDefinition {
                name: Self::NAME.to_string(),
                description: "智能向量搜索记忆，支持递归搜索和分层返回".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "搜索查询"
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "是否递归搜索子目录",
                            "default": true
                        },
                        "return_layers": {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "enum": ["L0", "L1", "L2"]
                            },
                            "description": "返回哪些层级（L0=摘要, L1=概览, L2=完整内容）",
                            "default": ["L0"]
                        },
                        "scope": {
                            "type": "string",
                            "description": "搜索范围 URI"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "最大结果数",
                            "default": 10
                        }
                    },
                    "required": ["query"]
                }),
            }
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(self.operations.search(args).await?)
    }
}

/// Find Tool - Quick search returning only L0 abstracts
pub struct FindTool {
    operations: Arc<MemoryOperations>,
}

impl FindTool {
    pub fn new(operations: Arc<MemoryOperations>) -> Self {
        Self { operations }
    }
}

impl Tool for FindTool {
    const NAME: &'static str = "find";

    type Error = ToolsError;
    type Args = FindArgs;
    type Output = FindResponse;

    fn definition(
        &self,
        _prompt: String,
    ) -> impl std::future::Future<Output = ToolDefinition> + Send + Sync {
        async {
            ToolDefinition {
                name: Self::NAME.to_string(),
                description: "快速查找内容，返回 L0 摘要".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "查找关键词"
                        },
                        "scope": {
                            "type": "string",
                            "description": "查找范围 URI"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "最大结果数",
                            "default": 5
                        }
                    },
                    "required": ["query"]
                }),
            }
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(self.operations.find(args).await?)
    }
}

// ==================== Filesystem Tools ====================

/// Ls Tool - List directory contents
pub struct LsTool {
    operations: Arc<MemoryOperations>,
}

impl LsTool {
    pub fn new(operations: Arc<MemoryOperations>) -> Self {
        Self { operations }
    }
}

impl Tool for LsTool {
    const NAME: &'static str = "ls";

    type Error = ToolsError;
    type Args = LsArgs;
    type Output = LsResponse;

    fn definition(
        &self,
        _prompt: String,
    ) -> impl std::future::Future<Output = ToolDefinition> + Send + Sync {
        async {
            ToolDefinition {
                name: Self::NAME.to_string(),
                description: "列出目录内容，浏览文件系统结构".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "uri": {
                            "type": "string",
                            "description": "目录 URI"
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "是否递归列出子目录",
                            "default": false
                        },
                        "include_abstracts": {
                            "type": "boolean",
                            "description": "是否包含文件的 L0 摘要",
                            "default": false
                        }
                    },
                    "required": []
                }),
            }
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(self.operations.ls(args).await?)
    }
}

/// Explore Tool - Intelligently explore memory space
pub struct ExploreTool {
    operations: Arc<MemoryOperations>,
}

impl ExploreTool {
    pub fn new(operations: Arc<MemoryOperations>) -> Self {
        Self { operations }
    }
}

impl Tool for ExploreTool {
    const NAME: &'static str = "explore";

    type Error = ToolsError;
    type Args = ExploreArgs;
    type Output = ExploreResponse;

    fn definition(
        &self,
        _prompt: String,
    ) -> impl std::future::Future<Output = ToolDefinition> + Send + Sync {
        async {
            ToolDefinition {
                name: Self::NAME.to_string(),
                description: "智能探索记忆空间，结合搜索和浏览".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "探索查询"
                        },
                        "start_uri": {
                            "type": "string",
                            "description": "起始 URI",
                            "default": "cortex://session"
                        },
                        "max_depth": {
                            "type": "integer",
                            "description": "最大探索深度",
                            "default": 3
                        },
                        "return_layers": {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "enum": ["L0", "L1", "L2"]
                            },
                            "description": "返回哪些层级",
                            "default": ["L0"]
                        }
                    },
                    "required": ["query"]
                }),
            }
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(self.operations.explore(args).await?)
    }
}

// ==================== Storage Tools ====================

/// Store Tool - Store content with automatic layer generation
pub struct StoreTool {
    operations: Arc<MemoryOperations>,
}

impl StoreTool {
    pub fn new(operations: Arc<MemoryOperations>) -> Self {
        Self { operations }
    }
}

impl Tool for StoreTool {
    const NAME: &'static str = "store";

    type Error = ToolsError;
    type Args = StoreArgs;
    type Output = StoreResponse;

    fn definition(
        &self,
        _prompt: String,
    ) -> impl std::future::Future<Output = ToolDefinition> + Send + Sync {
        async {
            ToolDefinition {
                name: Self::NAME.to_string(),
                description: "存储新内容，自动生成 L0/L1 分层摘要".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "要存储的内容"
                        },
                        "thread_id": {
                            "type": "string",
                            "description": "会话 ID 或 Agent ID（根据 scope 而定）"
                        },
                        "scope": {
                            "type": "string",
                            "description": "存储范围：'session'（会话，默认）、'user'（用户长期记忆）、'agent'（Agent 记忆）",
                            "enum": ["session", "user", "agent"],
                            "default": "session"
                        },
                        "metadata": {
                            "type": "object",
                            "description": "元数据（标签、重要性等）"
                        },
                        "auto_generate_layers": {
                            "type": "boolean",
                            "description": "是否自动生成 L0/L1 摘要",
                            "default": true
                        }
                    },
                    "required": ["content"]
                }),
            }
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(self.operations.store(args).await?)
    }
}
