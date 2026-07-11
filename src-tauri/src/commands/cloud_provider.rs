use crate::model_bus::cloud::{CloudModel, CloudProvider};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudProviderConfig {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub api_base: String,
    pub api_key: String,
    pub icon_url: Option<String>,
    pub models: Vec<CloudModel>,
    pub is_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCloudProviderRequest {
    pub name: String,
    pub provider_type: String,
    pub api_base: String,
    pub api_key: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCloudProviderRequest {
    pub name: Option<String>,
    pub provider_type: Option<String>,
    pub api_base: Option<String>,
    pub api_key: Option<String>,
    pub icon_url: Option<String>,
    pub is_enabled: Option<bool>,
}

/// 列出所有云服务商
#[tauri::command]
pub async fn list_cloud_providers(state: State<'_, AppState>) -> Result<Vec<CloudProviderConfig>, String> {
    state
        .db
        .with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, provider_type, api_base, api_key, icon_url, models_json, is_enabled, created_at, updated_at FROM cloud_provider ORDER BY created_at DESC",
            )?;

            let rows = stmt.query_map([], |row| {
                let models_json: String = row.get(6).unwrap_or_else(|_| "[]".to_string());
                let models: Vec<CloudModel> =
                    serde_json::from_str(&models_json).unwrap_or_default();

                Ok(CloudProviderConfig {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    provider_type: row.get(2)?,
                    api_base: row.get(3)?,
                    api_key: row.get(4)?,
                    icon_url: row.get(5)?,
                    models,
                    is_enabled: row.get::<_, i64>(7)? != 0,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })?;

            rows.collect::<Result<Vec<_>, _>>()
        })
        .map_err(|e| e.to_string())
}

/// 创建云服务商
#[tauri::command]
pub async fn create_cloud_provider(
    state: State<'_, AppState>,
    req: CreateCloudProviderRequest,
) -> Result<CloudProviderConfig, String> {
    let id = format!("cloud_{}", Uuid::new_v4());
    let models_json = serde_json::to_string(&Vec::<CloudModel>::new()).unwrap();

    state
        .db
        .with_conn(|conn| {
            conn.execute(
                "INSERT INTO cloud_provider (id, name, provider_type, api_base, api_key, icon_url, models_json, is_enabled) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
                rusqlite::params![id, req.name, req.provider_type, req.api_base, req.api_key, req.icon_url, models_json],
            )?;

            conn.query_row(
                "SELECT id, name, provider_type, api_base, api_key, icon_url, models_json, is_enabled, created_at, updated_at FROM cloud_provider WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    let models_json: String = row.get(6).unwrap_or_else(|_| "[]".to_string());
                    let models: Vec<CloudModel> =
                        serde_json::from_str(&models_json).unwrap_or_default();

                    Ok(CloudProviderConfig {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        provider_type: row.get(2)?,
                        api_base: row.get(3)?,
                        api_key: row.get(4)?,
                        icon_url: row.get(5)?,
                        models,
                        is_enabled: row.get::<_, i64>(7)? != 0,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                },
            )
        })
        .map_err(|e| e.to_string())
}

/// 更新云服务商
#[tauri::command]
pub async fn update_cloud_provider(
    state: State<'_, AppState>,
    id: String,
    req: UpdateCloudProviderRequest,
) -> Result<CloudProviderConfig, String> {
    state
        .db
        .with_conn(|conn| {
            // 构建动态更新语句
            let mut updates = Vec::new();
            let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

            if let Some(name) = req.name {
                updates.push("name = ?");
                params.push(Box::new(name));
            }
            if let Some(provider_type) = req.provider_type {
                updates.push("provider_type = ?");
                params.push(Box::new(provider_type));
            }
            if let Some(api_base) = req.api_base {
                updates.push("api_base = ?");
                params.push(Box::new(api_base));
            }
            if let Some(api_key) = req.api_key {
                updates.push("api_key = ?");
                params.push(Box::new(api_key));
            }
            if let Some(icon_url) = req.icon_url {
                updates.push("icon_url = ?");
                params.push(Box::new(icon_url));
            }
            if let Some(is_enabled) = req.is_enabled {
                updates.push("is_enabled = ?");
                params.push(Box::new(if is_enabled { 1 } else { 0 }));
            }

            if updates.is_empty() {
                return Err(rusqlite::Error::InvalidParameterName(
                    "No fields to update".to_string(),
                ));
            }

            updates.push("updated_at = datetime('now')");
            params.push(Box::new(id.clone()));

            let sql = format!(
                "UPDATE cloud_provider SET {} WHERE id = ?",
                updates.join(", ")
            );

            let params_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
            conn.execute(&sql, params_refs.as_slice())?;

            conn.query_row(
                "SELECT id, name, provider_type, api_base, api_key, icon_url, models_json, is_enabled, created_at, updated_at FROM cloud_provider WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    let models_json: String = row.get(6).unwrap_or_else(|_| "[]".to_string());
                    let models: Vec<CloudModel> =
                        serde_json::from_str(&models_json).unwrap_or_default();

                    Ok(CloudProviderConfig {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        provider_type: row.get(2)?,
                        api_base: row.get(3)?,
                        api_key: row.get(4)?,
                        icon_url: row.get(5)?,
                        models,
                        is_enabled: row.get::<_, i64>(7)? != 0,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                },
            )
        })
        .map_err(|e| e.to_string())
}

/// 删除云服务商
#[tauri::command]
pub async fn delete_cloud_provider(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .db
        .with_conn(|conn| {
            conn.execute("DELETE FROM cloud_provider WHERE id = ?1", rusqlite::params![id])?;
            Ok(())
        })
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub success: bool,
    pub models: Vec<CloudModel>,
    pub message: String,
}

/// 验证云服务商连接并获取模型列表
#[tauri::command]
pub async fn verify_cloud_provider(
    state: State<'_, AppState>,
    id: String,
) -> Result<VerifyResponse, String> {
    // 获取云服务商配置
    let config = state
        .db
        .with_conn(|conn| {
            conn.query_row(
                "SELECT name, api_base, api_key FROM cloud_provider WHERE id = ?1",
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
        .map_err(|e| e.to_string())?;

    let (name, api_base, api_key) = config;

    // 创建临时 CloudProvider 进行验证
    let provider = CloudProvider::new(&id, &name, &api_base, &api_key, "");

    match provider.verify().await {
        Ok(models) => {
            // 更新数据库中的模型列表
            let models_json = serde_json::to_string(&models).unwrap();
            state
                .db
                .with_conn(|conn| {
                    conn.execute(
                        "UPDATE cloud_provider SET models_json = ?1, updated_at = datetime('now') WHERE id = ?2",
                        rusqlite::params![models_json, id],
                    )?;
                    Ok(())
                })
                .map_err(|e| e.to_string())?;

            Ok(VerifyResponse {
                success: true,
                message: format!("验证成功，发现 {} 个模型", models.len()),
                models,
            })
        }
        Err(e) => Ok(VerifyResponse {
            success: false,
            models: vec![],
            message: format!("验证失败: {}", e),
        }),
    }
}

/// 同步云服务商模型到 model_config 表
#[tauri::command]
pub async fn sync_cloud_models(state: State<'_, AppState>, provider_id: String) -> Result<i32, String> {
    // 获取云服务商配置
    let config = state
        .db
        .with_conn(|conn| {
            conn.query_row(
                "SELECT name, provider_type, api_base, api_key, models_json FROM cloud_provider WHERE id = ?1",
                rusqlite::params![provider_id],
                |row| {
                    let models_json: String = row.get(4).unwrap_or_else(|_| "[]".to_string());
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        models_json,
                    ))
                },
            )
        })
        .map_err(|e| e.to_string())?;

    let (name, _provider_type, _api_base, _api_key, models_json) = config;
    let models: Vec<CloudModel> = serde_json::from_str(&models_json).unwrap_or_default();

    let mut synced_count = 0;

    for model in models {
        let model_id = format!("cloud_{}_{}", provider_id, model.id.replace("/", "_"));
        let provider_id_full = format!("cloud/{}/{}", provider_id, model.id);

        // 检查是否已存在
        let mut exists = 0;
        let _ = state.db.with_conn(|conn| {
            exists = conn.query_row(
                "SELECT COUNT(*) FROM model_config WHERE id = ?1",
                rusqlite::params![model_id],
                |row| row.get::<_, i64>(0),
            ).unwrap_or(0);
            Ok::<(), rusqlite::Error>(())
        });

        if exists == 0 {
            // 插入新模型
            let _ = state.db.with_conn(|conn| {
                conn.execute(
                    "INSERT INTO model_config (id, name, model_type, provider_id, status, is_active) VALUES (?1, ?2, 'llm', ?3, 'downloaded', 0)",
                    rusqlite::params![model_id, model.id, provider_id_full],
                )?;
                Ok::<(), rusqlite::Error>(())
            });

            synced_count += 1;
        }
    }

    Ok(synced_count)
}
