use crate::model_bus::provider::{ChatMessage, ContentPart, ModelProvider};
use futures_util::StreamExt;
use crate::model_bus::scheduler::ModelScheduler;
use crate::storage::models::Message;
use anyhow::{Context, Result};
use uuid::Uuid;

/// 从用户消息中抽取关键事件（使用 LLM）
pub async fn extract_event(
    scheduler: &ModelScheduler,
    session_id: &str,
    user_msg: &str,
    assistant_reply: &str,
) -> Result<Option<serde_json::Value>> {
    let provider = scheduler.active().await.context("No active LLM")?;
    let prompt = format!(
        "从以下对话中提取关键事件（如考试、生日、计划、重要事实）。\
         如果没有重要事件返回空 JSON 数组。JSON 格式：[{{\"type\": \"event\", \
         \"summary\": \"摘要\", \"importance\": 0.5, \"date\": null}}]\n\n\
         用户：{user_msg}\nAI：{assistant_reply}"
    );

    let req = crate::model_bus::provider::ChatRequest {
        messages: vec![ChatMessage::system(&prompt)],
        temperature: 0.3,
        max_tokens: 256,
        ..Default::default()
    };

    let mut stream = provider.chat_stream(req);
    let mut response = String::new();
    while let Some(chunk) = stream.next().await {
        response.push_str(&chunk.content);
    }

    if response.trim().is_empty() || response.trim() == "[]" {
        return Ok(None);
    }

    Ok(serde_json::from_str(&response).ok())
}
