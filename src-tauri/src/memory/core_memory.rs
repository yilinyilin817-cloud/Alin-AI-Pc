use crate::storage::Database;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoreMemory {
    pub persona_id: String,
    pub user_profile: UserProfile,
    pub preferences: Preferences,
    pub relationships: Vec<PersonRef>,
    pub pets: Vec<PetInfo>,
    pub key_events: Vec<KeyEvent>,
    pub shared_memories: SharedMemories,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserProfile {
    pub name: Option<String>,
    pub nickname: Option<String>,
    pub birthday: Option<String>,
    pub age: Option<u32>,
    pub gender: Option<String>,
    pub occupation: Option<String>,
    pub city: Option<String>,
    pub education: Option<String>,
    pub mbti: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Preferences {
    pub favorite_foods: Vec<String>,
    pub favorite_music: Vec<String>,
    pub favorite_movies: Vec<String>,
    pub favorite_games: Vec<String>,
    pub hobbies: Vec<String>,
    pub dislikes: Vec<String>,
    pub favorite_color: Option<String>,
    pub routines: Vec<String>,
    pub sleep_time: Option<String>,
    pub wake_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonRef {
    pub name: String,
    pub relation: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetInfo {
    pub name: String,
    pub species: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    pub date: String,
    pub title: String,
    pub description: String,
    pub emotion: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SharedMemories {
    pub our_songs: Vec<String>,
    pub our_places: Vec<String>,
    pub inside_jokes: Vec<String>,
    pub promises: Vec<String>,
    pub special_dates: Vec<SpecialDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialDate {
    pub date: String,
    pub label: String,
    pub note: Option<String>,
}

pub fn ensure_tables(db: &Database) -> rusqlite::Result<()> {
    db.with_conn(|conn| {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS core_memory (
                persona_id  TEXT PRIMARY KEY REFERENCES persona(id) ON DELETE CASCADE,
                memory_json TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );"
        )
    })
}

pub fn get_or_create(db: &Database, persona_id: &str) -> CoreMemory {
    db.with_conn(|conn| {
        let json: Option<String> = conn
            .query_row(
                "SELECT memory_json FROM core_memory WHERE persona_id = ?1",
                params![persona_id],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(j) = json {
            if let Ok(mut mem) = serde_json::from_str::<CoreMemory>(&j) {
                mem.persona_id = persona_id.to_string();
                return Ok(mem);
            }
        }

        let now = chrono::Utc::now().to_rfc3339();
        let mem = CoreMemory {
            persona_id: persona_id.to_string(),
            updated_at: now.clone(),
            ..Default::default()
        };
        let json_str = serde_json::to_string(&mem).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO core_memory (persona_id, memory_json, updated_at) VALUES (?1, ?2, ?3)",
            params![persona_id, json_str, now],
        )?;
        Ok(mem)
    })
    .unwrap_or_else(|_| CoreMemory {
        persona_id: persona_id.to_string(),
        ..Default::default()
    })
}

pub fn save(db: &Database, mem: &CoreMemory) -> rusqlite::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut mem = mem.clone();
    mem.updated_at = now.clone();
    let json_str = serde_json::to_string(&mem).unwrap_or_default();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO core_memory (persona_id, memory_json, updated_at) VALUES (?1, ?2, ?3)",
            params![mem.persona_id, json_str, now],
        )?;
        Ok(())
    })
}

pub fn build_core_memory_context(mem: &CoreMemory) -> String {
    let mut parts = Vec::new();

    let up = &mem.user_profile;
    let mut profile_parts = Vec::new();
    if let Some(ref name) = up.name {
        profile_parts.push(format!("姓名：{name}"));
    }
    if let Some(ref nick) = up.nickname {
        profile_parts.push(format!("昵称：{nick}"));
    }
    if let Some(ref bday) = up.birthday {
        profile_parts.push(format!("生日：{bday}"));
    }
    if let Some(ref age) = up.age {
        profile_parts.push(format!("年龄：{age}"));
    }
    if let Some(ref gender) = up.gender {
        profile_parts.push(format!("性别：{gender}"));
    }
    if let Some(ref occ) = up.occupation {
        profile_parts.push(format!("职业：{occ}"));
    }
    if let Some(ref city) = up.city {
        profile_parts.push(format!("所在城市：{city}"));
    }
    if let Some(ref mbti) = up.mbti {
        profile_parts.push(format!("MBTI：{mbti}"));
    }
    if !profile_parts.is_empty() {
        parts.push("【用户基本信息】\n".to_string() + &profile_parts.join("\n"));
    }

    let prefs = &mem.preferences;
    let mut pref_parts = Vec::new();
    if !prefs.hobbies.is_empty() {
        pref_parts.push(format!("兴趣爱好：{}", prefs.hobbies.join("、")));
    }
    if !prefs.favorite_foods.is_empty() {
        pref_parts.push(format!("喜欢的食物：{}", prefs.favorite_foods.join("、")));
    }
    if !prefs.favorite_music.is_empty() {
        pref_parts.push(format!("喜欢的音乐：{}", prefs.favorite_music.join("、")));
    }
    if !prefs.favorite_movies.is_empty() {
        pref_parts.push(format!("喜欢的电影：{}", prefs.favorite_movies.join("、")));
    }
    if !prefs.dislikes.is_empty() {
        pref_parts.push(format!("不喜欢：{}", prefs.dislikes.join("、")));
    }
    if let Some(ref st) = prefs.sleep_time {
        pref_parts.push(format!("通常睡觉时间：{st}"));
    }
    if let Some(ref wt) = prefs.wake_time {
        pref_parts.push(format!("通常起床时间：{wt}"));
    }
    if !pref_parts.is_empty() {
        parts.push("【用户偏好】\n".to_string() + &pref_parts.join("\n"));
    }

    if !mem.key_events.is_empty() {
        let event_lines: Vec<String> = mem
            .key_events
            .iter()
            .map(|e| {
                let emo = e.emotion.as_deref().unwrap_or("");
                format!("· {}（{}）{}", e.title, e.date, if emo.is_empty() { String::new() } else { format!("[情绪:{emo}]") })
            })
            .collect();
        parts.push("【重要事件】\n".to_string() + &event_lines.join("\n"));
    }

    if !mem.relationships.is_empty() {
        let rel_lines: Vec<String> = mem
            .relationships
            .iter()
            .map(|r| {
                let note = r.notes.as_deref().unwrap_or("");
                format!("· {}（{}）{}", r.name, r.relation, note)
            })
            .collect();
        parts.push("【用户身边的人】\n".to_string() + &rel_lines.join("\n"));
    }

    if !mem.pets.is_empty() {
        let pet_lines: Vec<String> = mem
            .pets
            .iter()
            .map(|p| {
                let note = p.notes.as_deref().unwrap_or("");
                format!("· {}（{}）{}", p.name, p.species, note)
            })
            .collect();
        parts.push("【宠物】\n".to_string() + &pet_lines.join("\n"));
    }

    let sm = &mem.shared_memories;
    let mut shared_parts = Vec::new();
    if !sm.our_songs.is_empty() {
        shared_parts.push(format!("我们的歌：{}", sm.our_songs.join("、")));
    }
    if !sm.inside_jokes.is_empty() {
        shared_parts.push(format!("我们的小梗：{}", sm.inside_jokes.join("、")));
    }
    if !sm.promises.is_empty() {
        shared_parts.push(format!("说过的约定：{}", sm.promises.join("、")));
    }
    if !shared_parts.is_empty() {
        parts.push("【我们的共同记忆】\n".to_string() + &shared_parts.join("\n"));
    }

    if parts.is_empty() {
        return String::new();
    }

    "以下是你记住的关于用户的重要信息，请在对话中自然运用这些信息，不要刻意提到\"我记得\"之类的，而是自然地融入对话：\n\n"
        .to_string() + &parts.join("\n\n")
}
