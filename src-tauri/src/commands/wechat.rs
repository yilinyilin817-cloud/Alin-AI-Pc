use crate::model_bus::ilink::{ILinkClient, QrCodeStatus};
use crate::state::AppState;
use crate::storage::models::{
    WeChatAccount, WeChatLoginStatus, WeChatMessage, WeChatQrCode, WeChatSession,
};
use crate::storage::{repo, Database};
use crate::wechat::sync::{map_qrcode_status_str, WeChatAccountEvent};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

fn default_account_id() -> String {
    "default".to_string()
}

fn map_status(s: &QrCodeStatus) -> &'static str {
    map_qrcode_status_str(s)
}

#[derive(Debug, Serialize)]
pub struct WeChatAccountView {
    #[serde(flatten)]
    pub account: WeChatAccount,
    pub qrcode_url: Option<String>,
    pub qrcode_status: Option<String>,
}

fn load_account_view(db: &Database, account_id: &str) -> Result<WeChatAccountView, String> {
    let acc = repo::ensure_wechat_account(db, account_id).map_err(|e| e.to_string())?;
    db.with_conn(|conn| {
        let mut stmt = conn
            .prepare("SELECT qrcode_url, qrcode_status FROM wechat_account WHERE id = ?1")?;
        let mut rows = stmt.query_map(rusqlite::params![account_id], |row| {
            Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?))
        })?;
        let (qrcode_url, qrcode_status) = rows.next().map(|r| r.unwrap_or((None, None))).unwrap_or((None, None));
        Ok(WeChatAccountView {
            account: acc,
            qrcode_url,
            qrcode_status,
        })
    })
    .map_err(|e| e.to_string())
}

/// 列出所有微信账号
#[tauri::command]
pub async fn list_wechat_accounts(
    state: State<'_, AppState>,
) -> Result<Vec<WeChatAccountView>, String> {
    let accounts = state
        .db
        .with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id FROM wechat_account ORDER BY created_at",
            )?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<Result<Vec<_>, _>>()
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for id in accounts {
        out.push(load_account_view(&state.db, &id).map_err(|e| e.to_string())?);
    }
    Ok(out)
}

/// 获取或创建默认微信账号
#[tauri::command]
pub async fn get_wechat_account(
    state: State<'_, AppState>,
    account_id: Option<String>,
) -> Result<WeChatAccountView, String> {
    let id = account_id.unwrap_or_else(default_account_id);
    load_account_view(&state.db, &id)
}

/// 申请登录二维码
#[tauri::command]
pub async fn wechat_request_qrcode(
    state: State<'_, AppState>,
    app: AppHandle,
    account_id: Option<String>,
) -> Result<WeChatQrCode, String> {
    let id = account_id.unwrap_or_else(default_account_id);
    let _ = repo::ensure_wechat_account(&state.db, &id).map_err(|e| e.to_string())?;

    let client = ILinkClient::new(None);
    let info = client.get_bot_qrcode().await.map_err(|e| e.to_string())?;

    repo::update_wechat_account_qrcode(&state.db, &id, &info.qrcode_key, &info.qrcode_url)
        .map_err(|e| e.to_string())?;

    let _ = app.emit(
        "wechat-account",
        WeChatAccountEvent {
            account_id: id.clone(),
            status: "logging_in".into(),
            nickname: None,
            avatar_url: None,
            last_error: None,
        },
    );

    Ok(WeChatQrCode {
        account_id: id,
        qrcode_url: info.qrcode_url,
        qrcode_key: info.qrcode_key,
        expires_in: info.expires_in,
    })
}

/// 轮询登录状态
#[tauri::command]
pub async fn wechat_poll_login(
    state: State<'_, AppState>,
    app: AppHandle,
    account_id: Option<String>,
) -> Result<WeChatLoginStatus, String> {
    let id = account_id.unwrap_or_else(default_account_id);
    let _acc = repo::ensure_wechat_account(&state.db, &id).map_err(|e| e.to_string())?;

    // 读出 qrcode_key
    let qrcode_key: String = state
        .db
        .with_conn(|conn| {
            let key: String = conn
                .query_row(
                    "SELECT qrcode_key FROM wechat_account WHERE id = ?1",
                    rusqlite::params![&id],
                    |row| row.get(0),
                )
                .unwrap_or_default();
            Ok::<String, rusqlite::Error>(key)
        })
        .map_err(|e| e.to_string())?;

    if qrcode_key.is_empty() {
        return Ok(WeChatLoginStatus {
            status: "error".into(),
            account_id: id,
            bot_token: None,
            user_id: None,
            nickname: None,
            avatar_url: None,
            message: Some("请先申请二维码".into()),
        });
    }

    let client = ILinkClient::new(None);
    let result = client
        .poll_qrcode_status(&qrcode_key)
        .await
        .map_err(|e| e.to_string())?;

    let mapped = map_status(&result.status).to_string();
    let _ = repo::update_wechat_account_qrcode_status(&state.db, &id, &mapped);

    if matches!(result.status, QrCodeStatus::Confirmed) {
        if let Some(login) = &result.login {
            repo::update_wechat_account_token(
                &state.db,
                &id,
                &login.user_id,
                login.nickname.as_deref(),
                login.avatar_url.as_deref(),
                &login.bot_token,
            )
            .map_err(|e| e.to_string())?;

            // 启动同步任务
            let _ = state
                .wechat_manager
                .ensure_started(&id, state.db.clone(), app.clone())
                .await;

            let _ = app.emit(
                "wechat-account",
                WeChatAccountEvent {
                    account_id: id.clone(),
                    status: "online".into(),
                    nickname: login.nickname.clone(),
                    avatar_url: login.avatar_url.clone(),
                    last_error: None,
                },
            );

            return Ok(WeChatLoginStatus {
                status: "confirmed".into(),
                account_id: id,
                bot_token: Some(login.bot_token.clone()),
                user_id: Some(login.user_id.clone()),
                nickname: login.nickname.clone(),
                avatar_url: login.avatar_url.clone(),
                message: Some("登录成功".into()),
            });
        }
    }

    if matches!(result.status, QrCodeStatus::Expired | QrCodeStatus::Canceled) {
        let _ = repo::update_wechat_account_offline(&state.db, &id);
        let _ = app.emit(
            "wechat-account",
            WeChatAccountEvent {
                account_id: id.clone(),
                status: "offline".into(),
                nickname: None,
                avatar_url: None,
                last_error: Some("二维码已过期".into()),
            },
        );
    }

    Ok(WeChatLoginStatus {
        status: mapped,
        account_id: id,
        bot_token: None,
        user_id: None,
        nickname: None,
        avatar_url: None,
        message: result.message,
    })
}

/// 主动登出
#[tauri::command]
pub async fn wechat_logout(
    state: State<'_, AppState>,
    app: AppHandle,
    account_id: Option<String>,
) -> Result<(), String> {
    let id = account_id.unwrap_or_else(default_account_id);
    state.wechat_manager.stop(&id).await;
    repo::update_wechat_account_offline(&state.db, &id).map_err(|e| e.to_string())?;
    let _ = app.emit(
        "wechat-account",
        WeChatAccountEvent {
            account_id: id.clone(),
            status: "offline".into(),
            nickname: None,
            avatar_url: None,
            last_error: None,
        },
    );
    Ok(())
}

/// 列出该账号下的会话
#[tauri::command]
pub async fn list_wechat_sessions(
    state: State<'_, AppState>,
    account_id: Option<String>,
) -> Result<Vec<WeChatSession>, String> {
    let id = account_id.unwrap_or_else(default_account_id);
    repo::list_wechat_sessions(&state.db, &id).map_err(|e| e.to_string())
}

/// 列出某会话的消息
#[tauri::command]
pub async fn list_wechat_messages(
    state: State<'_, AppState>,
    session_id: String,
    limit: Option<i64>,
) -> Result<Vec<WeChatMessage>, String> {
    repo::list_wechat_messages(&state.db, &session_id, limit.unwrap_or(100))
        .map_err(|e| e.to_string())
}

/// 标记会话已读
#[tauri::command]
pub async fn mark_wechat_session_read(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    repo::mark_wechat_session_read(&state.db, &session_id).map_err(|e| e.to_string())
}

/// 发送文本消息
#[tauri::command]
pub async fn send_wechat_text(
    state: State<'_, AppState>,
    app: AppHandle,
    session_id: String,
    text: String,
) -> Result<WeChatMessage, String> {
    if text.trim().is_empty() {
        return Err("消息内容不能为空".into());
    }
    log::info!("wechat send_wechat_text session={session_id} text={}", text.chars().take(80).collect::<String>());

    let session = repo::get_wechat_session(&state.db, &session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "会话不存在".to_string())?;

    // 找一条最近的 inbound 消息获取 context_token（24h 窗口内有效）
    let recent = repo::list_wechat_messages(&state.db, &session_id, 50).unwrap_or_default();
    let context_token = recent
        .iter()
        .find(|m| m.direction == "inbound" && m.context_token.is_some())
        .and_then(|m| m.context_token.clone())
        .ok_or_else(|| {
            "无法发送：未找到有效的 context_token（需等待用户先发送消息，Bot 才能在 24h 窗口内回复）".to_string()
        })?;

    log::debug!("wechat send: peer={} ctx_token_len={}", session.peer_id, context_token.len());

    let bot_token = repo::get_wechat_bot_token(&state.db, &session.account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "账号未登录".to_string())?;

    let pending_id = format!("wmsg_{}", Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();
    let pending = WeChatMessage {
        id: pending_id.clone(),
        account_id: session.account_id.clone(),
        session_id: session_id.clone(),
        remote_msg_id: None,
        direction: "outbound".into(),
        msg_type: "text".into(),
        content: Some(text.clone()),
        media_url: None,
        media_local_path: None,
        sender_id: None,
        sender_name: None,
        context_token: Some(context_token.clone()),
        status: "pending".into(),
        error: None,
        created_at: now.clone(),
    };
    repo::insert_wechat_message(&state.db, &pending).map_err(|e| e.to_string())?;

    // 落库后立即 emit pending
    let _ = app.emit(
        "wechat-message",
        crate::wechat::sync::WeChatMessageEvent {
            account_id: session.account_id.clone(),
            session_id: session_id.clone(),
            message: pending.clone(),
            is_new_session: false,
        },
    );

    let client = ILinkClient::new(Some(&bot_token));
    log::info!(
        "wechat → ilink sendmessage: peer={} ctx={} text_len={}",
        session.peer_id,
        &context_token.chars().take(20).collect::<String>(),
        text.len()
    );
    // message_state: 2 = 完成(FINISH), 必须用 2 才会真正投递消息;
    // 1 = 生成中 仅用于流式 typing 指示, 单独用 1 会导致消息静默不投递
    match client
        .send_message(&session.peer_id, &context_token, &text, 2)
        .await
    {
        Ok(remote_id) => {
            log::info!("wechat send ok: remote_id={remote_id}");
            let _ = repo::update_wechat_message_status(&state.db, &pending_id, "sent", None);
            // 回填 remote_msg_id
            state
                .db
                .with_conn(|conn| {
                    conn.execute(
                        "UPDATE wechat_message SET remote_msg_id = ?1 WHERE id = ?2",
                        rusqlite::params![remote_id, pending_id],
                    )?;
                    Ok(())
                })
                .map_err(|e| e.to_string())?;
        }
        Err(e) => {
            log::warn!("wechat send fail: {e}");
            let _ = repo::update_wechat_message_status(
                &state.db,
                &pending_id,
                "failed",
                Some(&e),
            );
            let _ = app.emit(
                "wechat-message",
                crate::wechat::sync::WeChatMessageEvent {
                    account_id: session.account_id.clone(),
                    session_id: session_id.clone(),
                    message: WeChatMessage {
                        id: pending_id.clone(),
                        status: "failed".into(),
                        error: Some(e.clone()),
                        ..pending.clone()
                    },
                    is_new_session: false,
                },
            );
            return Err(e);
        }
    }

    let final_msg = repo::list_wechat_messages(&state.db, &session_id, 1)
        .map_err(|e| e.to_string())?
        .pop()
        .unwrap_or(pending);

    // 更新会话预览
    let _ = repo::upsert_wechat_session(
        &state.db,
        &final_msg.account_id,
        &session.peer_id,
        &session.peer_type,
        session.peer_name.as_deref(),
        session.peer_avatar.as_deref(),
        Some(&text),
        Some(&final_msg.created_at),
    );

    let _ = app.emit(
        "wechat-message",
        crate::wechat::sync::WeChatMessageEvent {
            account_id: final_msg.account_id.clone(),
            session_id: session_id.clone(),
            message: final_msg.clone(),
            is_new_session: false,
        },
    );

    Ok(final_msg)
}

/// 启动/恢复同步（用于已登录但同步未启动的场景）
#[tauri::command]
pub async fn wechat_start_sync(
    state: State<'_, AppState>,
    app: AppHandle,
    account_id: Option<String>,
) -> Result<(), String> {
    let id = account_id.unwrap_or_else(default_account_id);
    let _ = state
        .wechat_manager
        .ensure_started(&id, state.db.clone(), app.clone())
        .await;
    Ok(())
}

/// 绑定角色到微信账号（设置后自动回复使用该角色的 system_prompt）
#[tauri::command]
pub async fn set_wechat_persona(
    state: State<'_, AppState>,
    account_id: Option<String>,
    persona_id: Option<String>,
) -> Result<(), String> {
    let id = account_id.unwrap_or_else(default_account_id);
    repo::update_wechat_persona(&state.db, &id, persona_id.as_deref())
        .map_err(|e| e.to_string())?;
    log::info!("wechat[{id}] 角色绑定: {:?}", persona_id);
    Ok(())
}

/// 查询微信账号绑定的角色 ID
#[tauri::command]
pub async fn get_wechat_persona(
    state: State<'_, AppState>,
    account_id: Option<String>,
) -> Result<Option<String>, String> {
    let id = account_id.unwrap_or_else(default_account_id);
    repo::get_wechat_persona(&state.db, &id).map_err(|e| e.to_string())
}
