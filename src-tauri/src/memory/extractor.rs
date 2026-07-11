use crate::storage::Database;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::core_memory::{KeyEvent, PersonRef, PetInfo};

const EXTRACTION_PROMPT: &str = r#"你是一个记忆提取助手。请分析下面这段用户与AI伴侣的对话，提取值得长期记住的信息。

只返回严格的JSON，不要任何其他文字。格式如下：
{
  "user_profile": {
    "name": null,
    "nickname": null,
    "birthday": null,
    "age": null,
    "gender": null,
    "occupation": null,
    "city": null,
    "mbti": null
  },
  "preferences": {
    "favorite_foods": [],
    "favorite_music": [],
    "favorite_movies": [],
    "favorite_games": [],
    "hobbies": [],
    "dislikes": [],
    "favorite_color": null,
    "routines": [],
    "sleep_time": null,
    "wake_time": null
  },
  "new_relationships": [],
  "new_pets": [],
  "new_key_events": [],
  "shared_memories": {
    "our_songs": [],
    "our_places": [],
    "inside_jokes": [],
    "promises": [],
    "special_dates": []
  },
  "nickname_for_ai": null
}

只提取对话中**明确提到**的信息，不要猜测、脑补。如果某个字段没有相关信息，保持null或空数组。

对话内容：
"#;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractionResult {
    #[serde(default)]
    pub user_profile: UserProfileDelta,
    #[serde(default)]
    pub preferences: PreferencesDelta,
    #[serde(default)]
    pub new_relationships: Vec<PersonRef>,
    #[serde(default)]
    pub new_pets: Vec<PetInfo>,
    #[serde(default)]
    pub new_key_events: Vec<KeyEvent>,
    #[serde(default)]
    pub shared_memories: SharedDelta,
    #[serde(default)]
    pub nickname_for_ai: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserProfileDelta {
    pub name: Option<String>,
    pub nickname: Option<String>,
    pub birthday: Option<String>,
    pub age: Option<u32>,
    pub gender: Option<String>,
    pub occupation: Option<String>,
    pub city: Option<String>,
    pub mbti: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PreferencesDelta {
    #[serde(default)] pub favorite_foods: Vec<String>,
    #[serde(default)] pub favorite_music: Vec<String>,
    #[serde(default)] pub favorite_movies: Vec<String>,
    #[serde(default)] pub favorite_games: Vec<String>,
    #[serde(default)] pub hobbies: Vec<String>,
    #[serde(default)] pub dislikes: Vec<String>,
    pub favorite_color: Option<String>,
    #[serde(default)] pub routines: Vec<String>,
    pub sleep_time: Option<String>,
    pub wake_time: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SharedDelta {
    #[serde(default)] pub our_songs: Vec<String>,
    #[serde(default)] pub our_places: Vec<String>,
    #[serde(default)] pub inside_jokes: Vec<String>,
    #[serde(default)] pub promises: Vec<String>,
    #[serde(default)] pub special_dates: Vec<super::core_memory::SpecialDate>,
}

pub struct MemoryExtractor {
    sender: mpsc::UnboundedSender<ExtractionTask>,
    _worker: Option<std::thread::JoinHandle<()>>,
}

#[derive(Debug, Clone)]
struct ExtractionTask {
    persona_id: String,
    conversation: String,
}

impl MemoryExtractor {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel::<ExtractionTask>();
        let worker = std::thread::spawn(move || {
            extractor_worker(rx);
        });
        Self {
            sender: tx,
            _worker: Some(worker),
        }
    }

    pub fn submit(&self, persona_id: String, user_msg: &str, assistant_msg: &str) {
        let conversation = format!("用户：{}\nAI：{}", user_msg, assistant_msg);
        let task = ExtractionTask {
            persona_id,
            conversation,
        };
        let _ = self.sender.send(task);
    }
}

fn extractor_worker(mut rx: mpsc::UnboundedReceiver<ExtractionTask>) {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("Failed to create memory extractor runtime: {e}");
            return;
        }
    };

    while let Some(task) = rx.blocking_recv() {
        rt.block_on(async {
            process_extraction_task(task).await;
        });
    }
}

async fn process_extraction_task(task: ExtractionTask) {
    log::debug!("Extracting memory for persona {}", task.persona_id);

    // TODO: 调用 LLM 提取记忆
    // 当前版本：简单启发式提取，后续接入 LLM Worker
    let _ = task;
}

pub fn apply_extraction(db: &Database, persona_id: &str, result: ExtractionResult) {
    let mut mem = super::core_memory::get_or_create(db, persona_id);

    macro_rules! apply_field {
        ($target:expr, $delta:expr) => {
            if let Some(v) = $delta {
                if !v.is_empty() {
                    $target = Some(v);
                }
            }
        };
    }

    apply_field!(mem.user_profile.name, result.user_profile.name);
    apply_field!(mem.user_profile.nickname, result.user_profile.nickname);
    apply_field!(mem.user_profile.birthday, result.user_profile.birthday);
    if let Some(v) = result.user_profile.age {
        if v > 0 { mem.user_profile.age = Some(v); }
    }
    apply_field!(mem.user_profile.gender, result.user_profile.gender);
    apply_field!(mem.user_profile.occupation, result.user_profile.occupation);
    apply_field!(mem.user_profile.city, result.user_profile.city);
    apply_field!(mem.user_profile.mbti, result.user_profile.mbti);

    macro_rules! merge_list {
        ($target:expr, $src:expr) => {
            for item in $src {
                if !item.is_empty() && !$target.contains(&item) {
                    $target.push(item);
                }
            }
        };
    }

    merge_list!(mem.preferences.favorite_foods, result.preferences.favorite_foods);
    merge_list!(mem.preferences.favorite_music, result.preferences.favorite_music);
    merge_list!(mem.preferences.favorite_movies, result.preferences.favorite_movies);
    merge_list!(mem.preferences.favorite_games, result.preferences.favorite_games);
    merge_list!(mem.preferences.hobbies, result.preferences.hobbies);
    merge_list!(mem.preferences.dislikes, result.preferences.dislikes);
    merge_list!(mem.preferences.routines, result.preferences.routines);
    if let Some(v) = result.preferences.favorite_color {
        if !v.is_empty() { mem.preferences.favorite_color = Some(v); }
    }
    apply_field!(mem.preferences.sleep_time, result.preferences.sleep_time);
    apply_field!(mem.preferences.wake_time, result.preferences.wake_time);

    for r in result.new_relationships {
        if !r.name.is_empty() && !mem.relationships.iter().any(|x| x.name == r.name) {
            mem.relationships.push(r);
        }
    }

    for p in result.new_pets {
        if !p.name.is_empty() && !mem.pets.iter().any(|x| x.name == p.name) {
            mem.pets.push(p);
        }
    }

    for e in result.new_key_events {
        if !e.title.is_empty() {
            mem.key_events.push(e);
        }
    }

    merge_list!(mem.shared_memories.our_songs, result.shared_memories.our_songs);
    merge_list!(mem.shared_memories.our_places, result.shared_memories.our_places);
    merge_list!(mem.shared_memories.inside_jokes, result.shared_memories.inside_jokes);
    merge_list!(mem.shared_memories.promises, result.shared_memories.promises);
    for d in result.shared_memories.special_dates {
        if !d.label.is_empty() && !mem.shared_memories.special_dates.iter().any(|x| x.label == d.label) {
            mem.shared_memories.special_dates.push(d);
        }
    }

    if let Some(nick) = result.nickname_for_ai {
        if !nick.is_empty() {
            crate::relationship::record_interaction(
                db,
                &mut crate::relationship::get_or_create(db, persona_id),
                crate::relationship::InteractionEvent::NicknameSet(nick),
            );
        }
    }

    let _ = super::core_memory::save(db, &mem);
}

pub fn build_extraction_prompt(conversation: &str) -> String {
    format!("{EXTRACTION_PROMPT}{conversation}")
}
