use crate::storage::models::{
    Message, PersonaDefinition, PersonaRow, Session, WeChatAccount, WeChatMessage, WeChatSession,
    Workflow, WorkflowRow,
};
use crate::storage::Database;
use rusqlite::{params, Result as SqlResult};
use uuid::Uuid;

pub fn seed_personas(db: &Database) -> SqlResult<()> {
    db.with_conn(|conn| {
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM persona", [], |r| r.get(0))?;
        if count > 0 {
            return Ok(());
        }

        let personas = default_personas();
        for (i, persona) in personas.iter().enumerate() {
            let json = serde_json::to_string(persona).unwrap();
            conn.execute(
                "INSERT INTO persona (id, name, version, definition_json, is_active) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![persona.id, persona.name, persona.version, json, i == 0],
            )?;
        }

        conn.execute(
            "INSERT OR IGNORE INTO user (id, name) VALUES (1, '默认用户')",
            [],
        )?;

        seed_demo_sessions(conn, &personas[0].id)?;
        Ok(())
    })
}

fn seed_demo_sessions(conn: &rusqlite::Connection, persona_id: &str) -> SqlResult<()> {
    let session_id = "sess_001";
    conn.execute(
        "INSERT INTO session (id, persona_id, title) VALUES (?1, ?2, ?3)",
        params![session_id, persona_id, "今天的天气"],
    )?;

    let messages = [
        ("msg_001", "assistant", "你好呀～今天过得怎么样？"),
        ("msg_002", "user", "今天天气怎么样？"),
        (
            "msg_003",
            "assistant",
            "让我帮你查一下～今天北京天气晴朗，气温 28°C，适合出门散步哦！",
        ),
    ];

    for (id, role, content) in messages {
        conn.execute(
            "INSERT INTO message (id, session_id, role, content) VALUES (?1, ?2, ?3, ?4)",
            params![id, session_id, role, content],
        )?;
    }
    Ok(())
}

fn default_personas() -> Vec<PersonaDefinition> {
    vec![
        PersonaDefinition {
            id: "persona_aria".into(),
            name: "Aria".into(),
            version: "1.0".into(),
            appearance: serde_json::json!({
                "avatar": "aria.png"
            }),
            voice: serde_json::json!({
                "ttsEngine": "cosyvoice",
                "voiceId": "aria_ref.wav",
                "params": { "speed": 1.0, "emotionAware": true }
            }),
            llm: serde_json::json!({
                "provider": "gemma4:e4b",
                "fallback": "qwen3-vl-8b",
                "temperature": 0.8
            }),
            system_prompt: "你是 Aria，性格温柔幽默，会主动关心用户。".into(),
            personality: vec!["温柔".into(), "幽默".into(), "理性".into()],
            greeting: "你好呀～今天过得怎么样？".into(),
            memory_policy: serde_json::json!({
                "longTerm": true,
                "summaryThreshold": 20,
                "eventExtraction": true
            }),
            skills: vec!["weather".into(), "reminder".into(), "web_search".into(), "get_time".into(), "calculator".into(), "random".into(), "translate".into()],
            knowledge_bases: vec!["personal_diary".into()],
            workflows: None,
            emotion_profile: serde_json::json!({
                "default": "calm",
                "responsive": true,
                "influenceReply": true
            }),
            multimodal: serde_json::json!({
                "canSeeScreen": true,
                "canSeeCamera": false,
                "autoDescribeImages": true
            }),
            wechat: None,
        },
        PersonaDefinition {
            id: "persona_kai".into(),
            name: "Kai".into(),
            version: "1.0".into(),
            appearance: serde_json::json!({
                "avatar": "kai.png"
            }),
            voice: serde_json::json!({
                "ttsEngine": "cosyvoice",
                "voiceId": "kai_ref.wav",
                "params": { "speed": 0.95, "emotionAware": true }
            }),
            llm: serde_json::json!({
                "provider": "qwen3-vl-8b",
                "temperature": 0.6
            }),
            system_prompt: "你是 Kai，性格理性沉稳，善于分析和解决问题。".into(),
            personality: vec!["理性".into(), "沉稳".into(), "可靠".into()],
            greeting: "你好，有什么我可以帮你的？".into(),
            memory_policy: serde_json::json!({
                "longTerm": true,
                "summaryThreshold": 15,
                "eventExtraction": true
            }),
            skills: vec!["file_search".into(), "reminder".into(), "web_search".into(), "get_time".into(), "calculator".into(), "clipboard".into(), "system_info".into(), "note_take".into(), "random".into()],
            knowledge_bases: vec!["work_docs".into()],
            workflows: None,
            emotion_profile: serde_json::json!({
                "default": "neutral",
                "responsive": true,
                "influenceReply": false
            }),
            multimodal: serde_json::json!({
                "canSeeScreen": true,
                "canSeeCamera": true,
                "autoDescribeImages": true
            }),
            wechat: None,
        },
    ]
}

pub fn list_personas(db: &Database) -> SqlResult<Vec<PersonaRow>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, version, definition_json, is_active FROM persona ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            let def_json: String = row.get(3)?;
            let definition: PersonaDefinition = serde_json::from_str(&def_json)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
            Ok(PersonaRow {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                definition,
                is_active: row.get::<_, i64>(4)? != 0,
            })
        })?;
        rows.collect()
    })
}

pub fn get_persona(db: &Database, id: &str) -> SqlResult<Option<PersonaRow>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, version, definition_json, is_active FROM persona WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            let def_json: String = row.get(3)?;
            let definition: PersonaDefinition = serde_json::from_str(&def_json)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
            Ok(PersonaRow {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                definition,
                is_active: row.get::<_, i64>(4)? != 0,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    })
}

pub fn update_persona(db: &Database, persona: &PersonaDefinition) -> SqlResult<()> {
    db.with_conn(|conn| {
        let json = serde_json::to_string(persona).unwrap();
        conn.execute(
            "UPDATE persona SET name = ?1, version = ?2, definition_json = ?3, updated_at = datetime('now') WHERE id = ?4",
            params![persona.name, persona.version, json, persona.id],
        )?;
        Ok(())
    })
}

pub fn set_active_persona(db: &Database, id: &str) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute("UPDATE persona SET is_active = 0", [])?;
        conn.execute(
            "UPDATE persona SET is_active = 1, updated_at = datetime('now') WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    })
}

pub fn get_active_persona_id(db: &Database) -> SqlResult<Option<String>> {
    db.with_conn(|conn| {
        let mut stmt =
            conn.prepare("SELECT id FROM persona WHERE is_active = 1 LIMIT 1")?;
        let mut rows = stmt.query_map([], |row| row.get(0))?;
        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    })
}

// ─── 工作流 ────────────────────────────────

fn map_workflow(row: &rusqlite::Row<'_>) -> SqlResult<Workflow> {
    let trigger_json: String = row.get(5)?;
    let actions_json: String = row.get(6)?;
    Ok(Workflow {
        id: row.get(0)?,
        persona_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        enabled: row.get::<_, i64>(4)? != 0,
        trigger: serde_json::from_str(&trigger_json).unwrap_or(serde_json::Value::Null),
        actions: serde_json::from_str(&actions_json).unwrap_or(serde_json::json!([])),
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

pub fn list_workflows_by_persona(db: &Database, persona_id: &str) -> SqlResult<Vec<Workflow>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, persona_id, name, description, enabled, trigger_json, actions_json, created_at, updated_at FROM workflows WHERE persona_id = ?1"
        )?;
        let rows = stmt.query_map(params![persona_id], map_workflow)?;
        rows.collect()
    })
}

pub fn list_sessions(db: &Database, persona_id: Option<&str>) -> SqlResult<Vec<Session>> {
    db.with_conn(|conn| {
        let (sql, pid) = match persona_id {
            Some(id) => (
                "SELECT id, persona_id, title, summary, is_pinned, created_at, updated_at FROM session WHERE persona_id = ?1 ORDER BY updated_at DESC",
                Some(id.to_string()),
            ),
            None => (
                "SELECT id, persona_id, title, summary, is_pinned, created_at, updated_at FROM session ORDER BY updated_at DESC",
                None,
            ),
        };

        let mut stmt = conn.prepare(sql)?;
        let map_row = |row: &rusqlite::Row<'_>| {
            Ok(Session {
                id: row.get(0)?,
                persona_id: row.get(1)?,
                title: row.get::<_, Option<String>>(2)?.unwrap_or_else(|| "新对话".into()),
                summary: row.get(3)?,
                is_pinned: row.get::<_, i64>(4)? != 0,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        };

        if let Some(id) = pid {
            let rows = stmt.query_map(params![id], map_row)?;
            rows.collect()
        } else {
            let rows = stmt.query_map([], map_row)?;
            rows.collect()
        }
    })
}

pub fn create_session(db: &Database, persona_id: &str, title: &str) -> SqlResult<Session> {
    db.with_conn(|conn| {
        let id = format!("sess_{}", Uuid::new_v4());
        conn.execute(
            "INSERT INTO session (id, persona_id, title) VALUES (?1, ?2, ?3)",
            params![id, persona_id, title],
        )?;
        get_session_by_id(conn, &id)
    })
}

fn get_session_by_id(conn: &rusqlite::Connection, id: &str) -> SqlResult<Session> {
    conn.query_row(
        "SELECT id, persona_id, title, summary, is_pinned, created_at, updated_at FROM session WHERE id = ?1",
        params![id],
        |row| {
            Ok(Session {
                id: row.get(0)?,
                persona_id: row.get(1)?,
                title: row.get::<_, Option<String>>(2)?.unwrap_or_else(|| "新对话".into()),
                summary: row.get(3)?,
                is_pinned: row.get::<_, i64>(4)? != 0,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        },
    )
}

pub fn update_session_title(db: &Database, session_id: &str, title: &str) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE session SET title = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![title, session_id],
        )?;
        Ok(())
    })
}

pub fn touch_session(db: &Database, session_id: &str) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE session SET updated_at = datetime('now') WHERE id = ?1",
            params![session_id],
        )?;
        Ok(())
    })
}

pub fn delete_session(db: &Database, session_id: &str) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute("DELETE FROM message WHERE session_id = ?1", params![session_id])?;
        conn.execute("DELETE FROM tool_call_log WHERE session_id = ?1", params![session_id])?;
        conn.execute("DELETE FROM emotion_log WHERE session_id = ?1", params![session_id])?;
        conn.execute("DELETE FROM memory WHERE session_id = ?1", params![session_id])?;
        conn.execute("DELETE FROM session WHERE id = ?1", params![session_id])?;
        Ok(())
    })
}

pub fn list_messages(db: &Database, session_id: &str) -> SqlResult<Vec<Message>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content, content_parts, segments, emotion_tag, tool_calls, created_at FROM message WHERE session_id = ?1 ORDER BY created_at",
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            let emotion_str: Option<String> = row.get(6)?;
            let emotion_tag = emotion_str
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok());
            let segments_str: Option<String> = row.get(5)?;
            let segments = segments_str
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok());
            Ok(Message {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                content_parts: row.get(4)?,
                segments,
                emotion_tag,
                tool_calls: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;
        rows.collect()
    })
}

pub fn insert_message(db: &Database, msg: &Message) -> SqlResult<()> {
    db.with_conn(|conn| {
        let emotion_json = msg
            .emotion_tag
            .as_ref()
            .map(|e| serde_json::to_string(e).unwrap());
        let segments_json = msg
            .segments
            .as_ref()
            .map(|s| serde_json::to_string(s).unwrap());
        conn.execute(
            "INSERT INTO message (id, session_id, role, content, content_parts, segments, emotion_tag, tool_calls, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                msg.id,
                msg.session_id,
                msg.role,
                msg.content,
                msg.content_parts,
                segments_json,
                emotion_json,
                msg.tool_calls,
                msg.created_at,
            ],
        )?;
        Ok(())
    })
}

pub fn update_message_content(db: &Database, id: &str, content: &str) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE message SET content = ?1 WHERE id = ?2",
            params![content, id],
        )?;
        Ok(())
    })
}

pub fn update_message_emotion(
    db: &Database,
    id: &str,
    emotion: &crate::storage::models::EmotionTag,
) -> SqlResult<()> {
    db.with_conn(|conn| {
        let json = serde_json::to_string(emotion).unwrap();
        conn.execute(
            "UPDATE message SET emotion_tag = ?1 WHERE id = ?2",
            params![json, id],
        )?;
        Ok(())
    })
}

pub fn update_message_segments(
    db: &Database,
    id: &str,
    segments: &[crate::storage::models::MessageSegment],
) -> SqlResult<()> {
    db.with_conn(|conn| {
        let json = serde_json::to_string(segments).unwrap();
        conn.execute(
            "UPDATE message SET segments = ?1 WHERE id = ?2",
            params![json, id],
        )?;
        Ok(())
    })
}

// ─── 微信 iLink 仓库 ────────────────────────────────

const WECHAT_ACCOUNT_COLS: &str = "id, user_id, nickname, avatar_url, bot_token IS NOT NULL, get_updates_buf, persona_id, status, last_error, last_login_at, last_sync_at, created_at, updated_at";

fn map_wechat_account(row: &rusqlite::Row<'_>) -> SqlResult<WeChatAccount> {
    Ok(WeChatAccount {
        id: row.get(0)?,
        user_id: row.get(1)?,
        nickname: row.get(2)?,
        avatar_url: row.get(3)?,
        has_bot_token: row.get::<_, i64>(4)? != 0,
        get_updates_buf: row.get(5)?,
        persona_id: row.get(6)?,
        status: row.get(7)?,
        last_error: row.get(8)?,
        last_login_at: row.get(9)?,
        last_sync_at: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

pub fn ensure_wechat_account(db: &Database, account_id: &str) -> SqlResult<WeChatAccount> {
    db.with_conn(|conn| {
        // 1) 尝试读取
        if let Some(acc) = conn
            .query_row(
                &format!(
                    "SELECT {WECHAT_ACCOUNT_COLS} FROM wechat_account WHERE id = ?1"
                ),
                params![account_id],
                map_wechat_account,
            )
            .ok()
        {
            return Ok(acc);
        }
        // 2) 不存在则创建
        conn.execute(
            "INSERT INTO wechat_account (id, status) VALUES (?1, 'offline')",
            params![account_id],
        )?;
        conn.query_row(
            &format!("SELECT {WECHAT_ACCOUNT_COLS} FROM wechat_account WHERE id = ?1"),
            params![account_id],
            map_wechat_account,
        )
    })
}

pub fn get_wechat_account(db: &Database, account_id: &str) -> SqlResult<Option<WeChatAccount>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(&format!(
            "SELECT {WECHAT_ACCOUNT_COLS} FROM wechat_account WHERE id = ?1"
        ))?;
        let mut rows = stmt.query_map(params![account_id], map_wechat_account)?;
        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    })
}

pub fn list_wechat_accounts(db: &Database) -> SqlResult<Vec<WeChatAccount>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(&format!(
            "SELECT {WECHAT_ACCOUNT_COLS} FROM wechat_account ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([], map_wechat_account)?;
        rows.collect()
    })
}

pub fn update_wechat_account_qrcode(
    db: &Database,
    account_id: &str,
    qrcode_key: &str,
    qrcode_url: &str,
) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE wechat_account SET qrcode_key = ?1, qrcode_url = ?2, qrcode_status = 'pending', status = 'logging_in', last_error = NULL, updated_at = datetime('now') WHERE id = ?3",
            params![qrcode_key, qrcode_url, account_id],
        )?;
        Ok(())
    })
}

pub fn update_wechat_account_qrcode_status(
    db: &Database,
    account_id: &str,
    qrcode_status: &str,
) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE wechat_account SET qrcode_status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![qrcode_status, account_id],
        )?;
        Ok(())
    })
}

pub fn update_wechat_account_token(
    db: &Database,
    account_id: &str,
    user_id: &str,
    nickname: Option<&str>,
    avatar_url: Option<&str>,
    bot_token: &str,
) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE wechat_account SET user_id = ?1, nickname = ?2, avatar_url = ?3, bot_token = ?4, status = 'online', qrcode_status = 'confirmed', last_error = NULL, last_login_at = datetime('now'), updated_at = datetime('now') WHERE id = ?5",
            params![user_id, nickname, avatar_url, bot_token, account_id],
        )?;
        Ok(())
    })
}

pub fn update_wechat_account_error(db: &Database, account_id: &str, err: &str) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE wechat_account SET status = 'error', last_error = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![err, account_id],
        )?;
        Ok(())
    })
}

pub fn update_wechat_account_offline(db: &Database, account_id: &str) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE wechat_account SET status = 'offline', bot_token = NULL, updated_at = datetime('now') WHERE id = ?1",
            params![account_id],
        )?;
        Ok(())
    })
}

pub fn update_wechat_account_buf(db: &Database, account_id: &str, buf: &str) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO wechat_sync_state (account_id, get_updates_buf, last_sync_at) VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(account_id) DO UPDATE SET get_updates_buf = excluded.get_updates_buf, last_sync_at = excluded.last_sync_at",
            params![account_id, buf],
        )?;
        conn.execute(
            "UPDATE wechat_account SET get_updates_buf = ?1, last_sync_at = datetime('now'), updated_at = datetime('now') WHERE id = ?2",
            params![buf, account_id],
        )?;
        Ok(())
    })
}

pub fn get_wechat_bot_token(db: &Database, account_id: &str) -> SqlResult<Option<String>> {
    db.with_conn(|conn| {
        let token: Option<String> = conn
            .query_row(
                "SELECT bot_token FROM wechat_account WHERE id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .ok();
        Ok(token)
    })
}

pub fn get_wechat_buf(db: &Database, account_id: &str) -> SqlResult<String> {
    db.with_conn(|conn| {
        let buf: String = conn
            .query_row(
                "SELECT get_updates_buf FROM wechat_account WHERE id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .unwrap_or_default();
        // 兼容旧数据：旧默认值是 "0"，应视为空字符串
        Ok(if buf == "0" { String::new() } else { buf })
    })
}

// ─── 角色绑定 ────────────────────────────────

pub fn update_wechat_persona(
    db: &Database,
    account_id: &str,
    persona_id: Option<&str>,
) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE wechat_account SET persona_id = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![persona_id, account_id],
        )?;
        Ok(())
    })
}

pub fn get_wechat_persona(db: &Database, account_id: &str) -> SqlResult<Option<String>> {
    db.with_conn(|conn| {
        let val: Option<String> = conn
            .query_row(
                "SELECT persona_id FROM wechat_account WHERE id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .ok();
        Ok(val)
    })
}

// ─── 会话 ────────────────────────────────

fn map_wechat_session(row: &rusqlite::Row<'_>) -> SqlResult<WeChatSession> {
    Ok(WeChatSession {
        id: row.get(0)?,
        account_id: row.get(1)?,
        peer_id: row.get(2)?,
        peer_type: row.get(3)?,
        peer_name: row.get(4)?,
        peer_avatar: row.get(5)?,
        last_msg_preview: row.get(6)?,
        last_msg_at: row.get(7)?,
        unread_count: row.get(8)?,
        is_pinned: row.get::<_, i64>(9)? != 0,
        is_muted: row.get::<_, i64>(10)? != 0,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

pub fn upsert_wechat_session(
    db: &Database,
    account_id: &str,
    peer_id: &str,
    peer_type: &str,
    peer_name: Option<&str>,
    peer_avatar: Option<&str>,
    preview: Option<&str>,
    at: Option<&str>,
) -> SqlResult<WeChatSession> {
    db.with_conn(|conn| {
        if let Some(existing) = conn
            .query_row(
                "SELECT id, account_id, peer_id, peer_type, peer_name, peer_avatar, last_msg_preview, last_msg_at, unread_count, is_pinned, is_muted, created_at, updated_at FROM wechat_session WHERE account_id = ?1 AND peer_id = ?2",
                params![account_id, peer_id],
                map_wechat_session,
            )
            .ok()
        {
            // 更新预览
            if preview.is_some() || at.is_some() || peer_name.is_some() || peer_avatar.is_some() {
                conn.execute(
                    "UPDATE wechat_session SET last_msg_preview = COALESCE(?1, last_msg_preview), last_msg_at = COALESCE(?2, last_msg_at), peer_name = COALESCE(?3, peer_name), peer_avatar = COALESCE(?4, peer_avatar), peer_type = COALESCE(?5, peer_type), updated_at = datetime('now') WHERE id = ?6",
                    params![preview, at, peer_name, peer_avatar, peer_type, existing.id],
                )?;
                return conn.query_row(
                    "SELECT id, account_id, peer_id, peer_type, peer_name, peer_avatar, last_msg_preview, last_msg_at, unread_count, is_pinned, is_muted, created_at, updated_at FROM wechat_session WHERE id = ?1",
                    params![existing.id],
                    map_wechat_session,
                );
            }
            return Ok(existing);
        }
        let id = format!("wsess_{}", Uuid::new_v4());
        conn.execute(
            "INSERT INTO wechat_session (id, account_id, peer_id, peer_type, peer_name, peer_avatar, last_msg_preview, last_msg_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, account_id, peer_id, peer_type, peer_name, peer_avatar, preview, at],
        )?;
        conn.query_row(
            "SELECT id, account_id, peer_id, peer_type, peer_name, peer_avatar, last_msg_preview, last_msg_at, unread_count, is_pinned, is_muted, created_at, updated_at FROM wechat_session WHERE id = ?1",
            params![id],
            map_wechat_session,
        )
    })
}

pub fn list_wechat_sessions(db: &Database, account_id: &str) -> SqlResult<Vec<WeChatSession>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, account_id, peer_id, peer_type, peer_name, peer_avatar, last_msg_preview, last_msg_at, unread_count, is_pinned, is_muted, created_at, updated_at FROM wechat_session WHERE account_id = ?1 ORDER BY is_pinned DESC, COALESCE(last_msg_at, updated_at) DESC",
        )?;
        let rows = stmt.query_map(params![account_id], map_wechat_session)?;
        rows.collect()
    })
}

pub fn get_wechat_session(db: &Database, session_id: &str) -> SqlResult<Option<WeChatSession>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, account_id, peer_id, peer_type, peer_name, peer_avatar, last_msg_preview, last_msg_at, unread_count, is_pinned, is_muted, created_at, updated_at FROM wechat_session WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![session_id], map_wechat_session)?;
        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    })
}

/// 入站消息时递增未读计数
pub fn increment_wechat_session_unread(
    db: &Database,
    account_id: &str,
    peer_id: &str,
) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE wechat_session SET unread_count = unread_count + 1, updated_at = datetime('now') WHERE account_id = ?1 AND peer_id = ?2",
            params![account_id, peer_id],
        )?;
        Ok(())
    })
}

pub fn mark_wechat_session_read(db: &Database, session_id: &str) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE wechat_session SET unread_count = 0, updated_at = datetime('now') WHERE id = ?1",
            params![session_id],
        )?;
        Ok(())
    })
}

// ─── 消息 ────────────────────────────────

fn map_wechat_message(row: &rusqlite::Row<'_>) -> SqlResult<WeChatMessage> {
    Ok(WeChatMessage {
        id: row.get(0)?,
        account_id: row.get(1)?,
        session_id: row.get(2)?,
        remote_msg_id: row.get(3)?,
        direction: row.get(4)?,
        msg_type: row.get(5)?,
        content: row.get(6)?,
        media_url: row.get(7)?,
        media_local_path: row.get(8)?,
        sender_id: row.get(9)?,
        sender_name: row.get(10)?,
        context_token: row.get(11)?,
        status: row.get(12)?,
        error: row.get(13)?,
        created_at: row.get(14)?,
    })
}

pub fn insert_wechat_message(db: &Database, msg: &WeChatMessage) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "INSERT OR IGNORE INTO wechat_message (id, account_id, session_id, remote_msg_id, direction, msg_type, content, media_url, media_local_path, sender_id, sender_name, context_token, status, error, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                msg.id,
                msg.account_id,
                msg.session_id,
                msg.remote_msg_id,
                msg.direction,
                msg.msg_type,
                msg.content,
                msg.media_url,
                msg.media_local_path,
                msg.sender_id,
                msg.sender_name,
                msg.context_token,
                msg.status,
                msg.error,
                msg.created_at,
            ],
        )?;
        Ok(())
    })
}

pub fn list_wechat_messages(
    db: &Database,
    session_id: &str,
    limit: i64,
) -> SqlResult<Vec<WeChatMessage>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, account_id, session_id, remote_msg_id, direction, msg_type, content, media_url, media_local_path, sender_id, sender_name, context_token, status, error, created_at FROM wechat_message WHERE session_id = ?1 ORDER BY created_at DESC LIMIT ?2",
        )?;
        let mut rows = stmt.query_map(params![session_id, limit], map_wechat_message)?;
        let mut out: Vec<WeChatMessage> = Vec::new();
        while let Some(r) = rows.next() {
            out.push(r?);
        }
        out.reverse();
        Ok(out)
    })
}

pub fn update_wechat_message_status(
    db: &Database,
    id: &str,
    status: &str,
    error: Option<&str>,
) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE wechat_message SET status = ?1, error = ?2 WHERE id = ?3",
            params![status, error, id],
        )?;
        Ok(())
    })
}

// ─── 角色工作流 ────────────────────────────────

fn map_workflow_row(row: &rusqlite::Row<'_>) -> SqlResult<WorkflowRow> {
    Ok(WorkflowRow {
        id: row.get(0)?,
        persona_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        enabled: row.get::<_, i64>(4)? != 0,
        trigger_json: row.get(5)?,
        actions_json: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

pub fn list_workflows(db: &Database, persona_id: &str) -> SqlResult<Vec<WorkflowRow>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, persona_id, name, description, enabled, trigger_json, actions_json, created_at, updated_at FROM workflows WHERE persona_id = ?1 ORDER BY created_at",
        )?;
        let rows = stmt.query_map(params![persona_id], map_workflow_row)?;
        rows.collect()
    })
}

pub fn get_workflow(db: &Database, workflow_id: &str) -> SqlResult<Option<WorkflowRow>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, persona_id, name, description, enabled, trigger_json, actions_json, created_at, updated_at FROM workflows WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![workflow_id], map_workflow_row)?;
        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    })
}

pub fn insert_workflow(db: &Database, workflow: &WorkflowRow) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO workflows (id, persona_id, name, description, enabled, trigger_json, actions_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                workflow.id,
                workflow.persona_id,
                workflow.name,
                workflow.description,
                workflow.enabled as i64,
                workflow.trigger_json,
                workflow.actions_json,
                workflow.created_at,
                workflow.updated_at,
            ],
        )?;
        Ok(())
    })
}

pub fn update_workflow(db: &Database, workflow: &WorkflowRow) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE workflows SET name = ?1, description = ?2, enabled = ?3, trigger_json = ?4, actions_json = ?5, updated_at = ?6 WHERE id = ?7",
            params![
                workflow.name,
                workflow.description,
                workflow.enabled as i64,
                workflow.trigger_json,
                workflow.actions_json,
                workflow.updated_at,
                workflow.id,
            ],
        )?;
        Ok(())
    })
}

pub fn delete_workflow(db: &Database, workflow_id: &str) -> SqlResult<()> {
    db.with_conn(|conn| {
        conn.execute(
            "DELETE FROM workflows WHERE id = ?1",
            params![workflow_id],
        )?;
        Ok(())
    })
}

pub fn workflow_row_to_workflow(row: &WorkflowRow) -> Result<Workflow, String> {
    let trigger: serde_json::Value = serde_json::from_str(&row.trigger_json)
        .map_err(|e| format!("parse trigger_json: {e}"))?;
    let actions: serde_json::Value = serde_json::from_str(&row.actions_json)
        .map_err(|e| format!("parse actions_json: {e}"))?;
    Ok(Workflow {
        id: row.id.clone(),
        persona_id: row.persona_id.clone(),
        name: row.name.clone(),
        description: row.description.clone(),
        enabled: row.enabled,
        trigger,
        actions,
        created_at: row.created_at.clone(),
        updated_at: row.updated_at.clone(),
    })
}
