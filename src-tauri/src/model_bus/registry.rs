use crate::model_bus::scheduler::ModelScheduler;
use crate::storage::Database;
use anyhow::Result;
use std::sync::Arc;

/// 从 DB 加载活跃模型配置并注册 Provider
pub async fn load_active_providers(
    db: &Database,
    scheduler: &ModelScheduler,
    ollama_provider: Arc<crate::model_bus::ollama::OllamaProvider>,
) -> Result<()> {
    // 注册 Ollama Provider（默认，文本+图像+工具）
    scheduler.register(ollama_provider.clone() as Arc<dyn crate::model_bus::provider::ModelProvider + Send + Sync>).await;

    // 从 model_config 表加载其他 LLM Provider
    let providers: Vec<(String, String, String)> = db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, provider_id, status FROM model_config WHERE model_type = 'llm' AND is_active = 1"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
    })?;

    for (id, provider_id, status) in providers {
        if status == "active" && provider_id.starts_with("ollama/") {
            let model_name = provider_id.trim_start_matches("ollama/");
            let provider = crate::model_bus::ollama::OllamaProvider::new(model_name);
            scheduler
                .register(Arc::new(provider) as Arc<dyn crate::model_bus::provider::ModelProvider + Send + Sync>)
                .await;
            scheduler.set_active(&id).await;
        }
    }

    Ok(())
}
