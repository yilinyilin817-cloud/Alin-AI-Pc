use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub persona_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub is_pinned: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub content_parts: Option<String>,
    pub segments: Option<Vec<MessageSegment>>,
    pub emotion_tag: Option<EmotionTag>,
    pub tool_calls: Option<String>,
    pub created_at: String,
}

/// 消息分段（用于多段式回复）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageSegment {
    #[serde(rename = "type")]
    pub segment_type: SegmentType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_bytes: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<MessageSegmentSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
}

/// 分段消息来源
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageSegmentSource {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SegmentType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "code")]
    Code,
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "think")]
    Think,
    #[serde(rename = "tool_result")]
    ToolResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmotionTag {
    pub emotion: String,
    pub valence: f64,
    pub arousal: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub id: String,
    pub persona_id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub trigger: serde_json::Value,
    pub actions: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkflowTrigger {
    #[serde(rename = "message")]
    Message { pattern: Option<String> },
    #[serde(rename = "scheduled")]
    Scheduled { cron: String },
    #[serde(rename = "event")]
    Event { event_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowAction {
    pub id: String,
    #[serde(rename = "type")]
    pub action_type: String,
    pub config: serde_json::Value,
    pub next_action_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonaDefinition {
    pub id: String,
    pub name: String,
    pub version: String,
    pub appearance: serde_json::Value,
    pub voice: serde_json::Value,
    pub llm: serde_json::Value,
    pub system_prompt: String,
    pub personality: Vec<String>,
    pub greeting: String,
    pub memory_policy: serde_json::Value,
    pub skills: Vec<String>,
    pub knowledge_bases: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflows: Option<Vec<Workflow>>,
    pub emotion_profile: serde_json::Value,
    pub multimodal: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wechat: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRow {
    pub id: String,
    pub persona_id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub trigger_json: String,
    pub actions_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonaRow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub definition: PersonaDefinition,
    pub is_active: bool,
}

// ─── 微信 iLink ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeChatAccount {
    pub id: String,
    pub user_id: Option<String>,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    /// 是否已登录（是否有 bot_token）
    pub has_bot_token: bool,
    /// 当前轮询游标
    pub get_updates_buf: Option<String>,
    /// 绑定的角色 ID（NULL = 不自动回复）
    pub persona_id: Option<String>,
    pub status: String,
    pub last_error: Option<String>,
    pub last_login_at: Option<String>,
    pub last_sync_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeChatSession {
    pub id: String,
    pub account_id: String,
    pub peer_id: String,
    pub peer_type: String,
    pub peer_name: Option<String>,
    pub peer_avatar: Option<String>,
    pub last_msg_preview: Option<String>,
    pub last_msg_at: Option<String>,
    pub unread_count: i64,
    pub is_pinned: bool,
    pub is_muted: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeChatMessage {
    pub id: String,
    pub account_id: String,
    pub session_id: String,
    pub remote_msg_id: Option<String>,
    pub direction: String,
    pub msg_type: String,
    pub content: Option<String>,
    pub media_url: Option<String>,
    pub media_local_path: Option<String>,
    pub sender_id: Option<String>,
    pub sender_name: Option<String>,
    pub context_token: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeChatQrCode {
    pub account_id: String,
    pub qrcode_url: String,
    pub qrcode_key: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeChatLoginStatus {
    pub status: String, // idle/pending/scanned/confirmed/expired/error
    pub account_id: String,
    pub bot_token: Option<String>,
    pub user_id: Option<String>,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub message: Option<String>,
}
