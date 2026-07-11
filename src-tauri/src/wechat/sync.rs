//! 微信 iLink 同步任务
//!
//! 每个登录的 wechat_account 持有一个 SyncHandle：
//! - 长轮询 getUpdates
//! - 把消息落库 + 通过 Tauri Event 推送到前端
//! - 网络异常时退避重连
//! - 收到消息后通过 LLM 通道（可选）生成 AI 回复

use crate::model_bus::ilink::{ILinkClient, IncomingMessage, MsgType, QrCodeStatus};
use crate::storage::models::{WeChatMessage};
use crate::storage::{repo, Database};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeChatMessageEvent {
    pub account_id: String,
    pub session_id: String,
    pub message: WeChatMessage,
    pub is_new_session: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeChatAccountEvent {
    pub account_id: String,
    pub status: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub last_error: Option<String>,
}

pub struct WeChatSync {
    pub account_id: String,
    pub db: Arc<Database>,
    pub app: AppHandle,
    pub running: Arc<Mutex<bool>>,
    pub task: Mutex<Option<JoinHandle<()>>>,
}

impl WeChatSync {
    pub fn new(account_id: String, db: Arc<Database>, app: AppHandle) -> Self {
        Self {
            account_id,
            db,
            app,
            running: Arc::new(Mutex::new(false)),
            task: Mutex::new(None),
        }
    }

    pub async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
        let mut task = self.task.lock().await;
        if let Some(t) = task.take() {
            t.abort();
        }
    }

    pub async fn start(&self) {
        {
            let mut running = self.running.lock().await;
            if *running {
                return;
            }
            *running = true;
        }

        let account_id = self.account_id.clone();
        let db = self.db.clone();
        let app = self.app.clone();
        let running = self.running.clone();

        let handle = tokio::spawn(async move {
            sync_loop(account_id, db, app, running).await;
        });

        *self.task.lock().await = Some(handle);
    }
}

async fn sync_loop(
    account_id: String,
    db: Arc<Database>,
    app: AppHandle,
    running: Arc<Mutex<bool>>,
) {
    let mut backoff_secs: u64 = 2;

    while *running.lock().await {
        // 读取 bot_token / buf
        let (Some(bot_token), buf) = (
            repo::get_wechat_bot_token(&db, &account_id).unwrap_or(None),
            repo::get_wechat_buf(&db, &account_id).unwrap_or_else(|_| "".into()),
        ) else {
            log::info!("wechat[{account_id}] 未登录，停止同步");
            break;
        };

        let client = ILinkClient::new(Some(&bot_token));
        log::trace!("wechat[{account_id}] getUpdates buf={buf}");

        match client.get_updates(&buf, 30).await {
            Ok(result) => {
                backoff_secs = 2;
                // 持久化新游标
                let _ = repo::update_wechat_account_buf(&db, &account_id, &result.next_buf);

                for msg in &result.messages {
                    match persist_inbound(&db, &account_id, msg).await {
                        Ok(persisted) => {
                            log::info!(
                                "wechat[{account_id}] 收到 {} from {} ({:?}): {}",
                                persisted.message.msg_type,
                                persisted.message.sender_id.as_deref().unwrap_or("?"),
                                persisted.message.sender_name,
                                persisted.message.content.as_deref().unwrap_or("(no text)").chars().take(60).collect::<String>()
                            );
                            // 推送消息事件给前端
                            let _ = app.emit(
                                "wechat-message",
                                WeChatMessageEvent {
                                    account_id: account_id.clone(),
                                    session_id: persisted.session_id.clone(),
                                    message: persisted.message.clone(),
                                    is_new_session: persisted.is_new_session,
                                },
                            );
                            // 新会话时也推 account 事件（用于左侧列表刷新）
                            if persisted.is_new_session {
                                let _ = app.emit(
                                    "wechat-account",
                                    WeChatAccountEvent {
                                        account_id: account_id.clone(),
                                        status: "online".into(),
                                        nickname: None,
                                        avatar_url: None,
                                        last_error: None,
                                    },
                                );
                            }

                            // 自动回复（仅文本消息）
                            if persisted.message.msg_type == "text" {
                                let db2 = db.clone();
                                let app2 = app.clone();
                                let account_id2 = account_id.clone();
                                let bot_token2 = bot_token.clone();
                                tokio::spawn(async move {
                                    try_auto_reply(db2, app2, account_id2, bot_token2, persisted).await;
                                });
                            }
                        }
                        Err(e) => {
                            log::warn!("wechat[{account_id}] 入站消息落库失败: {e}");
                        }
                    }
                }

                // 拉下一轮
                if !result.has_more && result.messages.is_empty() {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
            Err(e) => {
                // 会话过期：errcode -14（兼容 wechatbot.dev 协议 + iLink 错误码）
                let is_session_timeout = e.contains("-14") || e.contains("session timeout");
                log::warn!("wechat[{account_id}] getUpdates 失败: {e}, 退避 {backoff_secs}s");

                if is_session_timeout {
                    // 清掉 token，让前端跳回登录
                    let _ = repo::update_wechat_account_offline(&db, &account_id);
                    let _ = app.emit(
                        "wechat-account",
                        WeChatAccountEvent {
                            account_id: account_id.clone(),
                            status: "offline".into(),
                            nickname: None,
                            avatar_url: None,
                            last_error: Some("会话已过期，请重新扫码登录".into()),
                        },
                    );
                    log::info!("wechat[{account_id}] session timeout (-14)，停掉同步");
                    break;
                }

                let _ = repo::update_wechat_account_error(&db, &account_id, &e);
                let _ = app.emit(
                    "wechat-account",
                    WeChatAccountEvent {
                        account_id: account_id.clone(),
                        status: "error".into(),
                        nickname: None,
                        avatar_url: None,
                        last_error: Some(e),
                    },
                );
                tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(60);
                continue;
            }
        }
    }
}

/// persist_inbound 落库结果
pub struct PersistedInbound {
    pub session_id: String,
    pub message: WeChatMessage,
    pub is_new_session: bool,
}

async fn persist_inbound(
    db: &Arc<Database>,
    account_id: &str,
    msg: &IncomingMessage,
) -> Result<PersistedInbound, String> {
    // 0) 判断会话是否已存在（用于 is_new_session 标记）
    let peer_id = msg.peer_id.clone();
    let peer_type = if msg.peer_type == "room" { "room" } else { "user" };
    let existed: bool = db
        .with_conn(|conn| {
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM wechat_session WHERE account_id = ?1 AND peer_id = ?2",
                    rusqlite::params![account_id, peer_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            Ok(n > 0)
        })
        .unwrap_or(false);

    // 1) upsert 会话
    let preview: String = match msg.msg_type {
        MsgType::Text => msg.content.clone().unwrap_or_default(),
        MsgType::Image => "[图片]".into(),
        MsgType::Voice => "[语音]".into(),
        MsgType::Video => "[视频]".into(),
        MsgType::File => "[文件]".into(),
        MsgType::System => "[系统消息]".into(),
        MsgType::Unknown => msg.content.clone().unwrap_or_default(),
    };
    let session = repo::upsert_wechat_session(
        db,
        account_id,
        &peer_id,
        peer_type,
        msg.peer_name.as_deref(),
        None,
        Some(&preview),
        Some(&msg.received_at),
    )
    .map_err(|e| e.to_string())?;

    // 入站消息递增未读计数
    let _ = repo::increment_wechat_session_unread(db, account_id, &peer_id);

    // 2) 落库
    let we_msg = WeChatMessage {
        id: format!("wmsg_{}", Uuid::new_v4()),
        account_id: account_id.to_string(),
        session_id: session.id.clone(),
        remote_msg_id: Some(msg.remote_msg_id.clone()),
        direction: "inbound".into(),
        msg_type: msg_type_str(&msg.msg_type).to_string(),
        content: msg.content.clone(),
        media_url: msg.media_url.clone(),
        media_local_path: None,
        sender_id: msg.sender_id.clone(),
        sender_name: msg.sender_name.clone(),
        context_token: msg.context_token.clone(),
        status: "sent".into(),
        error: None,
        created_at: msg.received_at.clone(),
    };
    repo::insert_wechat_message(db, &we_msg).map_err(|e| e.to_string())?;

    Ok(PersistedInbound {
        session_id: session.id,
        message: we_msg,
        is_new_session: !existed,
    })
}

fn msg_type_str(t: &MsgType) -> &'static str {
    match t {
        MsgType::Text => "text",
        MsgType::Image => "image",
        MsgType::Voice => "voice",
        MsgType::Video => "video",
        MsgType::File => "file",
        MsgType::System => "system",
        MsgType::Unknown => "unknown",
    }
}

pub fn map_qrcode_status_str(s: &QrCodeStatus) -> &'static str {
    match s {
        QrCodeStatus::New => "pending",
        QrCodeStatus::Scanning => "scanned",
        QrCodeStatus::Confirmed => "confirmed",
        QrCodeStatus::Expired => "expired",
        QrCodeStatus::Canceled => "expired",
    }
}

/// 自动回复：收到文本消息后调用 LLM 生成回复并发送
async fn try_auto_reply(
    db: Arc<Database>,
    app: AppHandle,
    account_id: String,
    bot_token: String,
    persisted: PersistedInbound,
) {
    // 1. 检查是否开启自动回复
    let auto_reply: bool = db
        .with_conn(|conn| {
            let val: String = conn
                .query_row(
                    "SELECT value_json FROM settings WHERE key = 'wechat_auto_reply'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or_default();
            Ok::<bool, rusqlite::Error>(val == "\"true\"" || val == "true")
        })
        .unwrap_or(false);

    if !auto_reply {
        return;
    }

    // 2. 获取绑定的角色
    let persona_id = match repo::get_wechat_persona(&db, &account_id) {
        Ok(Some(id)) => id,
        _ => {
            log::debug!("wechat[{account_id}] 自动回复已启用但未绑定角色，跳过");
            return;
        }
    };

    let persona_row = match repo::get_persona(&db, &persona_id) {
        Ok(Some(p)) => p,
        _ => {
            log::warn!("wechat[{account_id}] 角色 {persona_id} 不存在");
            return;
        }
    };
    let persona = persona_row.definition;

    // 3. 构建上下文（该 peer 最近 10 条消息）
    let peer_context: Vec<(String, String)> = {
        let msgs = repo::list_wechat_messages(&db, &persisted.session_id, 10).unwrap_or_default();
        msgs.into_iter()
            .rev()
            .filter_map(|m| {
                let role = if m.direction == "inbound" { "user" } else { "assistant" };
                m.content.map(|c| (role.to_string(), c))
            })
            .collect()
    };

    let new_text = persisted.message.content.clone().unwrap_or_default();
    if new_text.trim().is_empty() {
        return;
    }

    let peer_id = persisted.message.sender_id.clone().unwrap_or_default();
    let context_token = persisted.message.context_token.clone().unwrap_or_default();
    if context_token.is_empty() {
        log::warn!("wechat[{account_id}] 缺少 context_token，无法发送回复");
        return;
    }

    // 4. 获取 ilink_user_id 用于打字状态
    let ilink_user_id: String = db
        .with_conn(|conn| {
            Ok(conn
                .query_row(
                    "SELECT user_id FROM wechat_account WHERE id = ?1",
                    rusqlite::params![account_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .unwrap_or(None)
                .unwrap_or_default())
        })
        .unwrap_or_default();

    // 5. 发送"正在输入"状态
    let client = ILinkClient::new(Some(&bot_token));
    if !ilink_user_id.is_empty() {
        if let Ok(ticket) = client.get_config(&ilink_user_id, Some(&context_token)).await {
            let _ = client.send_typing(&ilink_user_id, &ticket, 1).await;
            log::debug!("wechat[{account_id}] 发送打字状态 → {peer_id}");
        }
    }

    // 6. 读取微信配置
    let (segmented_enabled, segment_delay_ms, action_mode) = persona.wechat.as_ref().map(|w| {
        let enabled = w.get("enableSegmentedReply").and_then(|v| v.as_bool()).unwrap_or(false);
        let delay = w.get("segmentDelay").and_then(|v| v.as_u64()).unwrap_or(800);
        let mode = w.get("actionDescriptionMode").and_then(|v| v.as_str()).unwrap_or("inline");
        (enabled, delay, mode)
    }).unwrap_or((false, 800, "inline"));

    // 调用 LLM 流式生成（仅累积文本，不再发送 state=1 流式更新——完全取消流式回复）
    // 微信中流式打字效果通过 state=1 发送会与分段发送产生竞态，
    // 且微信对快速更新的消息支持不佳，改为等待完整回复后直接发送，更稳定可靠
    let scheduler = app.state::<crate::state::AppState>().model_scheduler.clone();
    let mut accumulated = String::new();

    let reply_text = match crate::orchestrator::pipeline::reply_via_llm_stream(
        &scheduler,
        &persona,
        &peer_context,
        &new_text,
        |chunk| {
            accumulated.push_str(chunk);
        },
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            log::warn!("wechat[{account_id}] LLM 回复失败: {e}");
            return;
        }
    };

    // 7. 取消打字状态
    if !ilink_user_id.is_empty() {
        if let Ok(ticket) = client.get_config(&ilink_user_id, Some(&context_token)).await {
            let _ = client.send_typing(&ilink_user_id, &ticket, 2).await;
        }
    }

    if reply_text.trim().is_empty() {
        return;
    }

    // 8. 先预处理文本（根据动作描述模式），再分段
    let processed_text = match action_mode {
        "remove" => remove_action_descriptions(&reply_text),
        _ => reply_text.clone(),
    };

    // 9. 智能分段
    let segments = if segmented_enabled {
        smart_split_segments(&processed_text, action_mode)
    } else {
        // 不分段模式：remove 模式已经预处理过，inline/separate 都直接整段发送
        // separate 模式但未启用多段式回复时，退化为 inline（不分段就无法独立发送）
        vec![processed_text.clone()]
    };

    log::info!(
        "wechat[{account_id}] AI回复 → {peer_id}: {} 字符，分 {} 段发送 (segmented={}, action={})",
        reply_text.len(),
        segments.len(),
        segmented_enabled,
        action_mode
    );

    for (idx, seg_text) in segments.iter().enumerate() {
        let seg_text = seg_text.trim();
        if seg_text.is_empty() {
            continue;
        }

        let out_id = format!("wmsg_{}", Uuid::new_v4());
        let now = chrono::Utc::now().to_rfc3339();

        // 落库 pending
        let out_msg = WeChatMessage {
            id: out_id.clone(),
            account_id: account_id.clone(),
            session_id: persisted.session_id.clone(),
            remote_msg_id: None,
            direction: "outbound".into(),
            msg_type: "text".into(),
            content: Some(seg_text.to_string()),
            media_url: None,
            media_local_path: None,
            sender_id: None,
            sender_name: None,
            context_token: Some(context_token.clone()),
            status: "pending".into(),
            error: None,
            created_at: now.clone(),
        };
        let _ = repo::insert_wechat_message(&db, &out_msg);

        // 第一段前加一个初始思考延迟（模拟LLM思考完到开始打字发送的间隔）
        // 段间使用随机延迟：基础延迟的 60% ~ 150% 随机波动，更像真人
        if idx == 0 {
            let initial_delay = random_delay_ms(500, 1200);
            tokio::time::sleep(Duration::from_millis(initial_delay)).await;
        }

        let send_result = client.send_message(&peer_id, &context_token, seg_text, 2).await;

        match send_result {
            Ok(remote_id) => {
                let _ = repo::update_wechat_message_status(&db, &out_id, "sent", None);
                let _ = app.emit(
                    "wechat-message",
                    WeChatMessageEvent {
                        account_id: account_id.clone(),
                        session_id: persisted.session_id.clone(),
                        message: WeChatMessage {
                            remote_msg_id: Some(remote_id),
                            status: "sent".into(),
                            ..out_msg
                        },
                        is_new_session: false,
                    },
                );
            }
            Err(e) => {
                let _ = repo::update_wechat_message_status(&db, &out_id, "failed", Some(&e));
                log::warn!("wechat[{account_id}] 第 {} 段发送失败: {e}", idx + 1);
            }
        }

        // 段间随机延迟（最后一段后无需等待）
        if idx < segments.len() - 1 && segment_delay_ms > 0 {
            let delay = random_delay_between(segment_delay_ms, 600, 1500);
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }
    }
}

/// 生成 [min, max] 范围内的随机延迟毫秒
fn random_delay_ms(min: u64, max: u64) -> u64 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    rng.gen_range(min..=max)
}

/// 以 base_ms 为基准，按百分比 [min_pct, max_pct] 波动生成随机延迟
/// 例如 base=1000, min=600, max=1500 → 600~1500ms 随机
fn random_delay_between(base_ms: u64, min_pct: u64, max_pct: u64) -> u64 {
    use rand::Rng;
    let min_ms = base_ms * min_pct / 100;
    let max_ms = base_ms * max_pct / 100;
    let mut rng = rand::thread_rng();
    rng.gen_range(min_ms..=max_ms)
}

/// 移除括号动作描写：删除中文括号（…）和英文括号(…)中的内容（包括括号本身）
fn remove_action_descriptions(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '（' || ch == '(' {
            let close = if ch == '（' { '）' } else { ')' };
            // 跳过直到匹配的闭括号
            i += 1;
            let mut depth = 1;
            while i < chars.len() && depth > 0 {
                if chars[i] == ch {
                    depth += 1;
                } else if chars[i] == close {
                    depth -= 1;
                }
                i += 1;
            }
        } else {
            result.push(ch);
            i += 1;
        }
    }
    // 清理移除后可能留下的多余空白/换行
    result.split('\n')
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 判断字符是否是句末标点（句子结束标志）
/// 省略号…不单独作为句末，需要后面跟其他标点或空格才算结束
fn is_sentence_terminator(ch: char, next_ch: Option<char>) -> bool {
    match ch {
        '。' | '！' | '？' | '!' | '?' => true,
        '~' | '～' => true,
        '…' => {
            // 省略号：只有后面跟着句末标点/空白/结束，或者后面没有内容时才算句末
            // 否则是句中停顿（如"有没有……有什么"）
            match next_ch {
                None => true,
                Some(nc) => nc.is_whitespace() || matches!(nc, '。' | '！' | '？' | '!' | '?' | '"' | '"' | '"' | '」' | '）' | ')'),
            }
        }
        _ => false,
    }
}

/// 判断字符是否是表情符号（emoji 通常在句末，分段时可在emoji后断句）
fn is_emoji(ch: char) -> bool {
    let cp = ch as u32;
    // 常见emoji范围
    (0x1F600..=0x1F64F).contains(&cp)   // 表情
        || (0x1F300..=0x1F5FF).contains(&cp)  // 符号&图
        || (0x1F680..=0x1F6FF).contains(&cp)  // 交通
        || (0x1F900..=0x1F9FF).contains(&cp)  // 补充符号
        || (0x2600..=0x26FF).contains(&cp)    // 杂项符号
        || (0x2700..=0x27BF).contains(&cp)    // 装饰符号
        || (0x1F400..=0x1F4FF).contains(&cp)  // 动物植物
        || (0x1F480..=0x1F4FF).contains(&cp)  // 爱心等
        || (0x2764..=0x2764).contains(&cp)    // ❤
        || (0x1F493..=0x1F49F).contains(&cp)  // 💓-💟
}

/// 智能分段：将AI回复文本按自然语言节奏分割成多条微信消息
/// 模拟真人逐条发送的节奏，每段约30-80个汉字，更符合微信聊天习惯
/// action_mode:
/// - "separate": 括号动作描写（（…）/(…)）作为独立段
/// - "inline"  : 括号内容保留在原文中，不单独分段
/// - "remove"  : 已在预处理中移除，括号不会出现在此函数输入中
fn smart_split_segments(text: &str, action_mode: &str) -> Vec<String> {
    let separate_actions = action_mode == "separate";
    let mut segments: Vec<String> = Vec::new();
    let mut buf = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    // 分段长度参数（以字符数计，含标点表情）
    const MIN_SEG_LEN: usize = 20;   // 小于这个长度不断句（除非到了段落分隔/括号）
    const TARGET_SEG_LEN: usize = 50; // 目标段长度，遇到句末标点即可分段
    const MAX_SEG_LEN: usize = 120;   // 硬上限，超过强制在逗号处截断

    while i < len {
        let ch = chars[i];
        let next_ch = if i + 1 < len { Some(chars[i + 1]) } else { None };

        // 处理括号动作描写
        if separate_actions && (ch == '（' || ch == '(') {
            let close = if ch == '（' { '）' } else { ')' };
            let trimmed = buf.trim().to_string();
            if !trimmed.is_empty() {
                segments.push(trimmed);
                buf.clear();
            }
            buf.push(ch);
            i += 1;
            let mut depth = 1;
            while i < len && depth > 0 {
                buf.push(chars[i]);
                if chars[i] == ch {
                    depth += 1;
                } else if chars[i] == close {
                    depth -= 1;
                }
                i += 1;
            }
            let action = buf.trim().to_string();
            if !action.is_empty() {
                segments.push(action);
            }
            buf.clear();
            continue;
        }

        // 双换行 → 段落分隔
        if ch == '\n' && i + 1 < len && chars[i + 1] == '\n' {
            let trimmed = buf.trim().to_string();
            if !trimmed.is_empty() {
                segments.push(trimmed);
            }
            buf.clear();
            i += 2;
            continue;
        }

        // 单换行
        if ch == '\n' {
            let trimmed = buf.trim().to_string();
            if !trimmed.is_empty() {
                segments.push(trimmed);
                buf.clear();
            }
            i += 1;
            continue;
        }

        buf.push(ch);
        let buf_len = buf.chars().count();

        // 在emoji后且达到最小长度时，若后面是空格/标点/结束则分段（emoji常出现在句尾）
        if is_emoji(ch) && buf_len >= MIN_SEG_LEN {
            match next_ch {
                None | Some(' ') | Some('\n') | Some('　') => {
                    let trimmed = buf.trim().to_string();
                    segments.push(trimmed);
                    buf.clear();
                    i += 1;
                    continue;
                }
                _ => {}
            }
        }

        // 句末标点：达到目标长度后可分段
        if is_sentence_terminator(ch, next_ch) && buf_len >= MIN_SEG_LEN {
            // 如果下一个字符是引号/闭括号等，把它也收入当前段再分段
            if let Some(nc) = next_ch {
                if matches!(nc, '"' | '"' | '"' | '」' | '）' | ')') {
                    buf.push(nc);
                    i += 1;
                }
            }
            let trimmed = buf.trim().to_string();
            // 分段：如果长度接近目标则分，太短则继续累积
            if buf_len >= TARGET_SEG_LEN || next_ch.is_none() {
                segments.push(trimmed);
                buf.clear();
            }
        } else if buf_len >= MAX_SEG_LEN {
            // 超长保护：在逗号/顿号/分号/空格处强制截断
            if matches!(ch, '，' | '、' | '；' | ',' | ';' | ' ') {
                let trimmed = buf.trim().to_string();
                segments.push(trimmed);
                buf.clear();
            }
        }

        i += 1;
    }

    // 收尾
    let remaining = buf.trim().to_string();
    if !remaining.is_empty() {
        segments.push(remaining);
    }

    // 合并过短的碎片段（<15字符且非动作描写）到前一段
    let mut merged: Vec<String> = Vec::new();
    for seg in segments {
        let seg_len = seg.chars().count();
        let is_action = seg.starts_with('（') || seg.starts_with('(');
        if seg_len < 15 && !merged.is_empty() && !is_action {
            // 不合并以标点/表情结尾的独立短句（如"好呀！""嗯嗯~"）——这类本身就是自然的短消息
            let ends_with_terminal = seg.chars().last().map(|c| matches!(c, '。' | '！' | '？' | '~' | '～' | '!' | '?') || is_emoji(c)).unwrap_or(false);
            if ends_with_terminal && seg_len >= 4 {
                merged.push(seg);
            } else if let Some(last) = merged.last_mut() {
                last.push_str(&seg);
            } else {
                merged.push(seg);
            }
        } else {
            merged.push(seg);
        }
    }

    if merged.is_empty() {
        vec![text.to_string()]
    } else {
        merged
    }
}
