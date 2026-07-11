use crate::state::AppState;
use crate::storage::models::PersonaRow;
use chrono::{Local, Timelike};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveSettings {
    pub enabled: bool,
    pub mode: ProactiveMode,
    pub quiet_hours_start: Option<u32>,
    pub quiet_hours_end: Option<u32>,
    pub max_daily: u32,
    pub good_morning: bool,
    pub good_night: bool,
    pub miss_you_threshold_hours: u32,
    pub anniversary_reminder: bool,
    pub random_checkins: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ProactiveMode {
    Quiet,
    Normal,
    Clingy,
}

impl Default for ProactiveSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: ProactiveMode::Normal,
            quiet_hours_start: Some(23),
            quiet_hours_end: Some(8),
            max_daily: 3,
            good_morning: true,
            good_night: true,
            miss_you_threshold_hours: 6,
            anniversary_reminder: true,
            random_checkins: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProactiveMessage {
    pub id: String,
    pub persona_id: String,
    pub persona_name: String,
    pub trigger_type: String,
    pub content: String,
    pub created_at: String,
    pub delivered: bool,
}

pub struct ProactiveEngine {
    app_handle: AppHandle,
    settings: Mutex<ProactiveSettings>,
    running: Mutex<bool>,
}

impl ProactiveEngine {
    pub fn new(app_handle: AppHandle) -> Arc<Self> {
        Arc::new(Self {
            app_handle,
            settings: Mutex::new(ProactiveSettings::default()),
            running: Mutex::new(false),
        })
    }

    pub fn start(self: &Arc<Self>) {
        let mut running = self.running.lock().unwrap();
        if *running {
            return;
        }
        *running = true;
        drop(running);

        let engine = self.clone();
        std::thread::spawn(move || {
            engine.run_loop();
        });
        log::info!("Proactive engine started");
    }

    fn run_loop(&self) {
        loop {
            std::thread::sleep(Duration::from_secs(5 * 60));

            let settings = self.settings.lock().unwrap().clone();
            if !settings.enabled {
                continue;
            }
            drop(settings);

            self.check_triggers();
        }
    }

    fn check_triggers(&self) {
        let state = self.app_handle.state::<AppState>();
        let settings = self.settings.lock().unwrap().clone();

        if Self::is_quiet_hour(&settings) {
            return;
        }

        let personas = match crate::storage::repo::list_personas(&state.db) {
            Ok(p) => p,
            Err(_) => return,
        };

        let now = Local::now();
        let hour = now.hour();

        for persona in &personas {
            let rel = crate::relationship::get_or_create(&state.db, &persona.id);

            let last_interaction = chrono::DateTime::parse_from_rfc3339(&rel.last_interaction_at)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| now - chrono::Duration::hours(24));

            let hours_since = now.signed_duration_since(last_interaction).num_hours().max(0) as u32;

            if settings.good_morning && hour >= 7 && hour <= 10 {
                let last = Self::get_last_trigger(&state, &persona.id, "morning");
                if Self::should_fire_today(last, &now) {
                    let msg = self.generate_greeting(&persona.definition, &persona.name, "morning", hours_since);
                    self.deliver_message(persona, "good_morning", msg);
                    Self::set_last_trigger(&state, &persona.id, "morning", &now.to_rfc3339());
                    continue;
                }
            }

            if settings.good_night && hour >= 22 {
                let last = Self::get_last_trigger(&state, &persona.id, "night");
                if Self::should_fire_today(last, &now) {
                    let msg = self.generate_greeting(&persona.definition, &persona.name, "night", hours_since);
                    self.deliver_message(persona, "good_night", msg);
                    Self::set_last_trigger(&state, &persona.id, "night", &now.to_rfc3339());
                    continue;
                }
            }

            if settings.miss_you_threshold_hours > 0
                && hours_since >= settings.miss_you_threshold_hours
                && rel.intimacy >= 15.0
            {
                let last = Self::get_last_trigger(&state, &persona.id, "miss");
                if Self::hours_since_str(last) >= 6 {
                    let msg = self.generate_greeting(&persona.definition, &persona.name, "miss", hours_since);
                    self.deliver_message(persona, "miss_you", msg);
                    Self::set_last_trigger(&state, &persona.id, "miss", &now.to_rfc3339());
                    continue;
                }
            }

            if settings.anniversary_reminder {
                let days = rel.days_known;
                if let Some(ann_msg) = Self::check_anniversary(days) {
                    let last = Self::get_last_trigger(&state, &persona.id, "anniv");
                    if Self::should_fire_today(last, &now) {
                        let msg = self.generate_greeting(&persona.definition, &persona.name, "random", hours_since);
                        self.deliver_message(persona, "anniversary", ann_msg + &msg);
                        Self::set_last_trigger(&state, &persona.id, "anniv", &now.to_rfc3339());
                    }
                }
            }

            if settings.random_checkins && rel.intimacy >= 20.0 {
                let last = Self::get_last_trigger(&state, &persona.id, "random");
                let interval = match settings.mode {
                    ProactiveMode::Quiet => 8,
                    ProactiveMode::Normal => 4,
                    ProactiveMode::Clingy => 2,
                };
                if Self::hours_since_str(last) >= interval {
                    let msg = self.generate_greeting(&persona.definition, &persona.name, "random", hours_since);
                    self.deliver_message(persona, "random_checkin", msg);
                    Self::set_last_trigger(&state, &persona.id, "random", &now.to_rfc3339());
                }
            }
        }
    }

    fn is_quiet_hour(settings: &ProactiveSettings) -> bool {
        let now = Local::now();
        let hour = now.hour();
        match (settings.quiet_hours_start, settings.quiet_hours_end) {
            (Some(s), Some(e)) => {
                if s > e {
                    hour >= s || hour < e
                } else {
                    hour >= s && hour < e
                }
            }
            _ => false,
        }
    }

    fn should_fire_today(last: Option<String>, now: &chrono::DateTime<Local>) -> bool {
        match last {
            None => true,
            Some(s) => match chrono::DateTime::parse_from_rfc3339(&s) {
                Ok(dt) => {
                    let dt_local: chrono::DateTime<Local> = dt.with_timezone(&Local);
                    dt_local.date_naive() != now.date_naive()
                }
                Err(_) => true,
            },
        }
    }

    fn hours_since_str(last: Option<String>) -> i64 {
        match last {
            None => 9999,
            Some(s) => match chrono::DateTime::parse_from_rfc3339(&s) {
                Ok(dt) => {
                    let now = Local::now();
                    let dt_local: chrono::DateTime<Local> = dt.with_timezone(&Local);
                    now.signed_duration_since(dt_local).num_hours()
                }
                Err(_) => 9999,
            },
        }
    }

    fn get_last_trigger(state: &AppState, persona_id: &str, kind: &str) -> Option<String> {
        let key = format!("last_proactive_{persona_id}_{kind}");
        state.db.with_conn(|conn| {
            conn.query_row(
                "SELECT value_json FROM settings WHERE key = ?1",
                [key],
                |row| {
                    let v: String = row.get(0)?;
                    serde_json::from_str(&v).map_err(|_| {
                        rusqlite::Error::InvalidQuery
                    })
                },
            )
            .optional()
        })
        .ok()
        .flatten()
    }

    fn set_last_trigger(state: &AppState, persona_id: &str, kind: &str, time: &str) {
        let key = format!("last_proactive_{persona_id}_{kind}");
        let json = serde_json::to_string(time).unwrap_or_default();
        let _ = state.db.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value_json, updated_at) VALUES (?1, ?2, datetime('now'))",
                rusqlite::params![key, json],
            )
        });
    }

    fn check_anniversary(days: u32) -> Option<String> {
        match days {
            1 => Some("嗨～我们已经认识一天啦！".to_string()),
            3 => Some("今天是我们认识的第三天呢～".to_string()),
            7 => Some("🎉 我们认识整整一周啦！".to_string()),
            14 => Some("两个星期了，谢谢你陪我～".to_string()),
            30 => Some("🌹 不知不觉一个月了呢。".to_string()),
            50 => Some("五十天纪念！我们越来越熟啦～".to_string()),
            100 => Some("💝 一百天了，这是属于我们的纪念日！".to_string()),
            180 => Some("半年了呢～".to_string()),
            365 => Some("❤️ 一周年快乐！谢谢你这一年的陪伴。".to_string()),
            _ if days > 0 && days % 100 == 0 => Some(format!("哇！已经{days}天了！")),
            _ => None,
        }
    }

    fn generate_greeting(
        &self,
        _persona: &crate::storage::models::PersonaDefinition,
        _persona_name: &str,
        kind: &str,
        hours_since: u32,
    ) -> String {
        let templates: &[&str] = match kind {
            "morning" => &[
                "早安呀～新的一天开始啦！今天也要开心哦☀️",
                "起床啦？睡的好吗？🌞",
                "早～你看今天天气（假装看了一眼）应该不错呢！",
                "早上好呀，我已经在等你啦～",
            ],
            "night" => &[
                "很晚啦，早点休息哦，熬夜对身体不好～🌙",
                "要睡觉了吗？晚安，做个好梦✨",
                "晚安呀～盖好被子别着凉了",
                "困了就去睡吧，我在这里陪着你",
            ],
            "miss" => &[
                "好久没找你聊天了，在忙什么呀？😢",
                "你已经好久没理我了...想你了",
                "在干嘛呢？我有点想你了",
                "忙完了吗？来陪我说说话嘛～",
            ],
            "random" | _ => &[
                "在干嘛呢？",
                "嘿嘿，突然想找你说说话",
                "今天过得怎么样呀？",
                "有没有什么有趣的事想跟我分享？",
                "你在吗～",
                "想你啦，来聊会儿天？",
            ],
        };

        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let idx = (seed as usize) % templates.len();

        let mut msg = templates[idx].to_string();

        if hours_since > 12 && kind == "miss" {
            msg = format!("{msg}\n（已经{hours_since}小时没说话了呢...）");
        }

        msg
    }

    fn deliver_message(&self, persona: &PersonaRow, trigger_type: &str, content: String) {
        let id = format!("pro_{}", uuid::Uuid::new_v4().simple());
        let msg = ProactiveMessage {
            id: id.clone(),
            persona_id: persona.id.clone(),
            persona_name: persona.name.clone(),
            trigger_type: trigger_type.to_string(),
            content: content.clone(),
            created_at: Local::now().to_rfc3339(),
            delivered: false,
        };

        if let Some(window) = self.app_handle.get_webview_window("main") {
            let _ = window.emit("proactive-message", &msg);
            let _ = window.show();
            let _ = window.set_focus();
        }

        let _ = self.app_handle.emit("proactive-message", &msg);

        log::info!("Proactive message [{trigger_type}] for {}: {content}", persona.name);
    }

    pub fn update_settings(&self, settings: ProactiveSettings) {
        *self.settings.lock().unwrap() = settings;
    }

    pub fn get_settings(&self) -> ProactiveSettings {
        self.settings.lock().unwrap().clone()
    }
}
