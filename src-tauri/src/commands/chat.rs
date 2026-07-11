use crate::commands::voice;
use crate::model_bus::provider::ContentPart;
use crate::state::AppState;
use crate::storage::models::{Message, Session};
use crate::storage::repo;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use tauri::State;
use std::sync::Arc;
use uuid::Uuid;

#[tauri::command]
pub fn list_sessions(
    state: State<'_, AppState>,
    persona_id: Option<String>,
) -> Result<Vec<Session>, String> {
    repo::list_sessions(&state.db, persona_id.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_session(
    state: State<'_, AppState>,
    persona_id: String,
    title: Option<String>,
) -> Result<Session, String> {
    repo::create_session(&state.db, &persona_id, title.as_deref().unwrap_or("新对话"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let voices_dir = state.data_dir.join("voices").join(&session_id);
    if voices_dir.exists() {
        let _ = std::fs::remove_dir_all(&voices_dir);
    }
    repo::delete_session(&state.db, &session_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<Message>, String> {
    repo::list_messages(&state.db, &session_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_message(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    content: String,
    parts: Option<Vec<ContentPart>>,
) -> Result<Message, String> {
    let user_message_id = format!("msg_{}", Uuid::new_v4());
    let (final_content, final_parts) =
        process_audio_parts(&state, &session_id, &user_message_id, content, parts).await?;

    crate::orchestrator::pipeline::handle_send_message(
        app,
        &state,
        session_id,
        final_content,
        final_parts,
        user_message_id,
    )
    .await
}

/// 预处理消息中的音频片段：
/// - 将原始音频 bytes 保存到本地文件
/// - 获取转写文本（优先使用已提供的 transcript，否则调用 ASR）
/// - 将 file_id 与 transcript 注入文本内容
/// - 将 AudioBytes 中的 data 清空，保留 file_id、duration、transcript、mime 元数据
async fn process_audio_parts(
    state: &AppState,
    session_id: &str,
    user_message_id: &str,
    mut content: String,
    parts: Option<Vec<ContentPart>>,
) -> Result<(String, Option<Vec<ContentPart>>), String> {
    let Some(mut parts_vec) = parts else {
        return Ok((content, None));
    };

    for part in parts_vec.iter_mut() {
        if let ContentPart::AudioBytes {
            data,
            duration,
            transcript,
            mime,
            file_id,
        } = part
        {
            // 已包含 file_id 表示前端已保存，仅注入上下文即可
            if data.is_empty() && file_id.is_some() {
                if let Some(ref fid) = file_id {
                    let text = transcript.clone().unwrap_or_default();
                    content.push_str(&format!("\n[语音消息: file_id={fid}, transcript={text}]"));
                }
                continue;
            }

            let audio_b64 = B64.encode(&data);
            let new_file_id = voice::save_voice_message_impl(
                state,
                session_id,
                user_message_id,
                &audio_b64,
                *duration,
                mime.clone(),
            )
            .await?;

            let transcript_text = if let Some(ref t) = transcript {
                t.clone()
            } else {
                voice::get_voice_transcript_impl(state, &audio_b64)
                    .await
                    .unwrap_or_default()
            };

            content.push_str(&format!(
                "\n[语音消息: file_id={new_file_id}, transcript={transcript_text}]"
            ));

            *part = ContentPart::AudioBytes {
                data: Vec::new(),
                duration: *duration,
                transcript: Some(transcript_text),
                mime: mime.clone().or(Some("audio/wav".to_string())),
                file_id: Some(new_file_id),
            };
        }
    }

    Ok((content, Some(parts_vec)))
}
