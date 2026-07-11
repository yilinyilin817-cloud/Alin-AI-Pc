use crate::storage::models::EmotionTag;

/// 加权融合文本和语音情绪
pub fn fuse(text: EmotionTag, voice: EmotionTag, voice_weight: f64) -> EmotionTag {
    let weight_voice = voice_weight.min(1.0).max(0.0);
    let weight_text = 1.0 - weight_voice;

    let valence = text.valence * weight_text + voice.valence * weight_voice;
    let arousal = text.arousal * weight_text + voice.arousal * weight_voice;

    // 加权多数决选情绪标签
    let emotion = if weight_voice > 0.6 {
        voice.emotion
    } else if weight_text > 0.6 {
        text.emotion
    } else {
        // 如果两者权重接近，选 arousal 更高的（更强烈的情绪）
        if text.arousal >= voice.arousal { text.emotion } else { voice.emotion }
    };

    EmotionTag {
        emotion,
        valence: (valence * 100.0).round() / 100.0,
        arousal: (arousal * 100.0).round() / 100.0,
    }
}
