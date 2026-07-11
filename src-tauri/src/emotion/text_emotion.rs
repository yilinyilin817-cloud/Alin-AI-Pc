use crate::model_bus::provider::{ChatMessage, ModelProvider};
use crate::model_bus::scheduler::ModelScheduler;
use crate::storage::models::EmotionTag;
use anyhow::{Context, Result};
use futures_util::StreamExt;

/// 用 LLM 分析文本情绪
pub async fn from_text(scheduler: &ModelScheduler, text: &str) -> Result<EmotionTag> {
    let provider = scheduler.active().await.context("No active LLM")?;
    let prompt = format!(
        "分析以下文本的情绪。返回 JSON：{{\"emotion\": \"happy|sad|angry|fearful|surprised|disgusted|neutral\", \"valence\": -1~1, \"arousal\": 0~1}}\n\n{text}"
    );

    let req = crate::model_bus::provider::ChatRequest {
        messages: vec![ChatMessage::system(&prompt)],
        temperature: 0.3,
        max_tokens: 128,
        ..Default::default()
    };

    let mut stream = provider.chat_stream(req);
    let mut response = String::new();
    while let Some(chunk) = stream.next().await {
        response.push_str(&chunk.content);
    }

    // 尝试解析 JSON
    if let Some(json_start) = response.find('{') {
        if let Some(json_end) = response[json_start..].find('}') {
            let json_str = &response[json_start..=json_start + json_end];
            if let Ok(tag) = serde_json::from_str::<EmotionTag>(json_str) {
                return Ok(tag);
            }
        }
    }

    Ok(EmotionTag {
        emotion: "neutral".to_string(),
        valence: 0.0,
        arousal: 0.3,
    })
}
