use crate::skill::registry::SkillRegistry;
use crate::storage::Database;
use anyhow::{Context, Result};
use serde_json::Value;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use uuid::Uuid;

/// 执行插件技能（mock，后续接入真实插件入口脚本）
async fn execute_plugin_skill(plugin_id: &str, skill_name: &str, args: &Value) -> Result<String> {
    log::info!("执行插件技能 '{skill_name}' (plugin_id={plugin_id})，当前返回 mock 结果");
    Ok(serde_json::json!({
        "status": "ok",
        "plugin_id": plugin_id,
        "skill": skill_name,
        "args": args,
        "message": "插件技能执行成功（mock）"
    })
    .to_string())
}

/// 执行技能并返回结果
pub async fn execute_skill(
    registry: &SkillRegistry,
    db: &Database,
    session_id: &str,
    skill_name: &str,
    args: serde_json::Value,
) -> Result<String> {
    let skill = registry
        .get(skill_name)
        .context(format!("Skill '{skill_name}' not found"))?;

    let start = std::time::Instant::now();
    let result = match skill.executor_type.as_str() {
        "rust" => {
            // Rust 内置技能
            crate::skill::builtin::execute_builtin(db, skill_name, &args).await?
        }
        "plugin" => {
            let plugin_id = skill.plugin_id.as_deref().unwrap_or("");
            execute_plugin_skill(plugin_id, skill_name, &args).await?
        }
        _ => {
            // Python 技能
            let skill_script = format!("skills/{}.py", skill_name);
            let mut child = Command::new(if cfg!(target_os = "windows") { "python" } else { "python3" })
                .arg(&skill_script)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .context(format!("spawn skill '{skill_name}'"))?;

            if let Some(mut stdin) = child.stdin.take() {
                let input = serde_json::to_string(&args)?;
                stdin.write_all(input.as_bytes())?;
            }

            let mut output = String::new();
            if let Some(mut stdout) = child.stdout.take() {
                stdout.read_to_string(&mut output)?;
            }

            child.wait()?;
            output.trim().to_string()
        }
    };

    // 记录调用日志
    let duration_ms = start.elapsed().as_millis() as i64;
    let log_id = format!("log_{}", Uuid::new_v4());
    db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO tool_call_log (id, session_id, skill_name, args_json, result_json, status, duration_ms) VALUES (?1, ?2, ?3, ?4, ?5, 'success', ?6)",
                rusqlite::params![log_id, session_id, skill_name, args.to_string(), result, duration_ms],
            )?;
            Ok(())
    })?;

    Ok(result)
}
