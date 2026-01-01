use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Local};

/// 消息角色
#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// 聊天消息
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Local>,
}

impl ChatMessage {
    pub fn new(role: MessageRole, content: String) -> Self {
        Self {
            role,
            content,
            timestamp: Local::now(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content.into())
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content.into())
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, content.into())
    }
}

/// Agent 抽象 trait
#[async_trait]
pub trait Agent: Send + Sync {
    /// 发送消息并获取响应
    async fn chat(&self, messages: &[ChatMessage]) -> Result<String>;

    /// 获取 Agent 名称
    fn name(&self) -> &str;

    /// 获取 Agent 描述
    fn description(&self) -> &str;
}

/// Mock Agent 实现，用于模拟 AI 调用
pub struct MockAgent {
    name: String,
    description: String,
}

impl MockAgent {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

#[async_trait]
impl Agent for MockAgent {
    async fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        // 模拟 AI 响应
        let last_message = messages.last().map(|m| m.content.as_str()).unwrap_or("");

        // 简单的模拟响应逻辑
        let response = if last_message.contains("你好") || last_message.contains("hello") {
            "你好！我是你的 AI 助手。很高兴为你服务！\n\n有什么我可以帮助你的吗？".to_string()
        } else if last_message.contains("markdown") || last_message.contains("表格") {
            "# Markdown 渲染演示\n\n这是一个 **Markdown** 渲染演示，包含各种格式。\n\n## 功能列表\n\n1. 支持多级标题\n2. 支持 **粗体** 和 *斜体*\n3. 支持有序列表\n4. 支持无序列表\n5. 支持 `代码`\n6. 支持引用\n\n## 数据表格\n\n| 名称 | 类型 | 描述 |\n|------|------|------|\n| String | 字符串 | 文本数据 |\n| Number | 数字 | 数值数据 |\n| Boolean | 布尔 | 真假值 |\n\n## 代码示例\n\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n    let x = 42;\n    println!(\"The answer is: {}\", x);\n}\n```\n\n```python\ndef hello():\n    print(\"Hello, Python!\")\n    return True\n```\n\n## 引用示例\n\n> 这是一段引用文本。\n> 可以有多行。\n\n希望这个演示对你有帮助！".to_string()
        } else if last_message.contains("帮助") || last_message.contains("help") {
            "# 帮助信息\n\n我可以演示以下 Markdown 功能：\n\n- 输入 \"markdown\" 或 \"表格\" 查看 Markdown 渲染\n- 输入 \"你好\" 查看简单问候\n\n## 快捷键\n\n- **Enter**: 发送消息\n- **Shift+Enter**: 换行\n- **l**: 打开/关闭日志面板\n- **Esc**: 关闭日志面板\n- **q**: 退出程序".to_string()
        } else {
            format!(
                "# 响应\n\n我收到了你的消息：\n\n> {}\n\n这是一个模拟的 AI 响应。在实际使用中，这里会调用真实的 AI API。\n\n## 提示\n\n你可以尝试输入以下内容：\n\n1. \"你好\" - 查看问候\n2. \"markdown\" - 查看 Markdown 渲染效果\n3. \"帮助\" - 查看帮助信息",
                last_message
            )
        };

        Ok(response)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Agent 工厂
pub struct AgentFactory;

impl AgentFactory {
    /// 创建 Mock Agent
    pub fn create_mock_agent(name: impl Into<String>, description: impl Into<String>) -> Box<dyn Agent> {
        Box::new(MockAgent::new(name, description))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_agent() {
        let agent = AgentFactory::create_mock_agent("TestBot", "A test agent");
        
        let messages = vec![
            ChatMessage::system("你是一个有用的助手"),
            ChatMessage::user("你好"),
        ];

        let response = agent.chat(&messages).await.unwrap();
        assert!(response.contains("你好"));
    }
}