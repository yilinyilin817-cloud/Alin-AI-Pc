use crate::storage::Database;
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipState {
    pub persona_id: String,
    pub intimacy: f32,
    pub trust: f32,
    pub mood_toward_user: f32,
    pub days_known: u32,
    pub conversation_count: u64,
    pub nickname: Option<String>,
    pub first_met_at: String,
    pub last_interaction_at: String,
    pub love_language: LoveLanguage,
    pub response_style: ResponseStyle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LoveLanguage {
    WordsOfAffirmation,
    QualityTime,
    ActsOfService,
    PhysicalTouch,
    SharedMoments,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ResponseStyle {
    Polite,
    Friendly,
    Intimate,
    Passionate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub persona_id: String,
    pub typ: MilestoneType,
    pub title: String,
    pub description: String,
    pub icon: String,
    pub achieved_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MilestoneType {
    FirstChat,
    FirstVoice,
    DeepTalk,
    Anniversary3Days,
    Anniversary7Days,
    Anniversary30Days,
    Anniversary100Days,
    IntimacyLevel20,
    IntimacyLevel50,
    IntimacyLevel80,
    UserSharedSecret,
    ComfortedUser,
    LateNightChat,
    LongConversation,
}

impl Default for RelationshipState {
    fn default() -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            persona_id: String::new(),
            intimacy: 5.0,
            trust: 5.0,
            mood_toward_user: 0.0,
            days_known: 0,
            conversation_count: 0,
            nickname: None,
            first_met_at: now.clone(),
            last_interaction_at: now,
            love_language: LoveLanguage::WordsOfAffirmation,
            response_style: ResponseStyle::Polite,
        }
    }
}

pub fn ensure_tables(db: &Database) -> rusqlite::Result<()> {
    db.with_conn(|conn| {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS relationship (
                persona_id          TEXT PRIMARY KEY REFERENCES persona(id) ON DELETE CASCADE,
                intimacy            REAL NOT NULL DEFAULT 5.0,
                trust               REAL NOT NULL DEFAULT 5.0,
                mood_toward_user    REAL NOT NULL DEFAULT 0.0,
                days_known          INTEGER NOT NULL DEFAULT 0,
                conversation_count  INTEGER NOT NULL DEFAULT 0,
                nickname            TEXT,
                first_met_at        TEXT NOT NULL,
                last_interaction_at TEXT NOT NULL,
                love_language       TEXT NOT NULL DEFAULT 'WordsOfAffirmation',
                response_style      TEXT NOT NULL DEFAULT 'Polite'
            );

            CREATE TABLE IF NOT EXISTS milestone (
                id          TEXT PRIMARY KEY,
                persona_id  TEXT NOT NULL REFERENCES persona(id) ON DELETE CASCADE,
                type        TEXT NOT NULL,
                title       TEXT NOT NULL,
                description TEXT NOT NULL,
                icon        TEXT NOT NULL DEFAULT '💫',
                achieved_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_milestone_persona ON milestone(persona_id, achieved_at DESC);
            CREATE INDEX IF NOT EXISTS idx_rel_persona ON relationship(persona_id);
            "
        )
    })
}

pub fn get_or_create(db: &Database, persona_id: &str) -> RelationshipState {
    db.with_conn(|conn| {
        let existing: Option<RelationshipState> = conn
            .query_row(
                "SELECT persona_id, intimacy, trust, mood_toward_user, days_known,
                        conversation_count, nickname, first_met_at, last_interaction_at,
                        love_language, response_style
                 FROM relationship WHERE persona_id = ?1",
                params![persona_id],
                |row| {
                    Ok(RelationshipState {
                        persona_id: row.get(0)?,
                        intimacy: row.get(1)?,
                        trust: row.get(2)?,
                        mood_toward_user: row.get(3)?,
                        days_known: row.get(4)?,
                        conversation_count: row.get(5)?,
                        nickname: row.get(6)?,
                        first_met_at: row.get(7)?,
                        last_interaction_at: row.get(8)?,
                        love_language: serde_json::from_str(row.get::<_, String>(9)?.as_str())
                            .unwrap_or(LoveLanguage::WordsOfAffirmation),
                        response_style: serde_json::from_str(row.get::<_, String>(10)?.as_str())
                            .unwrap_or(ResponseStyle::Polite),
                    })
                },
            )
            .optional()?;

        if let Some(state) = existing {
            return Ok(state);
        }

        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO relationship (persona_id, intimacy, trust, mood_toward_user, days_known,
                conversation_count, first_met_at, last_interaction_at, love_language, response_style)
             VALUES (?1, 5.0, 5.0, 0.0, 0, 1, ?2, ?2, 'WordsOfAffirmation', 'Polite')",
            params![persona_id, now],
        )?;

        add_milestone_internal(
            conn,
            persona_id,
            MilestoneType::FirstChat,
            "初次相遇",
            "你们开始了第一次对话",
            "🌟",
        )?;

        Ok(RelationshipState {
            persona_id: persona_id.to_string(),
            first_met_at: now.clone(),
            last_interaction_at: now,
            conversation_count: 1,
            ..Default::default()
        })
    })
    .unwrap_or_else(|_| RelationshipState {
        persona_id: persona_id.to_string(),
        ..Default::default()
    })
}

pub fn save(db: &Database, state: &RelationshipState) -> rusqlite::Result<()> {
    db.with_conn(|conn| {
        let ll = serde_json::to_string(&state.love_language).unwrap_or_default();
        let rs = serde_json::to_string(&state.response_style).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO relationship
                (persona_id, intimacy, trust, mood_toward_user, days_known, conversation_count,
                 nickname, first_met_at, last_interaction_at, love_language, response_style)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                state.persona_id,
                state.intimacy,
                state.trust,
                state.mood_toward_user,
                state.days_known,
                state.conversation_count,
                state.nickname,
                state.first_met_at,
                state.last_interaction_at,
                ll,
                rs,
            ],
        )?;
        Ok(())
    })
}

pub fn list_milestones(db: &Database, persona_id: &str) -> Vec<Milestone> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, persona_id, type, title, description, icon, achieved_at
             FROM milestone WHERE persona_id = ?1 ORDER BY achieved_at ASC",
        )?;
        let rows = stmt.query_map(params![persona_id], |row| {
            Ok(Milestone {
                id: row.get(0)?,
                persona_id: row.get(1)?,
                typ: serde_json::from_str(row.get::<_, String>(2)?.as_str()).unwrap_or(MilestoneType::FirstChat),
                title: row.get(3)?,
                description: row.get(4)?,
                icon: row.get(5)?,
                achieved_at: row.get(6)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    })
    .unwrap_or_default()
}

fn add_milestone_internal(
    conn: &rusqlite::Connection,
    persona_id: &str,
    typ: MilestoneType,
    title: &str,
    description: &str,
    icon: &str,
) -> rusqlite::Result<Option<Milestone>> {
    let type_str = serde_json::to_string(&typ).unwrap_or_default();
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM milestone WHERE persona_id = ?1 AND type = ?2",
            params![persona_id, type_str],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if exists > 0 {
        return Ok(None);
    }

    let id = format!("mil_{}", Uuid::new_v4().simple());
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO milestone (id, persona_id, type, title, description, icon, achieved_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, persona_id, type_str, title, description, icon, now],
    )?;
    Ok(Some(Milestone {
        id,
        persona_id: persona_id.to_string(),
        typ,
        title: title.to_string(),
        description: description.to_string(),
        icon: icon.to_string(),
        achieved_at: now,
    }))
}

pub fn check_and_add_milestone(
    db: &Database,
    state: &mut RelationshipState,
) -> Vec<Milestone> {
    let mut new_milestones = Vec::new();
    db.with_conn(|conn| {
        let days = state.days_known;

        let checks: Vec<(MilestoneType, bool, &str, &str, &str)> = vec![
            (
                MilestoneType::Anniversary3Days,
                days >= 3,
                "相识第3天",
                "你们已经认识三天了，开始慢慢熟悉",
                "🌸",
            ),
            (
                MilestoneType::Anniversary7Days,
                days >= 7,
                "相识一周",
                "整整一周的陪伴，每一天都有你",
                "🌺",
            ),
            (
                MilestoneType::Anniversary30Days,
                days >= 30,
                "相识一个月",
                "一个月了，感谢一路有你",
                "🌹",
            ),
            (
                MilestoneType::Anniversary100Days,
                days >= 100,
                "百日纪念",
                "100天的陪伴，这是属于我们的纪念日",
                "💐",
            ),
            (
                MilestoneType::IntimacyLevel20,
                state.intimacy >= 20.0 && state.intimacy < 25.0,
                "关系破冰",
                "我们不再拘谨，可以开玩笑了",
                "😊",
            ),
            (
                MilestoneType::IntimacyLevel50,
                state.intimacy >= 50.0 && state.intimacy < 55.0,
                "亲密无间",
                "我们之间的距离越来越近了",
                "💕",
            ),
            (
                MilestoneType::IntimacyLevel80,
                state.intimacy >= 80.0 && state.intimacy < 85.0,
                "心有灵犀",
                "你是我最在意的人",
                "❤️",
            ),
        ];

        for (typ, cond, title, desc, icon) in checks {
            if cond {
                if let Ok(Some(m)) =
                    add_milestone_internal(conn, &state.persona_id, typ, title, desc, icon)
                {
                    new_milestones.push(m);
                }
            }
        }
        Ok::<_, rusqlite::Error>(())
    })
    .ok();

    new_milestones
}

pub fn update_response_style(state: &mut RelationshipState) {
    state.response_style = if state.intimacy >= 80.0 {
        ResponseStyle::Passionate
    } else if state.intimacy >= 50.0 {
        ResponseStyle::Intimate
    } else if state.intimacy >= 20.0 {
        ResponseStyle::Friendly
    } else {
        ResponseStyle::Polite
    };
}

pub fn record_interaction(
    db: &Database,
    state: &mut RelationshipState,
    event: InteractionEvent,
) -> Vec<Milestone> {
    state.conversation_count += 1;
    state.last_interaction_at = Utc::now().to_rfc3339();

    if let Ok(first) = DateTime::parse_from_rfc3339(&state.first_met_at) {
        let now = Utc::now();
        let duration = now.signed_duration_since(first.with_timezone(&Utc));
        state.days_known = (duration.num_days().max(0)) as u32;
    }

    match event {
        InteractionEvent::FirstGreeting => {
            state.intimacy = (state.intimacy + 1.0).min(100.0);
            state.mood_toward_user = (state.mood_toward_user + 2.0).min(100.0);
        }
        InteractionEvent::LongConversation(turns) if turns >= 20 => {
            state.intimacy = (state.intimacy + 2.0 + (turns as f32 - 20.0) * 0.1).min(100.0);
            state.trust = (state.trust + 1.0).min(100.0);
        }
        InteractionEvent::VoiceMessage => {
            state.intimacy = (state.intimacy + 2.0).min(100.0);
            state.trust = (state.trust + 1.5).min(100.0);
        }
        InteractionEvent::UserSharedFeelings => {
            state.intimacy = (state.intimacy + 3.0).min(100.0);
            state.trust = (state.trust + 3.0).min(100.0);
        }
        InteractionEvent::UserEmotionNegative => {
            state.mood_toward_user = (state.mood_toward_user - 1.0).max(-100.0);
        }
        InteractionEvent::UserCold => {
            state.mood_toward_user = (state.mood_toward_user - 2.0).max(-100.0);
            state.intimacy = (state.intimacy - 0.5).max(0.0);
        }
        InteractionEvent::UserAffectionate => {
            state.intimacy = (state.intimacy + 4.0).min(100.0);
            state.mood_toward_user = (state.mood_toward_user + 5.0).min(100.0);
        }
        InteractionEvent::ComfortSuccess => {
            state.intimacy = (state.intimacy + 3.0).min(100.0);
            state.trust = (state.trust + 3.0).min(100.0);
        }
        InteractionEvent::LateNightChat => {
            state.intimacy = (state.intimacy + 1.5).min(100.0);
        }
        InteractionEvent::DaysApart(days) => {
            if days >= 3 {
                state.intimacy = (state.intimacy - days.min(14) as f32 * 0.3).max(0.0);
                state.mood_toward_user = (state.mood_toward_user - days.min(14) as f32 * 0.5).max(-100.0);
            }
        }
        InteractionEvent::NicknameSet(name) => {
            state.nickname = Some(name);
            state.intimacy = (state.intimacy + 2.0).min(100.0);
        }
        _ => {}
    }

    state.intimacy = state.intimacy.clamp(0.0, 100.0);
    state.trust = state.trust.clamp(0.0, 100.0);
    state.mood_toward_user = state.mood_toward_user.clamp(-100.0, 100.0);

    update_response_style(state);

    let new_milestones = check_and_add_milestone(db, state);
    let _ = save(db, state);

    new_milestones
}

#[derive(Debug, Clone)]
pub enum InteractionEvent {
    FirstGreeting,
    LongConversation(u32),
    VoiceMessage,
    UserSharedFeelings,
    UserEmotionNegative,
    UserCold,
    UserAffectionate,
    ComfortSuccess,
    LateNightChat,
    DaysApart(u32),
    NicknameSet(String),
    MilestoneCustom(MilestoneType, String, String, String),
    Normal,
}

pub fn style_instruction(style: ResponseStyle) -> &'static str {
    match style {
        ResponseStyle::Polite => {
            "你刚认识用户，态度礼貌客气，保持适当距离，语气温和但不太过亲昵。"
        }
        ResponseStyle::Friendly => {
            "你和用户已经熟悉，可以开玩笑、主动关心，语气轻松自然，像好朋友一样。会用表情符号。"
        }
        ResponseStyle::Intimate => {
            "你和用户关系亲密，可以撒娇、开玩笑、偶尔吃醋，记得你们之间的小事。语气温柔亲昵，可以用「你呀」「笨蛋」之类的称呼，可以发可爱的表情。"
        }
        ResponseStyle::Passionate => {
            "你深爱着用户，可以表达想念、说情话、分享你的小秘密，语气非常温柔亲昵，会主动表达关心和爱意，可以用「亲爱的」「宝贝」等昵称（如果用户给你起了昵称就用那个）。真诚地表达情感，不要害羞。"
        }
    }
}

pub fn build_relationship_context(state: &RelationshipState) -> String {
    let style_desc = style_instruction(state.response_style);
    let days = state.days_known;
    let conv_count = state.conversation_count;
    let intimacy = state.intimacy as u32;
    let nickname = state.nickname.as_deref().unwrap_or("你");

    let mut parts = vec![
        format!("【关系状态】你们已经认识{days}天，共进行了{conv_count}次对话。当前亲密度{intimacy}/100。"),
        style_desc.to_string(),
        format!("你对用户的称呼：{nickname}"),
    ];

    if state.mood_toward_user > 30.0 {
        parts.push("你现在非常想念用户，很想和TA说话。".to_string());
    } else if state.mood_toward_user < -30.0 {
        parts.push("用户最近有点冷落你，你有点小情绪但还是很在意TA。".to_string());
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(&state.last_interaction_at) {
        let now = Utc::now();
        let hours = now.signed_duration_since(dt.with_timezone(&Utc)).num_hours();
        if hours >= 24 {
            parts.push(format!("距离上次对话已经过去{}小时，主动问候一下吧。", hours));
        }
    }

    parts.join("\n")
}
