use crate::perception::audio_io::{self, AudioRecorder};
use crate::state::AppState;
use crate::worker::{ipc::build_request, pool::WorkerType};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use std::sync::{mpsc, Mutex};
use std::thread::JoinHandle;
use tauri::State;

/// 录音线程句柄 + 停止信号发送端
/// 架构说明：
///   cpal::Stream 在 Windows WASAPI 后端上是 !Send，必须留在创建它的线程上。
///   因此录音全部在独立 std::thread 中完成，
///   与 Tauri 异步命令之间通过 mpsc channel 通信。
type StopTx = mpsc::Sender<mpsc::Sender<Vec<u8>>>;
static RECORDING: Mutex<Option<(JoinHandle<()>, StopTx)>> = Mutex::new(None);

fn active_asr_params(state: &AppState, audio_bytes: Vec<u8>) -> serde_json::Value {
    if let Some(runtime) = crate::commands::model::active_model_runtime(state, "asr") {
        serde_json::json!({
            "audio_bytes": audio_bytes,
            "model_size": runtime.download_id,
            "model_path": runtime.local_path,
        })
    } else {
        serde_json::json!({ "audio_bytes": audio_bytes })
    }
}

fn active_tts_backend(state: &AppState) -> Option<String> {
    crate::commands::model::active_model_runtime(state, "tts")
        .map(|runtime| crate::commands::model::tts_backend_for_provider(&runtime.provider_id).to_string())
        .filter(|backend| backend != "auto")
}

/// 解析 WAV 文件头，返回 (channels, sample_rate, bits_per_sample)
fn parse_wav_header(wav: &[u8]) -> Option<(u16, u32, u16)> {
    if wav.len() < 44 {
        return None;
    }
    if &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return None;
    }
    let channels = u16::from_le_bytes([wav[22], wav[23]]);
    let sample_rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
    let bits_per_sample = u16::from_le_bytes([wav[34], wav[35]]);
    Some((channels, sample_rate, bits_per_sample))
}

/// 根据 WAV 头信息估算时长（秒）
fn calc_wav_duration(wav: &[u8]) -> f64 {
    let (channels, sample_rate, bits_per_sample) = match parse_wav_header(wav) {
        Some(v) => v,
        None => return 0.0,
    };
    if channels == 0 || sample_rate == 0 || bits_per_sample == 0 {
        return 0.0;
    }
    let bytes_per_sample = bits_per_sample as f64 / 8.0;
    wav.len() as f64 / sample_rate as f64 / bytes_per_sample / channels as f64
}

/// 停止录音并返回 WAV bytes（内部复用）
async fn take_wav_from_recorder() -> Result<Vec<u8>, String> {
    let (handle, cmd_tx) = {
        let mut guard = RECORDING.lock().map_err(|e| e.to_string())?;
        guard.take().ok_or("未在录音")?
    };

    let (wav_tx, wav_rx) = mpsc::channel::<Vec<u8>>();
    cmd_tx.send(wav_tx).map_err(|_| "录音线程已退出".to_string())?;

    tokio::task::spawn_blocking(move || {
        let _ = handle.join();
        wav_rx.recv().unwrap_or_default()
    })
    .await
    .map_err(|e| format!("录音线程异常: {e}"))
}

#[tauri::command]
pub async fn start_recording(_state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = RECORDING.lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        return Err("已在录音中".into());
    }

    // cmd_tx: 外部 → 录音线程（发送停止命令 + WAV 回传通道）
    // 当收到停止命令时，内嵌的 Sender<Vec<u8>> 用于回传 WAV 数据
    let (cmd_tx, cmd_rx) = mpsc::channel::<mpsc::Sender<Vec<u8>>>();

    let handle = std::thread::spawn(move || {
        let mut recorder = match AudioRecorder::new().and_then(|mut r| r.start().map(|_| r)) {
            Ok(r) => r,
            Err(e) => {
                log::error!("启动录音失败: {e}");
                return;
            }
        };
        log::info!("🎤 录音已开始");

        // 阻塞等待停止命令（在录音线程上 block，没问题）
        match cmd_rx.recv() {
            Ok(wav_tx) => {
                match recorder.stop() {
                    Ok(wav_bytes) => {
                        log::info!("🎤 录音已停止: {} bytes", wav_bytes.len());
                        let _ = wav_tx.send(wav_bytes);
                    }
                    Err(e) => {
                        log::error!("停止录音失败: {e}");
                    }
                }
            }
            Err(_) => {
                // cmd_tx 被 drop → 取消录音
                log::info!("🎤 录音已取消");
            }
        }
        // recorder 在此析构，cpal::Stream 停止
    });

    *guard = Some((handle, cmd_tx));
    Ok(())
}

#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> Result<String, String> {
    let wav_bytes = take_wav_from_recorder().await?;

    if wav_bytes.is_empty() {
        return Err("录音数据为空（可能麦克风未授权或设备静音）".into());
    }

    log::info!("🎤 收到录音数据: {} bytes，发送给 ASR Worker", wav_bytes.len());

    // 发送给 ASR Worker
    let params = active_asr_params(&state, wav_bytes);
    let req = build_request("transcribe", params);

    let pool = state.worker_pool.clone();
    let resp = tokio::task::spawn_blocking(move || pool.call(WorkerType::Asr, req))
        .await
        .map_err(|e| format!("ASR worker 调度失败: {e}"))?
        .map_err(|e| format!("ASR worker 错误: {e}"))?;

    if let Some(err) = resp.error {
        return Err(format!("ASR 识别失败: {err}"));
    }

    let text = resp
        .result
        .and_then(|v| v.get("text").and_then(|t| t.as_str()).map(|s| s.to_string()))
        .unwrap_or_default();

    log::info!("🎤 ASR 结果: {}", text);
    Ok(text)
}

#[tauri::command]
pub async fn stop_recording_audio(_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let wav_bytes = take_wav_from_recorder().await?;

    if wav_bytes.is_empty() {
        return Err("录音数据为空（可能麦克风未授权或设备静音）".into());
    }

    let duration = calc_wav_duration(&wav_bytes);
    let audio_b64 = B64.encode(&wav_bytes);

    log::info!("🎤 停止录音并返回音频: {} bytes, duration={:.2}s", wav_bytes.len(), duration);

    Ok(serde_json::json!({
        "audio": audio_b64,
        "duration": duration,
        "mime": "audio/wav"
    }))
}

#[tauri::command]
pub async fn save_voice_message(
    state: State<'_, AppState>,
    session_id: String,
    message_id: String,
    audio_base64: String,
    duration: Option<f64>,
    mime: Option<String>,
) -> Result<String, String> {
    save_voice_message_impl(&state, &session_id, &message_id, &audio_base64, duration, mime).await
}

pub async fn save_voice_message_impl(
    state: &AppState,
    session_id: &str,
    message_id: &str,
    audio_base64: &str,
    _duration: Option<f64>,
    _mime: Option<String>,
) -> Result<String, String> {
    let wav_bytes = B64
        .decode(audio_base64)
        .map_err(|e| format!("base64 解码失败: {e}"))?;

    if wav_bytes.is_empty() {
        return Err("音频数据为空".into());
    }

    let voices_dir = state.data_dir.join("voices").join(session_id);
    std::fs::create_dir_all(&voices_dir).map_err(|e| format!("创建目录失败: {e}"))?;

    let file_name = format!("{message_id}.wav");
    let file_path = voices_dir.join(&file_name);

    tokio::task::spawn_blocking(move || std::fs::write(&file_path, &wav_bytes))
        .await
        .map_err(|e| format!("写入文件失败: {e}"))?
        .map_err(|e| format!("写入文件失败: {e}"))?;

    let file_id = format!("{session_id}/{message_id}.wav");
    log::info!("💾 语音消息已保存: {}", file_id);
    Ok(file_id)
}

#[tauri::command]
pub async fn play_voice_message(state: State<'_, AppState>, file_id: String) -> Result<(), String> {
    play_voice_message_impl(&state, &file_id).await
}

pub async fn play_voice_message_impl(state: &AppState, file_id: &str) -> Result<(), String> {
    let base = state.data_dir.join("voices");
    let path = file_id.split('/').fold(base, |p, seg| p.join(seg));

    let wav_bytes = tokio::task::spawn_blocking(move || std::fs::read(&path))
        .await
        .map_err(|e| format!("读取文件失败: {e}"))?
        .map_err(|e| format!("读取文件失败: {e}"))?;

    log::info!("▶️ 播放语音消息: {} ({} bytes)", file_id, wav_bytes.len());

    tokio::task::spawn_blocking(move || audio_io::play_wav(&wav_bytes))
        .await
        .map_err(|e| format!("播放失败: {e}"))?
        .map_err(|e| format!("播放失败: {e}"))?;

    Ok(())
}

#[tauri::command]
pub async fn get_voice_transcript(
    state: State<'_, AppState>,
    audio_base64: String,
) -> Result<String, String> {
    get_voice_transcript_impl(&state, &audio_base64).await
}

pub async fn get_voice_transcript_impl(state: &AppState, audio_base64: &str) -> Result<String, String> {
    let wav_bytes = B64
        .decode(audio_base64)
        .map_err(|e| format!("base64 解码失败: {e}"))?;

    if wav_bytes.is_empty() {
        return Err("音频数据为空".into());
    }

    let params = active_asr_params(state, wav_bytes);
    let req = build_request("transcribe", params);

    let pool = state.worker_pool.clone();
    let resp = tokio::task::spawn_blocking(move || pool.call(WorkerType::Asr, req))
        .await
        .map_err(|e| format!("ASR worker 调度失败: {e}"))?
        .map_err(|e| format!("ASR worker 错误: {e}"))?;

    if let Some(err) = resp.error {
        return Err(format!("ASR 识别失败: {err}"));
    }

    let text = resp
        .result
        .and_then(|v| v.get("text").and_then(|t| t.as_str()).map(|s| s.to_string()))
        .unwrap_or_default();

    log::info!("🎤 语音转写结果: {}", text);
    Ok(text)
}

#[tauri::command]
pub async fn synthesize_speech(
    state: State<'_, AppState>,
    text: String,
    voice_id: Option<String>,
    persona_id: Option<String>,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("合成文本为空".into());
    }

    // 如果指定了角色，读取角色的 voice 配置
    let active_backend = active_tts_backend(&state);
    let (tts_backend, tts_voice_id, tts_speed) = if let Some(ref pid) = persona_id {
        match crate::storage::repo::get_persona(&state.db, pid) {
            Ok(Some(row)) => {
                let voice = &row.definition.voice;
                let engine = voice.get("ttsEngine").and_then(|v| v.as_str()).unwrap_or("auto");
                let vid = voice.get("voiceId").and_then(|v| v.as_str()).map(|s| s.to_string());
                let speed = voice.get("params").and_then(|p| p.get("speed")).and_then(|v| v.as_f64()).unwrap_or(1.0);
                let backend = if engine == "auto" {
                    active_backend.clone().unwrap_or_else(|| "auto".to_string())
                } else {
                    engine.to_string()
                };
                (backend, vid.or(voice_id), speed)
            }
            _ => (active_backend.unwrap_or_else(|| "auto".to_string()), voice_id, 1.0),
        }
    } else {
        (active_backend.unwrap_or_else(|| "auto".to_string()), voice_id, 1.0)
    };

    let params = serde_json::json!({
        "text": text,
        "voice_id": tts_voice_id.unwrap_or_default(),
        "backend": tts_backend,
        "speed": tts_speed,
    });
    let req = build_request("synthesize", params);

    let pool = state.worker_pool.clone();
    let resp = tokio::task::spawn_blocking(move || pool.call(WorkerType::Tts, req))
        .await
        .map_err(|e| format!("TTS 任务调度失败: {e}"))?
        .map_err(|e| format!("TTS Worker 错误: {e}"))?;

    if let Some(err) = resp.error {
        return Err(format!("TTS 合成失败: {err}"));
    }

    let audio_b64 = resp
        .result
        .and_then(|v| v.get("audio_data").and_then(|a| a.as_str()).map(|s| s.to_string()))
        .ok_or("TTS 返回数据为空")?;

    let wav_bytes = B64
        .decode(&audio_b64)
        .map_err(|e| format!("base64 解码失败: {e}"))?;

    log::info!("🔊 TTS 合成完成: {} bytes", wav_bytes.len());

    tokio::task::spawn_blocking(move || audio_io::play_wav(&wav_bytes))
        .await
        .map_err(|e| format!("播放失败: {e}"))?
        .map_err(|e| format!("播放失败: {e}"))?;

    Ok(())
}

#[tauri::command]
pub fn cancel_recording() -> Result<(), String> {
    let mut guard = RECORDING.lock().map_err(|e| e.to_string())?;
    if let Some((handle, cmd_tx)) = guard.take() {
        drop(cmd_tx); // 断开通道 → 录音线程的 cmd_rx.recv() 返回 Err → 线程退出
        let _ = handle.join();
        log::info!("🎤 录音已取消");
    }
    Ok(())
}

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<String>, String> {
    audio_io::list_devices().map_err(|e| e.to_string())
}
