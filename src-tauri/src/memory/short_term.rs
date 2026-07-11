use crate::model_bus::provider::ChatMessage;
use crate::model_bus::scheduler::ModelScheduler;
use anyhow::Result;
use futures_util::StreamExt;

/// 用 LLM 生成会话摘要
pub async fn summarize(
    scheduler: &ModelScheduler,
    messages: &[crate::storage::models::Message],
) -> Result<String> {
    let provider = scheduler.active().await.ok_or_else(|| anyhow::anyhow!("No active LLM"))?;

    let conversation = messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "用一句话概括以下对话的核心内容（中文，不超过 50 字）：\n\n{conversation}"
    );

    let req = crate::model_bus::provider::ChatRequest {
        messages: vec![ChatMessage::system(&prompt)],
        temperature: 0.3,
        max_tokens: 100,
        ..Default::default()
    };

    let mut stream = provider.chat_stream(req);
    let mut summary = String::new();
    while let Some(chunk) = stream.next().await {
        summary.push_str(&chunk.content);
    }

    Ok(summary.trim().to_string())
}

/// 获取最近 N 条消息作为短期工作记忆
pub fn short_term(messages: &[crate::storage::models::Message], n: usize) -> Vec<String> {
    messages
        .iter()
        .rev()
        .take(n)
        .rev()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect()
}
