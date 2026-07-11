use crate::storage::models::EmotionTag;
use anyhow::Result;

/// 语音情绪识别（调用 emotion_worker）
pub async fn from_voice(_audio_bytes: &[u8]) -> Result<EmotionTag> {
    // 真实实现调用 WorkerPool::call(Emotion, ...)
    // 当前返回中性情绪占位
    Ok(EmotionTag {
        emotion: "neutral".to_string(),
        valence: 0.0,
        arousal: 0.3,
    })
}
