mod commands;
mod context;
mod emotion;
mod memory;
mod model_bus;
mod orchestrator;
mod perception;
mod plugin;
mod proactive;
mod rag;
mod relationship;
mod skill;
mod state;
mod storage;
mod vector;
mod wechat;
mod worker;

use model_bus::ollama::OllamaProvider;
use model_bus::scheduler::ModelScheduler;
use state::AppState;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use storage::{repo, Database};
use tauri::{Emitter, Manager};
use tauri::path::BaseDirectory;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use vector::factory;

/// 移除 Windows 冗长路径前缀 `\\?\`，避免传递给子进程时解析失败
#[cfg(windows)]
fn normalize_path(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if s.starts_with(r"\\?\") {
        PathBuf::from(&s[4..])
    } else {
        p.to_path_buf()
    }
}

#[cfg(not(windows))]
fn normalize_path(p: &Path) -> PathBuf {
    p.to_path_buf()
}

/// 解析 workers 目录：优先资源目录（打包后），再尝试 dev 模式下的项目根目录
fn resolve_workers_dir(app: &tauri::AppHandle) -> PathBuf {
    let _ = app;
    crate::worker::pool::resolve_workers_dir()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            // 初始化日志
            let _ = env_logger::Builder::from_env(
                env_logger::Env::default().default_filter_or("info"),
            )
            .try_init();

            let data_dir: PathBuf = app
                .path()
                .app_data_dir()
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("data dir: {e}")))?
                .join("AiCompanion");
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(data_dir.join("personas"))?;
            std::fs::create_dir_all(data_dir.join("skills"))?;
            std::fs::create_dir_all(data_dir.join("knowledge"))?;
            std::fs::create_dir_all(data_dir.join("memory"))?;
            std::fs::create_dir_all(data_dir.join("media"))?;
            std::fs::create_dir_all(data_dir.join("models"))?;

            // 插件注册表
            let plugins_dir = data_dir.join("plugins");
            std::fs::create_dir_all(&plugins_dir)?;
            let mut plugin_registry = crate::plugin::PluginRegistry::new(plugins_dir);
            if let Err(e) = plugin_registry.load() {
                log::warn!("加载已安装插件失败: {e}");
            }
            let plugin_registry = std::sync::Arc::new(std::sync::RwLock::new(plugin_registry));

            // 数据库
            let db_path = data_dir.join("db.sqlite");
            let db = Arc::new(Database::open(&db_path).map_err(|e| {
                Box::<dyn std::error::Error>::from(format!("db open: {e}"))
            })?);

            // 种子数据
            repo::seed_personas(&db).map_err(|e| {
                Box::<dyn std::error::Error>::from(format!("seed: {e}"))
            })?;

            // 伴侣功能模块表初始化
            relationship::ensure_tables(&db).map_err(|e| {
                Box::<dyn std::error::Error>::from(format!("relationship tables: {e}"))
            })?;
            memory::core_memory::ensure_tables(&db).map_err(|e| {
                Box::<dyn std::error::Error>::from(format!("core_memory tables: {e}"))
            })?;

            // 记忆提取器
            let memory_extractor = Arc::new(memory::extractor::MemoryExtractor::new());

            // 解析模型目录（优先使用 settings.modelsDir，否则默认 data_dir/models）
            let models_dir = {
                let default_dir = data_dir.join("models");
                let custom_dir: Option<String> = db.with_conn(|conn| {
                    let dir: Option<String> = conn.query_row(
                        "SELECT value_json FROM settings WHERE key = 'modelsDir'",
                        [],
                        |row| row.get(0),
                    ).ok().and_then(|v: String| serde_json::from_str(&v).ok());
                    Ok::<_, rusqlite::Error>(dir)
                }).unwrap_or(None);
                match custom_dir {
                    Some(p) if !p.is_empty() => {
                        let path = PathBuf::from(&p);
                        std::fs::create_dir_all(&path).ok();
                        p
                    }
                    _ => default_dir.to_string_lossy().to_string(),
                }
            };

            // Worker 池
            let workers_dir = resolve_workers_dir(&app.handle()).to_string_lossy().to_string();
            log::info!("workers_dir resolved to: {}", workers_dir);
            log::info!("models_dir resolved to: {}", models_dir);
            let worker_pool = Arc::new(worker::pool::WorkerPool::new(&workers_dir, &models_dir));

            // 向量存储（sqlite-vec 默认）
            let vector_store = factory::create_vector_store(db.clone(), "sqlite_vec");

            // ModelBus 调度器
            let scheduler = Arc::new(ModelScheduler::new("gemma4:e4b"));

            // 注册 Ollama Provider（异步检查）
            let ollama = Arc::new(OllamaProvider::new("gemma4:e4b"));
            let ollama_for_sched = ollama.clone() as Arc<dyn model_bus::provider::ModelProvider + Send + Sync>;
            let sched_clone = scheduler.clone();
            let db_clone = db.clone();

            tauri::async_runtime::spawn(async move {
                let available = ollama.is_available().await;
                ollama.set_availability(available).await;
                sched_clone.register(ollama_for_sched).await;
                if available {
                    log::info!("Ollama available — LLM ready");
                } else {
                    log::warn!("Ollama not detected — fallback mock will be used");
                }
            });

            // 微信管理器
            let wechat_manager = Arc::new(wechat::WeChatManager::new());

            app.manage(AppState {
                db: db.clone(),
                data_dir,
                model_scheduler: scheduler,
                worker_pool,
                vector_store,
                wechat_manager: wechat_manager.clone(),
                plugin_registry,
                memory_extractor,
                active_downloads: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            });

            // 主动互动引擎
            let proactive = proactive::ProactiveEngine::new(app.handle().clone());
            proactive.start();

            // 自动恢复已登录的微信账号的 sync_loop（应用重启后)
            let wm = wechat_manager.clone();
            let db2 = db.clone();
            let app2 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let n = wm.start_all_online_accounts(db2, app2).await;
                log::info!("wechat 自动恢复完成: {n} 个账号");
            });

            // 桌面端：系统托盘 + 全局快捷键
            #[cfg(desktop)]
            {
                // 全局快捷键插件初始化
                app.handle().plugin(tauri_plugin_global_shortcut::Builder::new().build())?;

                // 注册托盘图标
                let tray_icon = tauri::image::Image::from_path(
                    app.path().resolve("icons/icon.png", BaseDirectory::Resource)?,
                )
                .unwrap_or_else(|_| app.default_window_icon().cloned().expect("default window icon"));

                // 托盘菜单
                let show_i = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
                let new_chat_i = MenuItem::with_id(app, "new-chat", "新建聊天", true, None::<&str>)?;
                let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_i, &new_chat_i, &quit_i])?;

                TrayIconBuilder::new()
                    .icon(tray_icon)
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "new-chat" => {
                            let _ = app.emit("tray-new-chat", ());
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } => {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                if window.is_visible().unwrap_or(false) {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                        _ => {}
                    })
                    .build(app)?;

                // 全局快捷键
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
                app.global_shortcut().on_shortcut("CommandOrControl+Shift+N", |app, _, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = app.emit("tray-new-chat", ());
                    }
                })?;
                app.global_shortcut().on_shortcut("CommandOrControl+Shift+T", |app, _, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = app.emit("toggle-theme", ());
                    }
                })?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // System
            commands::health_check,
            commands::get_app_info,
            // Chat / Session
            commands::list_sessions,
            commands::create_session,
            commands::delete_session,
            commands::get_messages,
            commands::send_message,
            // Persona
            commands::list_personas,
            commands::get_persona,
            commands::update_persona,
            commands::set_active_persona,
            commands::get_active_persona_id,
            // Workflow
            commands::list_workflows,
            commands::get_workflow,
            commands::create_workflow,
            commands::update_workflow,
            commands::delete_workflow,
            // Knowledge
            commands::list_knowledge_bases,
            commands::create_knowledge_base,
            commands::list_knowledge_docs,
            commands::import_document,
            commands::delete_doc,
            commands::search_knowledge,
            // Skill
            commands::list_skills,
            commands::toggle_skill,
            commands::approve_skill_permission,
            commands::list_tool_call_logs,
            commands::run_skill_manual,
            // Plugin
            commands::list_plugins,
            commands::get_plugin,
            commands::install_plugin,
            commands::uninstall_plugin,
            commands::enable_plugin,
            commands::configure_plugin,
            // Model
            commands::list_models,
            commands::get_gpu_info,
            commands::download_model,
            commands::cancel_download,
            commands::activate_model,
            commands::get_model_status,
            commands::check_ollama,
            commands::test_model,
            commands::delete_model,
            commands::import_3d_model,
            commands::diagnose_network,
            commands::get_models_dir_info,
            commands::set_models_dir,
            commands::migrate_models,
            // Voice
            commands::start_recording,
            commands::stop_recording,
            commands::stop_recording_audio,
            commands::save_voice_message,
            commands::play_voice_message,
            commands::get_voice_transcript,
            commands::cancel_recording,
            commands::synthesize_speech,
            commands::list_audio_devices,
            // Capture
            commands::capture_screen,
            commands::capture_camera,
            // Settings
            commands::load_settings,
            commands::save_settings,
            // Memory
            commands::list_memories,
            commands::delete_memory,
            // Cloud Provider
            commands::list_cloud_providers,
            commands::create_cloud_provider,
            commands::update_cloud_provider,
            commands::delete_cloud_provider,
            commands::verify_cloud_provider,
            commands::sync_cloud_models,
            // Cloud TTS Provider
            commands::list_cloud_tts_providers,
            commands::get_cloud_tts_provider,
            commands::create_cloud_tts_provider,
            commands::update_cloud_tts_provider,
            commands::delete_cloud_tts_provider,
            commands::verify_cloud_tts_provider,
            commands::check_cloud_tts_quota,
            commands::cloud_tts_synthesize,
            commands::cloud_tts_preview,
            // 微信 iLink 通道
            commands::list_wechat_accounts,
            commands::get_wechat_account,
            commands::wechat_request_qrcode,
            commands::wechat_poll_login,
            commands::wechat_logout,
            commands::wechat_start_sync,
            commands::list_wechat_sessions,
            commands::list_wechat_messages,
            commands::mark_wechat_session_read,
            commands::send_wechat_text,
            commands::set_wechat_persona,
            commands::get_wechat_persona,
            // Companion / Relationship
            commands::get_relationship,
            commands::get_all_relationships,
            commands::update_core_memory,
            commands::set_nickname,
            commands::reset_relationship,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
