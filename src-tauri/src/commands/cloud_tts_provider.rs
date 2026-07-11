use crate::model_bus::wusound::{
    WusoundProvider, WusoundQuota, WusoundSynthesizeOptions, WusoundVoice,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudTtsProviderConfig {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub api_base: String,
    /// 脱敏后的 API Key（前 4 后 4）。前端如要修改应单独传全量
    pub api_key_masked: String,
    /// 是否已配置 API Key
    pub has_api_key: bool,
    pub icon_url: Option<String>,
    pub voices: Vec<WusoundVoice>,
    pub is_enabled: bool,
    pub last_verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCloudTtsProviderRequest {
    pub name: String,
    pub provider_type: String,
    pub api_base: String,
    pub api_key: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCloudTtsProviderRequest {
    pub name: Option<String>,
    pub provider_type: Option<String>,
    pub api_base: Option<String>,
    pub api_key: Option<String>,
    pub icon_url: Option<String>,
    pub is_enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct VerifyTtsResponse {
    pub success: bool,
    pub voices: Vec<WusoundVoice>,
    pub message: String,
    pub quota: Option<WusoundQuota>,
    pub verified_at: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct SynthesizeRequest {
    pub provider_id: String,
    pub text: String,
    pub voice_id: String,
    pub prompt_id: Option<String>,
    pub speed: Option<f32>,
    pub pitch: Option<f32>,
    pub volume: Option<f32>,
    pub format: Option<String>,
    pub sample_rate: Option<u32>,
}

fn mask_key(key: &str) -> String {
    if key.is_empty() {
        return String::new();
    }
    let len = key.chars().count();
    if len <= 8 {
        return "****".into();
    }
    let head: String = key.chars().take(4).collect();
    let tail: String = key.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{head}…{tail}")
}

fn row_to_config(row: &rusqlite::Row<'_>) -> rusqlite::Result<CloudTtsProviderConfig> {
    let voices_json: String = row.get(6).unwrap_or_else(|_| "[]".to_string());
    let voices: Vec<WusoundVoice> = serde_json::from_str(&voices_json).unwrap_or_default();
    let api_key: String = row.get(4)?;
    Ok(CloudTtsProviderConfig {
        id: row.get(0)?,
        name: row.get(1)?,
        provider_type: row.get(2)?,
        api_base: row.get(3)?,
        api_key_masked: mask_key(&api_key),
        has_api_key: !api_key.is_empty(),
        icon_url: row.get(5)?,
        voices,
        is_enabled: row.get::<_, i64>(7)? != 0,
        last_verified_at: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

const SELECT_CONFIG_SQL: &str = "SELECT id, name, provider_type, api_base, api_key, icon_url, voices_json, is_enabled, last_verified_at, created_at, updated_at FROM cloud_tts_provider";

/// 列出所有云端 TTS 服务商
#[tauri::command]
pub async fn list_cloud_tts_providers(
    state: State<'_, AppState>,
) -> Result<Vec<CloudTtsProviderConfig>, String> {
    state
        .db
        .with_conn(|conn| {
            let sql = format!("{SELECT_CONFIG_SQL} ORDER BY created_at DESC");
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map([], row_to_config)?;
            rows.collect::<Result<Vec<_>, _>>()
        })
        .map_err(|e| e.to_string())
}

/// 获取单个云端 TTS 服务商
#[tauri::command]
pub async fn get_cloud_tts_provider(
    state: State<'_, AppState>,
    id: String,
) -> Result<CloudTtsProviderConfig, String> {
    state
        .db
        .with_conn(|conn| {
            let sql = format!("{SELECT_CONFIG_SQL} WHERE id = ?1");
            conn.query_row(&sql, rusqlite::params![id], row_to_config)
        })
        .map_err(|e| e.to_string())
}

/// 创建云端 TTS 服务商
#[tauri::command]
pub async fn create_cloud_tts_provider(
    state: State<'_, AppState>,
    request: CreateCloudTtsProviderRequest,
) -> Result<CloudTtsProviderConfig, String> {
    if request.name.trim().is_empty() {
        return Err("名称不能为空".into());
    }
    if request.api_base.trim().is_empty() {
        return Err("API 地址不能为空".into());
    }
    if request.api_key.trim().is_empty() {
        return Err("API Key 不能为空".into());
    }
    if !request.api_base.starts_with("http://") && !request.api_base.starts_with("https://") {
        return Err("API 地址必须以 http:// 或 https:// 开头".into());
    }

    let id = format!("cloud_tts_{}", Uuid::new_v4());
    let voices_json = serde_json::to_string(&Vec::<WusoundVoice>::new()).unwrap();

    state
        .db
        .with_conn(|conn| {
            conn.execute(
                "INSERT INTO cloud_tts_provider (id, name, provider_type, api_base, api_key, icon_url, voices_json, is_enabled) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
                rusqlite::params![id, request.name, request.provider_type, request.api_base, request.api_key, request.icon_url, voices_json],
            )?;

            let sql = format!("{SELECT_CONFIG_SQL} WHERE id = ?1");
            conn.query_row(&sql, rusqlite::params![id], row_to_config)
        })
        .map_err(|e| e.to_string())
}

/// 更新云端 TTS 服务商
#[tauri::command]
pub async fn update_cloud_tts_provider(
    state: State<'_, AppState>,
    id: String,
    request: UpdateCloudTtsProviderRequest,
) -> Result<CloudTtsProviderConfig, String> {
    state
        .db
        .with_conn(|conn| {
            // 构建动态更新语句
            let mut updates = Vec::new();
            let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

            if let Some(name) = request.name {
                if name.trim().is_empty() {
                    return Err(rusqlite::Error::InvalidParameterName("名称不能为空".into()));
                }
                updates.push("name = ?");
                params.push(Box::new(name));
            }
            if let Some(provider_type) = request.provider_type {
                updates.push("provider_type = ?");
                params.push(Box::new(provider_type));
            }
            if let Some(api_base) = request.api_base {
                if !api_base.starts_with("http://") && !api_base.starts_with("https://") {
                    return Err(rusqlite::Error::InvalidParameterName(
                        "API 地址必须以 http:// 或 https:// 开头".into(),
                    ));
                }
                updates.push("api_base = ?");
                params.push(Box::new(api_base));
            }
            if let Some(api_key) = request.api_key {
                if api_key.trim().is_empty() {
                    return Err(rusqlite::Error::InvalidParameterName("API Key 不能为空".into()));
                }
                updates.push("api_key = ?");
                params.push(Box::new(api_key));
            }
            if let Some(icon_url) = request.icon_url {
                updates.push("icon_url = ?");
                params.push(Box::new(icon_url));
            }
            if let Some(is_enabled) = request.is_enabled {
                updates.push("is_enabled = ?");
                params.push(Box::new(if is_enabled { 1 } else { 0 }));
            }

            if updates.is_empty() {
                return Err(rusqlite::Error::InvalidParameterName(
                    "没有可更新的字段".into(),
                ));
            }

            updates.push("updated_at = datetime('now')");
            params.push(Box::new(id.clone()));

            let sql = format!(
                "UPDATE cloud_tts_provider SET {} WHERE id = ?",
                updates.join(", ")
            );

            let params_refs: Vec<&dyn rusqlite::types::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();
            conn.execute(&sql, params_refs.as_slice())?;

            let select_sql = format!("{SELECT_CONFIG_SQL} WHERE id = ?1");
            conn.query_row(&select_sql, rusqlite::params![id], row_to_config)
        })
        .map_err(|e| e.to_string())
}

/// 删除云端 TTS 服务商
#[tauri::command]
pub async fn delete_cloud_tts_provider(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state
        .db
        .with_conn(|conn| {
            conn.execute(
                "DELETE FROM cloud_tts_provider WHERE id = ?1",
                rusqlite::params![id],
            )?;
            Ok(())
        })
        .map_err(|e| e.to_string())
}

fn load_provider_config(
    state: &AppState,
    id: &str,
) -> Result<(String, String, String), String> {
    state
        .db
        .with_conn(|conn| {
            conn.query_row(
                "SELECT name, api_base, api_key FROM cloud_tts_provider WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
        })
        .map_err(|e| e.to_string())
}

/// 验证云端 TTS 服务商连接并获取语音角色列表 + 配额
#[tauri::command]
pub async fn verify_cloud_tts_provider(
    state: State<'_, AppState>,
    id: String,
) -> Result<VerifyTtsResponse, String> {
    let (name, api_base, api_key) = load_provider_config(&state, &id)?;

    let provider = WusoundProvider::new(&id, &name, &api_base, &api_key);
    let verified_at = chrono::Utc::now().to_rfc3339();

    let verify_res = provider.verify().await;
    let quota_res = provider.quota().await.unwrap_or(None);

    match verify_res {
        Ok(voices) => {
            let voice_count = voices.len();
            let voices_json = serde_json::to_string(&voices).unwrap();
            state
                .db
                .with_conn(|conn| {
                    conn.execute(
                        "UPDATE cloud_tts_provider SET voices_json = ?1, last_verified_at = ?2, updated_at = datetime('now') WHERE id = ?3",
                        rusqlite::params![voices_json, verified_at, id],
                    )?;
                    Ok(())
                })
                .map_err(|e| e.to_string())?;

            let mut message = format!("验证成功，发现 {voice_count} 个语音角色");
            if let Some(ref q) = quota_res {
                if q.total_chars > 0 {
                    message.push_str(&format!(
                        "，剩余 {} / {} 字符",
                        q.remaining_chars, q.total_chars
                    ));
                }
            }

            Ok(VerifyTtsResponse {
                success: true,
                voices,
                message,
                quota: quota_res,
                verified_at,
            })
        }
        Err(e) => Ok(VerifyTtsResponse {
            success: false,
            voices: vec![],
            message: format!("验证失败: {e}"),
            quota: quota_res,
            verified_at,
        }),
    }
}

/// 查询云端 TTS 服务商配额
#[tauri::command]
pub async fn check_cloud_tts_quota(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<WusoundQuota>, String> {
    let (name, api_base, api_key) = load_provider_config(&state, &id)?;
    let provider = WusoundProvider::new(&id, &name, &api_base, &api_key);
    provider.quota().await
}

/// TTS 合成（使用云端 TTS 服务商）
#[tauri::command]
pub async fn cloud_tts_synthesize(
    state: State<'_, AppState>,
    request: SynthesizeRequest,
) -> Result<Vec<u8>, String> {
    if request.text.trim().is_empty() {
        return Err("合成文本不能为空".into());
    }
    if request.voice_id.trim().is_empty() {
        return Err("必须选择 voice".into());
    }

    let (name, api_base, api_key) = load_provider_config(&state, &request.provider_id)?;
    let provider = WusoundProvider::new(&request.provider_id, &name, &api_base, &api_key);

    let options = WusoundSynthesizeOptions {
        speed: request.speed,
        pitch: request.pitch,
        volume: request.volume,
        format: request.format,
        sample_rate: request.sample_rate,
    };

    provider
        .synthesize(
            &request.text,
            &request.voice_id,
            request.prompt_id.as_deref(),
            Some(options),
        )
        .await
}

/// 试听合成（与 cloud_tts_synthesize 行为相同，但前端语义区分）
#[tauri::command]
pub async fn cloud_tts_preview(
    state: State<'_, AppState>,
    request: SynthesizeRequest,
) -> Result<Vec<u8>, String> {
    cloud_tts_synthesize(state, request).await
}
