use futures_util::stream::BoxStream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;

/// 模型能力
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Capabilities {
    pub vision: bool,         // 图像输入
    pub audio_input: bool,    // 音频输入
    pub video: bool,          // 视频逐帧
    pub tool_use: bool,       // Function calling
    pub max_context: usize,   // 最大上下文 token
}

/// 多模态内容片段
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text(String),
    #[serde(rename = "image_bytes")]
    ImageBytes(Vec<u8>),
    #[serde(rename = "image_url")]
    ImageUrl(String),
    #[serde(rename = "audio_bytes")]
    AudioBytes {
        data: Vec<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        transcript: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        file_id: Option<String>,
    },
    #[serde(rename = "video_frames")]
    VideoFrames(Vec<String>), // base64 帧路径
}

impl ContentPart {
    pub fn text(text: &str) -> Self {
        ContentPart::Text(text.to_string())
    }
    pub fn image_bytes(data: Vec<u8>) -> Self {
        ContentPart::ImageBytes(data)
    }
}

/// 消息（对话历史）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user" | "assistant" | "system" | "tool"
    pub content: Vec<ContentPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(text: &str) -> Self {
        Self {
            role: "system".into(),
            content: vec![ContentPart::text(text)],
            tool_calls: None,
            tool_call_id: None,
        }
    }
    pub fn user(text: &str) -> Self {
        Self {
            role: "user".into(),
            content: vec![ContentPart::text(text)],
            tool_calls: None,
            tool_call_id: None,
        }
    }
    pub fn assistant(text: &str) -> Self {
        Self {
            role: "assistant".into(),
            content: vec![ContentPart::text(text)],
            tool_calls: None,
            tool_call_id: None,
        }
    }
    pub fn user_multimodal(parts: Vec<ContentPart>) -> Self {
        Self {
            role: "user".into(),
            content: parts,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant_with_tools(text: &str, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".into(),
            content: vec![ContentPart::text(text)],
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    pub fn tool(tool_call_id: &str, content: &str) -> Self {
        Self {
            role: "tool".into(),
            content: vec![ContentPart::text(content)],
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// 工具（Skill）定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value, // JSON Schema
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// 聊天请求
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<ToolSchema>,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: usize,
}

impl Default for ChatRequest {
    fn default() -> Self {
        Self {
            messages: vec![],
            tools: vec![],
            temperature: 0.7,
            top_p: 0.9,
            max_tokens: 2048,
        }
    }
}

/// 聊天响应块（流式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    pub done: bool,
}

/// ModelProvider 统一抽象
pub trait ModelProvider: Debug + Send + Sync {
    fn id(&self) -> &str;
    fn capabilities(&self) -> Capabilities;
    fn chat_stream(&self, req: ChatRequest) -> BoxStream<'static, ChatChunk>;
    fn embed(&self, texts: &[String]) -> BoxStream<'static, Vec<Vec<f32>>>;
}
