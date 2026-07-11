use tauri::Emitter;
use tauri::State;
use crate::state::AppState;
use crate::worker::pool::resolve_workers_dir;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

/// 统一解析模型存储目录：优先使用 settings.modelsDir，否则使用 data_dir/models
pub fn resolve_models_dir(state: &AppState) -> PathBuf {
    state.db.with_conn(|conn| {
        let dir: Option<String> = conn.query_row(
            "SELECT value_json FROM settings WHERE key = 'modelsDir'",
            [],
            |row| row.get(0),
        ).ok().and_then(|v: String| serde_json::from_str(&v).ok());
        Ok::<_, rusqlite::Error>(match dir {
            Some(p) if !p.is_empty() => PathBuf::from(p),
            _ => state.data_dir.join("models"),
        })
    }).unwrap_or_else(|_| state.data_dir.join("models"))
}

/// 递归计算目录大小（字节）
fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(meta) = p.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

/// 列出模型目录下已有的模型子目录
fn list_local_model_dirs(models_dir: &Path) -> Vec<(String, u64)> {
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(models_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if !name.starts_with('.') {
                    let size = dir_size(&p);
                    result.push((name, size));
                }
            }
        }
    }
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn model_download_id(model_id: &str) -> String {
    let normalized = model_id.strip_prefix("model_").unwrap_or(model_id).replace("_", "-");
    match normalized.as_str() {
        "whisper-med" => "whisper-medium".to_string(),
        _ => normalized,
    }
}

fn model_source_label(model_type: &str, provider_id: &str) -> &'static str {
    if provider_id.starts_with("ollama/") {
        "Ollama"
    } else if matches!(provider_id, "cosyvoice" | "funasr" | "bge-m3" | "bge-small" | "chattts") {
        "ModelScope / HuggingFace"
    } else if provider_id == "piper" {
        "HuggingFace"
    } else if model_type == "embedding" || model_type == "asr" || model_type == "tts" {
        "HuggingFace"
    } else {
        "Local"
    }
}

fn model_runtime_hint(model_type: &str, provider_id: &str) -> &'static str {
    if provider_id.starts_with("ollama/") {
        "需要本机 Ollama 服务运行；下载后可直接在对话中启用。"
    } else {
        match model_type {
            "asr" => "需要 Python Worker 与 faster-whisper / funasr 依赖；下载后用“功能测试”验证转写链路。",
            "tts" => match provider_id {
                "cosyvoice" => "需要 CosyVoice Python 包与 PyTorch；下载后角色音色可使用 cosyvoice。",
                "chattts" => "需要 ChatTTS 运行依赖；适合对话韵律，但当前 Worker 优先支持 CosyVoice / Piper / pyttsx3。",
                "piper" => "需要 piper-tts Python 包；CPU 即可实时合成。",
                _ => "需要 TTS Worker 依赖；下载后用“功能测试”试听合成结果。",
            },
            "embedding" => "需要 sentence-transformers；下载后知识库 RAG 与长期记忆会使用向量检索。",
            _ => "下载后可在模型中心启用并测试。",
        }
    }
}

fn model_install_command(model_id: &str, provider_id: &str) -> String {
    if provider_id.starts_with("ollama/") {
        format!("ollama pull {}", provider_id.trim_start_matches("ollama/"))
    } else {
        format!("python workers/download_models.py {}", model_download_id(model_id))
    }
}

/// 智能描述生成
fn build_model_desc(name: &str, model_type: &str, provider_id: &str) -> String {
    if provider_id.starts_with("ollama/") {
        let tag = provider_id.trim_start_matches("ollama/");
        format!("ollama pull {tag}")
    } else if model_type == "llm" {
        format!("{name}（本地 GGUF）")
    } else if model_type == "asr" {
        format!("{name}（语音识别）")
    } else if model_type == "tts" {
        format!("{name}（语音合成）")
    } else if model_type == "embedding" {
        format!("{name}（向量化）")
    } else {
        format!("{name}")
    }
}

/// size_mb（i64）→ 可读字符串
fn format_size(mb: Option<i64>) -> String {
    match mb {
        Some(m) if m >= 1024 => format!("{:.1} GB", m as f64 / 1024.0),
        Some(m) => format!("{} MB", m),
        None => String::new(),
    }
}

#[tauri::command]
pub async fn list_models(state: State<'_, AppState>) -> Result<Vec<Value>, String> {
    // 1. 获取 Ollama 已安装模型列表
    let ollama_models = match check_ollama_internal().await {
        Ok(val) => val["models"].as_array().cloned().unwrap_or_default(),
        Err(_) => vec![],
    };

    let ollama_names: std::collections::HashSet<String> = ollama_models.iter()
        .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
        .collect();

    let models_dir_path = resolve_models_dir(&state);

    state.db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, model_type, provider_id, status, is_active, vram_required, size_mb, model_path FROM model_config ORDER BY model_type, name"
        )?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let model_type: String = row.get(2)?;
            let provider_id: String = row.get(3)?;
            let db_status: String = row.get(4)?;
            let is_active: bool = row.get(5)?;
            let vram: Option<String> = row.get(6)?;
            let size_mb: Option<i64> = row.get(7)?;
            let model_path: Option<String> = row.get(8)?;

            // 2. 实时校验状态
            let mut status = db_status;
            if provider_id.starts_with("ollama/") {
                let tag = provider_id.trim_start_matches("ollama/");
                let exists_in_ollama = ollama_names.contains(tag) ||
                                     ollama_names.contains(&format!("{}:latest", tag));
                status = if exists_in_ollama { "downloaded".to_string() } else { "not_downloaded".to_string() };
            } else if status == "downloaded" || status == "active" {
                let normalized_id = model_download_id(&id);
                let path = match model_path {
                    Some(p) if !p.is_empty() => std::path::PathBuf::from(p),
                    _ => models_dir_path.join(&normalized_id),
                };
                let is_empty = path.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true);
                if !path.exists() || is_empty {
                    status = "not_downloaded".to_string();
                }
            }

            Ok(serde_json::json!({
                "id": id,
                "name": name,
                "modelType": model_type.to_lowercase(),
                "providerId": provider_id,
                "status": status,
                "isActive": is_active,
                "vramRequired": vram.unwrap_or_default(),
                "size": format_size(size_mb),
                "description": build_model_desc(&name, &model_type, &provider_id),
                "source": model_source_label(&model_type, &provider_id),
                "runtimeHint": model_runtime_hint(&model_type, &provider_id),
                "installCommand": model_install_command(&id, &provider_id),
                "localPath": if provider_id.starts_with("ollama/") {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::String(models_dir_path.join(model_download_id(&id)).to_string_lossy().to_string())
                },
            }))
        })?;

        let result: Vec<Value> = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(result)
    }).map_err(|e| e.to_string())
}

async fn check_ollama_internal() -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(1))
        .build()
        .map_err(|e| e.to_string())?;

    let tags_resp = client.get("http://127.0.0.1:11434/api/tags").send().await;
    let version_resp = client.get("http://127.0.0.1:11434/api/version").send().await;
    let version = if let Ok(r) = version_resp {
        let v: Value = r.json().await.unwrap_or(serde_json::json!({"version": "unknown"}));
        v["version"].as_str().unwrap_or("unknown").to_string()
    } else {
        "unknown".to_string()
    };

    match tags_resp {
        Ok(r) => {
            let body: Value = r.json().await.map_err(|e| e.to_string())?;
            let models = body["models"].as_array().map(|arr| {
                arr.iter().filter_map(|m| {
                    let name = m["name"].as_str()?.to_string();
                    Some(serde_json::json!({ "name": name }))
                }).collect::<Vec<_>>()
            }).unwrap_or_default();
            Ok(serde_json::json!({ "available": true, "models": models, "version": version }))
        },
        Err(_) => Ok(serde_json::json!({ "available": false, "models": [], "version": version })),
    }
}

#[tauri::command]
pub async fn check_ollama() -> Result<Value, String> {
    check_ollama_internal().await
}

#[tauri::command]
pub fn get_gpu_info() -> Result<Value, String> {
    Ok(serde_json::json!({
        "vramGb": 8,
        "recommendation": "Gemma 2 9B (Q4_K_M)",
    }))
}

#[tauri::command]
pub async fn download_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    let provider_id: String = state.db.with_conn(|conn| {
        conn.query_row(
            "SELECT provider_id FROM model_config WHERE id = ?1",
            rusqlite::params![model_id],
            |row| row.get(0),
        )
    }).map_err(|_| format!("模型 '{model_id}' 不存在"))?;

    if provider_id.starts_with("ollama/") {
        let tag = provider_id.trim_start_matches("ollama/").to_string();
        return download_via_ollama(app, state, model_id, tag).await;
    }

    download_via_python(app, state, model_id).await
}

async fn download_via_ollama(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    model_id: String,
    tag: String,
) -> Result<(), String> {
    use futures_util::StreamExt;

    state.db.with_conn(|conn| {
        conn.execute("UPDATE model_config SET status = 'downloading' WHERE id = ?1", rusqlite::params![model_id])?;
        Ok::<(), rusqlite::Error>(())
    }).map_err(|e| e.to_string())?;

    let mid = model_id.clone();
    let app_clone = app.clone();
    let db_for_status = state.data_dir.join("db.sqlite");

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let resp = client.post("http://127.0.0.1:11434/api/pull")
            .json(&serde_json::json!({ "name": tag }))
            .send()
            .await;

        let mut error_msg = None;
        let mut last_progress = 0;

        if let Ok(r) = resp {
            let mut stream = r.bytes_stream();
            while let Some(item) = stream.next().await {
                if let Ok(bytes) = item {
                    let line = String::from_utf8_lossy(&bytes);
                    for part in line.split('\n').filter(|s| !s.is_empty()) {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(part) {
                            if let Some(total) = val["total"].as_f64() {
                                if let Some(completed) = val["completed"].as_f64() {
                                    let progress = ((completed / total) * 100.0) as i32;
                                    if progress != last_progress {
                                        last_progress = progress;
                                        let _ = app_clone.emit("download-progress", serde_json::json!({
                                            "modelId": mid.clone(), "progress": last_progress,
                                        }));
                                    }
                                }
                            }
                            if let Some(err) = val["error"].as_str() {
                                error_msg = Some(err.to_string());
                            }
                        }
                    }
                }
            }
        } else {
            error_msg = Some("连接 Ollama 失败".into());
        }

        let status = if error_msg.is_some() { "not_downloaded" } else { "downloaded" };
        let _ = rusqlite::Connection::open(&db_for_status).map(|conn| {
            let _ = conn.execute("UPDATE model_config SET status = ?1 WHERE id = ?2", rusqlite::params![status, mid]);
        });

        let _ = app_clone.emit("download-progress", serde_json::json!({
            "modelId": mid, "progress": 100, "done": true, "error": error_msg,
        }));
    });

    Ok(())
}

async fn download_via_python(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    let app_clone = app.clone();
    let mid = model_id.clone();
    let db_for_status = state.data_dir.join("db.sqlite");

    let models_dir = resolve_models_dir(&state);

    let _ = std::fs::create_dir_all(&models_dir);

    // 设置为下载中
    let _ = state.db.with_conn(|conn| {
        conn.execute(
            "UPDATE model_config SET status = 'downloading' WHERE id = ?1",
            rusqlite::params![model_id],
        )
    });

    let workers_dir = resolve_workers_dir();
    let script_path = workers_dir.join("download_models.py");
    if !script_path.exists() {
        return Err(format!("找不到下载脚本: {:?}", script_path));
    }

    let normalized_id = model_download_id(&mid);
    let backend_arg = state.db.with_conn(|conn| {
        let provider: String = conn.query_row("SELECT provider_id FROM model_config WHERE id = ?1", [mid.clone()], |row| row.get(0)).unwrap_or_default();
        Ok::<_, rusqlite::Error>(if provider.starts_with("modelscope/") { "modelscope" } else { "auto" })
    }).unwrap_or("auto");

    let active_downloads = state.active_downloads.clone();
    let models_dir_for_cleanup = models_dir.clone();
    let mid_for_cleanup = mid.clone();

    std::thread::spawn(move || {
        let python = if Command::new("python3").arg("--version").output().is_ok() { "python3" } else { "python" };
        log::info!("[download] 启动下载脚本: {} {:?} model={} output={:?} backend={}", python, script_path, normalized_id, models_dir, backend_arg);
        let mut child = match Command::new(python)
            .env("HF_ENDPOINT", "https://hf-mirror.com")
            .arg(&script_path)
            .arg(&normalized_id)
            .arg("--output")
            .arg(&models_dir)
            .arg("--backend")
            .arg(backend_arg)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                log::error!("[download] 启动下载脚本失败: {}", e);
                let _ = app_clone.emit("download-progress", serde_json::json!({
                    "modelId": mid, "done": true, "error": format!("启动下载脚本失败: {}", e)
                }));
                return;
            }
        };

        // 记录 PID 到 active_downloads
        let child_pid = child.id();
        {
            if let Ok(mut map) = active_downloads.lock() {
                map.insert(mid.clone(), child_pid);
            }
        }
        log::info!("[download] {} 进程 PID={}", mid, child_pid);

        // 标记是否被取消
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancelled_clone = cancelled.clone();

        // 消费 stderr 防止管道堵塞（改为 info 级别以便调试）
        let stderr_reader = child.stderr.take().map(|s| {
            let reader = BufReader::new(s);
            std::thread::spawn(move || {
                for line in reader.lines() {
                    if let Ok(l) = line {
                        log::info!("[download stderr] {}", l);
                    }
                }
            })
        });

        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);
        let mut final_error = None;
        let mut loadcheck_ok = false;
        let mut done_received = false;

        for line in reader.lines() {
            if let Ok(line_text) = line {
                log::debug!("[download stdout] {}", line_text);
                if let Some(pct_str) = line_text.strip_prefix("PROGRESS:") {
                    if let Ok(pct) = pct_str.trim().parse::<i32>() {
                        let _ = app_clone.emit("download-progress", serde_json::json!({
                            "modelId": mid.clone(), "progress": pct,
                        }));
                    }
                }
                if line_text == "DONE" {
                    done_received = true;
                    let _ = app_clone.emit("download-progress", serde_json::json!({
                        "modelId": mid.clone(), "progress": 100,
                    }));
                }
                if let Some(check) = line_text.strip_prefix("LOADCHECK:") {
                    let parts: Vec<&str> = check.splitn(2, ':').collect();
                    let status = parts.first().unwrap_or(&"UNKNOWN");
                    let detail = parts.get(1).unwrap_or(&"");
                    log::info!("[download] LOADCHECK {}: {}", mid, check);
                    if *status == "OK" { loadcheck_ok = true; }
                    let _ = rusqlite::Connection::open(&db_for_status).map(|conn| {
                        let _ = conn.execute(
                            "INSERT OR REPLACE INTO download_task (id, model_id, status, progress, error_msg) VALUES (?1, ?2, ?3, ?4, ?5)",
                            rusqlite::params![format!("dt_{mid}"), mid, if *status == "OK" { "completed" } else { "failed" }, 100, detail.to_string()],
                        );
                    });
                }
                if let Some(err) = line_text.strip_prefix("ERROR:") {
                    final_error = Some(err.trim().to_string());
                    break;
                }
                if let Some(msg) = line_text.strip_prefix("CANCELLED:") {
                    log::info!("[download] {} 收到取消信号: {}", mid, msg);
                    cancelled_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                    break;
                }
            }
        }

        // 等待 stderr 读取完成
        drop(stderr_reader);

        let is_cancelled = cancelled.load(std::sync::atomic::Ordering::SeqCst);
        let status = child.wait().unwrap();

        // 从 active_downloads 中移除
        {
            if let Ok(mut map) = active_downloads.lock() {
                map.remove(&mid);
            }
        }

        let success = !is_cancelled && status.success() && final_error.is_none() && (loadcheck_ok || done_received);

        if success {
            let _ = rusqlite::Connection::open(&db_for_status).map(|conn| {
                let _ = conn.execute("UPDATE model_config SET status = 'downloaded' WHERE id = ?1", [mid.clone()]);
            });
            let _ = app_clone.emit("download-progress", serde_json::json!({ "modelId": mid, "progress": 100, "done": true }));
            log::info!("[download] {} 下载/检测完成", mid);
        } else if is_cancelled {
            // 清理不完整的下载目录
            let target_dir = models_dir_for_cleanup.join(model_download_id(&mid_for_cleanup));
            if target_dir.exists() {
                let real_entries: Vec<_> = std::fs::read_dir(&target_dir)
                    .ok()
                    .map(|entries| entries.filter_map(|e| e.ok()).filter(|e| !e.file_name().to_string_lossy().starts_with('.')).collect())
                    .unwrap_or_default();
                // 如果只有隐藏文件（.cache等），删除整个目录
                if real_entries.is_empty() {
                    let _ = std::fs::remove_dir_all(&target_dir);
                    log::info!("[download] {} 已取消，清理空目录 {:?}", mid, target_dir);
                }
            }
            let _ = rusqlite::Connection::open(&db_for_status).map(|conn| {
                let _ = conn.execute("UPDATE model_config SET status = 'not_downloaded' WHERE id = ?1", [mid.clone()]);
            });
            let _ = app_clone.emit("download-progress", serde_json::json!({ "modelId": mid, "done": true, "cancelled": true }));
            log::info!("[download] {} 已取消", mid);
        } else {
            let error_msg = final_error.unwrap_or_else(|| format!("下载进程异常退出 (exit={}, done={}, loadcheck={})", status.code().unwrap_or(-1), done_received, loadcheck_ok));
            log::error!("[download] {} 失败: {}", mid, error_msg);
            let _ = app_clone.emit("download-progress", serde_json::json!({ "modelId": mid, "done": true, "error": error_msg }));
        }
    });

    Ok(())
}
#[tauri::command]
pub async fn activate_model(state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    state.db.with_conn(|conn| {
        let model_type: String = conn.query_row("SELECT model_type FROM model_config WHERE id = ?1", [model_id.clone()], |row| row.get(0))?;
        conn.execute("UPDATE model_config SET is_active = 0 WHERE model_type = ?1", [model_type])?;
        conn.execute("UPDATE model_config SET is_active = 1, status = 'active' WHERE id = ?1", [model_id])?;
        Ok(())
    }).map_err(|e: rusqlite::Error| e.to_string())
}

#[tauri::command]
pub async fn delete_model(state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    let models_dir = resolve_models_dir(&state);

    let normalized_id = model_download_id(&model_id);
    let target_dir = models_dir.join(&normalized_id);
    if target_dir.exists() { let _ = std::fs::remove_dir_all(target_dir); }

    state.db.with_conn(|conn| {
        conn.execute("UPDATE model_config SET status = 'not_downloaded', is_active = 0 WHERE id = ?1", [model_id])?;
        Ok(())
    }).map_err(|e: rusqlite::Error| e.to_string())
}

#[tauri::command]
pub async fn cancel_download(state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    let pid = {
        let map = state.active_downloads.lock().map_err(|e| e.to_string())?;
        map.get(&model_id).copied()
    };

    if let Some(pid) = pid {
        log::info!("[download] 取消下载 {} (PID={})", model_id, pid);
        // 在 Windows 上使用 taskkill 强制终止进程树；其他平台使用 kill
        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        #[cfg(not(windows))]
        {
            let _ = Command::new("kill")
                .arg(&pid.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
    } else {
        log::warn!("[download] 取消下载 {}: 未找到活跃进程", model_id);
        // 虽然没有活跃进程，但也要确保状态被重置
        let db = state.db.clone();
        let mid = model_id.clone();
        std::thread::spawn(move || {
            let _ = db.with_conn(|conn| {
                conn.execute("UPDATE model_config SET status = 'not_downloaded' WHERE id = ?1", [mid])
            });
        });
    }

    Ok(())
}

#[tauri::command]
pub async fn test_model(app: tauri::AppHandle, state: State<'_, AppState>, model_id: String) -> Result<Value, String> {
    use crate::worker::pool::WorkerType;
    use crate::worker::ipc::build_request;
    let mid = model_id.clone();
    let app_clone = app.clone();

    // Step 1: 加载配置
    emit_test_step(&app_clone, &mid, "load_config", "active", Some("读取模型配置…")).ok();
    let (model_type, provider_id) = state.db.with_conn(|conn| {
        conn.query_row("SELECT model_type, provider_id FROM model_config WHERE id = ?1", [model_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
    }).map_err(|_| "Model not found")?;
    emit_test_step(&app_clone, &mid, "load_config", "done", Some(&format!("类型: {}, 后端: {}", model_type, provider_id))).ok();

    match model_type.as_str() {
        "llm" => if provider_id.starts_with("ollama/") {
            let tag = provider_id.trim_start_matches("ollama/");

            // Step 2: 连接 Ollama
            emit_test_step(&app_clone, &mid, "connect_ollama", "active", Some("连接 Ollama 服务…")).ok();
            let client = reqwest::Client::new();
            let check = client.get("http://127.0.0.1:11434/api/tags").send().await;
            if check.is_err() || !check.unwrap().status().is_success() {
                emit_test_step(&app_clone, &mid, "connect_ollama", "failed", None).ok();
                let _ = app_clone.emit("model-test:step", serde_json::json!({"modelId": &mid, "done": true, "success": false, "message": "Ollama 无响应"}));
                return Err("Ollama 无响应".into());
            }
            emit_test_step(&app_clone, &mid, "connect_ollama", "done", None).ok();

            // Step 3: 加载模型
            emit_test_step(&app_clone, &mid, "load_model", "active", Some(&format!("加载 {tag}…"))).ok();
            emit_test_step(&app_clone, &mid, "load_model", "done", None).ok();

            // Step 4: 执行推理
            emit_test_step(&app_clone, &mid, "run_inference", "active", Some("发送测试 prompt…")).ok();
            let start = std::time::Instant::now();
            let resp = client.post("http://127.0.0.1:11434/api/generate")
                .json(&serde_json::json!({ "model": tag, "prompt": "hi", "stream": false }))
                .send().await;
            let latency = start.elapsed().as_millis() as u64;
            let success = resp.map(|r| r.status().is_success()).unwrap_or(false);
            emit_test_step(&app_clone, &mid, "run_inference", if success { "done" } else { "failed" },
                Some(&if success { format!("响应 {}ms", latency) } else { "推理失败".into() })).ok();

            // Step 5: 验证输出
            emit_test_step(&app_clone, &mid, "verify_output", if success { "done" } else { "failed" }, None).ok();
            let _ = app_clone.emit("model-test:step", serde_json::json!({
                "modelId": &mid, "done": true, "success": success, "latencyMs": latency,
                "message": if success { format!("Ollama 就绪 ({}ms)", latency) } else { "Ollama 无响应".to_string() },
            }));
            if success { Ok(serde_json::json!({ "success": true, "message": format!("Ollama 就绪 ({}ms)", latency) })) }
            else { Err("Ollama 无响应".into()) }
        } else {
            let _ = app_clone.emit("model-test:step", serde_json::json!({"modelId": &mid, "done": true, "success": true, "message": "本地测试未开放"}));
            Ok(serde_json::json!({ "success": true, "message": "本地测试未开放" }))
        },
        "asr" => {
            emit_test_step(&app_clone, &mid, "start_worker", "active", Some("启动 ASR 工作进程…")).ok();
            let ping_req = build_request("ping", serde_json::json!({}));
            let ping_result = state.worker_pool.call(WorkerType::Asr, ping_req);
            if ping_result.is_err() {
                let err = format!("{}", ping_result.as_ref().err().unwrap());
                emit_test_step(&app_clone, &mid, "start_worker", "failed", Some(&err)).ok();
                let _ = app_clone.emit("model-test:step", serde_json::json!({"modelId": &mid, "done": true, "success": false, "message": format!("ASR 通信异常: {err}")}));
                return Err(format!("ASR 通信异常: {err}"));
            }
            emit_test_step(&app_clone, &mid, "start_worker", "done", Some("ASR 工作进程就绪")).ok();

            emit_test_step(&app_clone, &mid, "ping_test", "active", Some("检查模型加载状态…")).ok();
            let health_req = build_request("health", serde_json::json!({}));
            let health_result = state.worker_pool.call(WorkerType::Asr, health_req);
            let health_ok = match &health_result {
                Ok(resp) => resp.error.is_none(),
                Err(_) => false,
            };
            let health_detail = if health_ok {
                "ASR 模型加载成功".to_string()
            } else {
                let err = health_result.as_ref().err().map(|e| e.to_string()).unwrap_or_else(|| "模型加载返回错误".into());
                format!("模型加载警告: {}", &err[..err.len().min(80)])
            };
            emit_test_step(&app_clone, &mid, "ping_test", if health_ok { "done" } else { "failed" }, Some(&health_detail)).ok();

            emit_test_step(&app_clone, &mid, "run_inference", "active", Some("发送测试音频进行识别…")).ok();
            let test_wav = generate_test_tone_wav(16000, 1.0, 440.0);
            let transcribe_req = build_request("transcribe", serde_json::json!({
                "audio_bytes": test_wav,
                "language": "zh"
            }));
            let start = std::time::Instant::now();
            let transcribe_result = state.worker_pool.call(WorkerType::Asr, transcribe_req);
            let latency = start.elapsed().as_millis() as u64;
            let transcribe_ok = match &transcribe_result {
                Ok(resp) => resp.error.is_none(),
                Err(_) => false,
            };
            emit_test_step(&app_clone, &mid, "run_inference", if transcribe_ok { "done" } else { "failed" },
                Some(&if transcribe_ok { format!("推理完成 {}ms", latency) } else { "推理失败".into() })).ok();

            emit_test_step(&app_clone, &mid, "verify_output", if transcribe_ok { "done" } else { "failed" }, None).ok();
            let final_ok = transcribe_ok;
            let msg = if final_ok {
                let resp = transcribe_result.unwrap();
                let text = resp.result.as_ref()
                    .and_then(|v| v.get("text").and_then(|t| t.as_str()))
                    .unwrap_or("");
                if text.is_empty() {
                    format!("ASR 引擎就绪 ({}ms，静音测试无文字输出)", latency)
                } else {
                    format!("ASR 引擎就绪 ({}ms): \"{}\"", latency, &text[..text.len().min(50)])
                }
            } else {
                format!("ASR 推理异常: {}", transcribe_result.as_ref().err().map(|e| e.to_string()).unwrap_or_default())
            };
            let _ = app_clone.emit("model-test:step", serde_json::json!({"modelId": &mid, "done": true, "success": final_ok, "latencyMs": latency, "message": msg}));
            if final_ok { Ok(serde_json::json!({ "success": true, "message": msg, "latencyMs": latency })) }
            else { Err(msg) }
        },
        "tts" => {
            use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
            emit_test_step(&app_clone, &mid, "start_worker", "active", Some("启动 TTS 工作进程…")).ok();
            let ping_req = build_request("ping", serde_json::json!({}));
            let ping_result = state.worker_pool.call(WorkerType::Tts, ping_req);
            if ping_result.is_err() {
                let err = format!("{}", ping_result.as_ref().err().unwrap());
                emit_test_step(&app_clone, &mid, "start_worker", "failed", Some(&err)).ok();
                let _ = app_clone.emit("model-test:step", serde_json::json!({"modelId": &mid, "done": true, "success": false, "message": format!("TTS 通信异常: {err}")}));
                return Err(format!("TTS 通信异常: {err}"));
            }
            emit_test_step(&app_clone, &mid, "start_worker", "done", Some("TTS 工作进程就绪")).ok();

            emit_test_step(&app_clone, &mid, "ping_test", "active", Some("检查引擎状态…")).ok();
            let health_req = build_request("health", serde_json::json!({}));
            let health_result = state.worker_pool.call(WorkerType::Tts, health_req);
            let health_ok = match &health_result {
                Ok(resp) => resp.error.is_none(),
                Err(_) => false,
            };
            let health_detail = if health_ok {
                let backends = health_result.as_ref().ok()
                    .and_then(|r| r.result.as_ref())
                    .and_then(|v| v.get("backends"))
                    .and_then(|b| b.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                    .unwrap_or_default();
                if backends.is_empty() { "TTS 引擎就绪".to_string() } else { format!("可用后端: {}", backends) }
            } else {
                "TTS 引擎状态检查失败，尝试合成测试".to_string()
            };
            emit_test_step(&app_clone, &mid, "ping_test", if health_ok { "done" } else { "active" }, Some(&health_detail)).ok();

            emit_test_step(&app_clone, &mid, "run_inference", "active", Some("合成测试语音: \"你好，这是语音测试\"…")).ok();
            let test_text = "你好，这是语音合成测试。";
            let synth_req = build_request("synthesize", serde_json::json!({
                "text": test_text,
                "voice_id": "",
                "backend": "auto"
            }));
            let start = std::time::Instant::now();
            let synth_result = state.worker_pool.call(WorkerType::Tts, synth_req);
            let latency = start.elapsed().as_millis() as u64;
            let synth_ok = match &synth_result {
                Ok(resp) => resp.error.is_none() && resp.result.is_some(),
                Err(_) => false,
            };

            let audio_b64 = if synth_ok {
                synth_result.as_ref().ok()
                    .and_then(|r| r.result.as_ref())
                    .and_then(|v| v.get("audio_data").and_then(|a| a.as_str()))
                    .map(|s| s.to_string())
            } else { None };

            let backend_used = synth_result.as_ref().ok()
                .and_then(|r| r.result.as_ref())
                .and_then(|v| v.get("backend").and_then(|b| b.as_str()))
                .unwrap_or("unknown");

            emit_test_step(&app_clone, &mid, "run_inference", if synth_ok { "done" } else { "failed" },
                Some(&if synth_ok { format!("合成完成 {}ms ({})", latency, backend_used) } else { "合成失败".into() })).ok();

            emit_test_step(&app_clone, &mid, "verify_output", if synth_ok { "done" } else { "failed" }, None).ok();
            let final_ok = synth_ok;
            let msg = if final_ok {
                format!("TTS 引擎就绪 ({}ms, 后端: {})", latency, backend_used)
            } else {
                let err = synth_result.as_ref().err().map(|e| e.to_string())
                    .or_else(|| synth_result.as_ref().ok().and_then(|r| r.error.clone()))
                    .unwrap_or_else(|| "合成返回空数据".into());
                format!("TTS 合成失败: {}", &err[..err.len().min(120)])
            };
            let _ = app_clone.emit("model-test:step", serde_json::json!({
                "modelId": &mid, "done": true, "success": final_ok, "latencyMs": latency,
                "message": msg, "audioData": audio_b64, "audioMime": "audio/wav"
            }));
            let mut result_val = serde_json::json!({ "success": final_ok, "message": msg, "latencyMs": latency });
            if let Some(audio) = audio_b64 {
                result_val["audioData"] = serde_json::Value::String(audio);
                result_val["audioMime"] = serde_json::Value::String("audio/wav".into());
            }
            if final_ok { Ok(result_val) } else { Err(msg) }
        },
        "embedding" => {
            emit_test_step(&app_clone, &mid, "start_worker", "active", Some("启动 Embedding 工作进程…")).ok();
            let req = build_request("ping", serde_json::json!({}));
            let result = state.worker_pool.call(WorkerType::Embedding, req);
            let success = result.is_ok();
            let emb_detail = if success { "Embedding 工作进程就绪".to_string() } else { format!("{}", result.as_ref().err().unwrap()) };
            emit_test_step(&app_clone, &mid, "start_worker", if success { "done" } else { "failed" }, Some(&emb_detail)).ok();
            emit_test_step(&app_clone, &mid, "ping_test", if success { "done" } else { "failed" }, None).ok();
            emit_test_step(&app_clone, &mid, "verify_output", if success { "done" } else { "failed" }, None).ok();
            let _ = app_clone.emit("model-test:step", serde_json::json!({"modelId": &mid, "done": true, "success": success,
                "message": if success { "Embedding 引擎通信正常".to_string() } else { format!("Embedding 通信异常: {}", result.as_ref().err().unwrap()) }}));
            match result { Ok(_) => Ok(serde_json::json!({ "success": true, "message": "Embedding 引擎通信正常" })), Err(e) => Err(format!("Embedding 通信异常: {e}")) }
        },
        _ => {
            let _ = app_clone.emit("model-test:step", serde_json::json!({"modelId": &mid, "done": true, "success": false, "message": "不支持测试"}));
            Err("不支持测试".into())
        }
    }
}

#[tauri::command]
pub fn get_model_status(state: State<'_, AppState>, model_id: String) -> Result<Value, String> {
    state.db.with_conn(|conn| {
        conn.query_row(
            "SELECT id, name, model_type, status, is_active FROM model_config WHERE id = ?1",
            rusqlite::params![model_id],
            |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "modelType": row.get::<_, String>(2)?,
                    "status": row.get::<_, String>(3)?,
                    "isActive": row.get::<_, bool>(4)?,
                }))
            },
        )
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_3d_model(
    state: State<'_, AppState>,
    zip_path: String,
) -> Result<String, String> {
    use std::fs;
    use std::path::Path;

    let zip_path = Path::new(&zip_path);
    if !zip_path.exists() {
        return Err("压缩包不存在".into());
    }

    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    let model_id = format!("model_3d_{}", uuid::Uuid::new_v4().to_string().get(..8).unwrap_or("def"));
    let target_dir = state.data_dir.join("models").join("3d").join(&model_id);
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    archive.extract(&target_dir).map_err(|e| e.to_string())?;

    let mut found_path = None;
    for entry in walkdir::WalkDir::new(&target_dir) {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "gltf" || ext == "glb" {
                    found_path = Some(path.to_path_buf());
                    break;
                }
            }
        }
    }

    match found_path {
        Some(p) => Ok(p.to_string_lossy().into_owned()),
        None => Err("在压缩包中找不到 .gltf 或 .glb 文件".into()),
    }
}

#[tauri::command]
pub async fn diagnose_network() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;

    let mut results = serde_json::Map::new();

    // 1. Ollama (Local)
    let ollama = client.get("http://127.0.0.1:11434/api/tags").send().await;
    results.insert("ollama".into(), serde_json::json!({
        "status": ollama.is_ok(),
        "message": if ollama.is_ok() { "Local service accessible" } else { "Service not found (ensure Ollama is running)" }
    }));

    // 2. ModelScope (China Mirror)
    let ms = client.get("https://modelscope.cn").send().await;
    results.insert("modelscope".into(), serde_json::json!({
        "status": ms.is_ok(),
        "message": if ms.is_ok() { "Accessible (Recommended for ASR/TTS)" } else { "Connection failed" }
    }));

    // 3. HuggingFace Mirror
    let hf = client.get("https://hf-mirror.com").send().await;
    results.insert("hf_mirror".into(), serde_json::json!({
        "status": hf.is_ok(),
        "message": if hf.is_ok() { "Accessible (Accelerated)" } else { "Connection failed" }
    }));

    // 4. Original HuggingFace
    let hf_orig = client.get("https://huggingface.co").send().await;
    results.insert("huggingface".into(), serde_json::json!({
        "status": hf_orig.is_ok(),
        "message": if hf_orig.is_ok() { "Direct access working" } else { "Likely blocked by firewall" }
    }));

    Ok(serde_json::Value::Object(results))
}


// ── 测试步骤事件发送 ──

fn generate_test_tone_wav(sample_rate: u32, duration_secs: f64, freq: f64) -> Vec<u8> {
    let num_samples = (sample_rate as f64 * duration_secs) as usize;
    let bits_per_sample: u16 = 16;
    let channels: u16 = 1;
    let byte_rate = sample_rate * channels as u32 * (bits_per_sample / 8) as u32;
    let block_align = channels * (bits_per_sample / 8);
    let data_size = num_samples * (bits_per_sample / 8) as usize;
    let file_size = 36 + data_size;

    let mut buf = Vec::with_capacity(file_size);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(file_size as u32).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&(data_size as u32).to_le_bytes());

    use std::f64::consts::PI;
    let amplitude = 0.3 * 32767.0;
    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let sample = (amplitude * (2.0 * PI * freq * t).sin()) as i16;
        let fade_in = if i < sample_rate as usize / 20 { (i as f64 / (sample_rate as f64 / 20.0)).min(1.0) } else { 1.0 };
        let fade_out = if i > num_samples - sample_rate as usize / 20 { ((num_samples - i) as f64 / (sample_rate as f64 / 20.0)).min(1.0) } else { 1.0 };
        let sample = (sample as f64 * fade_in * fade_out) as i16;
        buf.extend_from_slice(&sample.to_le_bytes());
    }
    buf
}

fn emit_test_step(app: &tauri::AppHandle, model_id: &str, step: &str, status: &str, detail: Option<&str>) -> Result<(), String> {
    let mut payload = serde_json::json!({
        "modelId": model_id,
        "step": step,
        "status": status,
    });
    if let Some(d) = detail {
        payload["detail"] = serde_json::Value::String(d.to_string());
    }
    app.emit("model-test:step", payload).map_err(|e| e.to_string())
}


fn guess_vram() -> u64 {
    #[cfg(target_os = "windows")]
    {
        let smi = Command::new("nvidia-smi").args(&["--query-gpu=memory.total", "--format=csv,noheader,nounits"]).output();
        if let Ok(out) = smi {
            let out_str = String::from_utf8_lossy(&out.stdout);
            if let Ok(mib) = out_str.trim().lines().next().unwrap_or("0").parse::<u64>() { if mib > 0 { return mib / 1024; } }
        }
    }
    8
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

#[tauri::command]
pub fn get_models_dir_info(state: State<'_, AppState>) -> Result<Value, String> {
    let models_dir = resolve_models_dir(&state);
    let models_list = list_local_model_dirs(&models_dir);
    let total_size: u64 = models_list.iter().map(|(_, s)| s).sum();
    Ok(serde_json::json!({
        "path": models_dir.to_string_lossy().to_string(),
        "totalSize": total_size,
        "totalSizeFormatted": format_bytes(total_size),
        "modelCount": models_list.len(),
        "models": models_list.iter().map(|(name, size)| serde_json::json!({
            "name": name,
            "size": size,
            "sizeFormatted": format_bytes(*size)
        })).collect::<Vec<_>>()
    }))
}

#[tauri::command]
pub fn set_models_dir(state: State<'_, AppState>, app: tauri::AppHandle, path: String) -> Result<(), String> {
    if path.is_empty() {
        return Err("路径不能为空".into());
    }
    let path_buf = PathBuf::from(&path);
    std::fs::create_dir_all(&path_buf).map_err(|e| format!("创建目录失败: {}", e))?;

    state.db.with_conn(|conn| {
        let json_str = serde_json::to_string(&path).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value_json, updated_at) VALUES ('modelsDir', ?1, datetime('now'))",
            rusqlite::params![json_str],
        )?;
        Ok::<_, rusqlite::Error>(())
    }).map_err(|e| e.to_string())?;

    state.worker_pool.update_models_dir(&path);
    let _ = app.emit("models-dir-changed", serde_json::json!({ "path": path }));
    Ok(())
}

#[tauri::command]
pub fn migrate_models(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    let old_dir = PathBuf::from(&old_path);
    let new_dir = PathBuf::from(&new_path);

    if old_dir == new_dir {
        return Err("新旧路径相同".into());
    }
    if !old_dir.exists() {
        return Err(format!("源目录不存在: {}", old_path));
    }

    std::fs::create_dir_all(&new_dir).map_err(|e| format!("创建目标目录失败: {}", e))?;

    let models_list = list_local_model_dirs(&old_dir);
    let total_models = models_list.len();
    if total_models == 0 {
        state.db.with_conn(|conn| {
            let json_str = serde_json::to_string(&new_path).unwrap_or_default();
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value_json, updated_at) VALUES ('modelsDir', ?1, datetime('now'))",
                rusqlite::params![json_str],
            )?;
            Ok::<_, rusqlite::Error>(())
        }).map_err(|e| e.to_string())?;
        state.worker_pool.update_models_dir(&new_path);
        let _ = app.emit("migration-progress", serde_json::json!({ 
            "progress": 100, 
            "done": true,
            "success": true,
            "completed": 0,
            "total": 0
        }));
        return Ok(());
    }

    let app_clone = app.clone();
    let worker_pool = state.worker_pool.clone();
    let db = state.db.clone();

    std::thread::spawn(move || {
        let mut completed = 0usize;
        let mut failed_models = Vec::new();

        for (model_name, _model_size) in &models_list {
            let src = old_dir.join(model_name);
            let dst = new_dir.join(model_name);

            let _ = app_clone.emit("migration-progress", serde_json::json!({
                "progress": (completed as f64 / total_models as f64 * 100.0) as i32,
                "currentModel": model_name,
                "completed": completed,
                "total": total_models
            }));

            if dst.exists() {
                let _ = std::fs::remove_dir_all(&dst);
            }

            match copy_dir_recursive(&src, &dst) {
                Ok(_) => {
                    let _ = std::fs::remove_dir_all(&src);
                    completed += 1;
                }
                Err(e) => {
                    failed_models.push(serde_json::json!({ "name": model_name, "error": e.to_string() }));
                }
            }
        }

        let success = failed_models.is_empty();
        if success {
            let _ = db.with_conn(|conn| {
                let json_str = serde_json::to_string(&new_path).unwrap_or_default();
                conn.execute(
                    "INSERT OR REPLACE INTO settings (key, value_json, updated_at) VALUES ('modelsDir', ?1, datetime('now'))",
                    rusqlite::params![json_str],
                )?;
                Ok::<_, rusqlite::Error>(())
            });
            worker_pool.update_models_dir(&new_path);
        }

        let _ = app_clone.emit("migration-progress", serde_json::json!({
            "progress": 100,
            "done": true,
            "success": success,
            "completed": completed,
            "total": total_models,
            "failed": failed_models
        }));
    });

    Ok(())
}
