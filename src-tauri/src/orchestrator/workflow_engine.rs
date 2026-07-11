use crate::skill::registry::SkillRegistry;
use crate::state::AppState;
use crate::storage::models::{Workflow, WorkflowAction, WorkflowTrigger};
use std::collections::HashMap;

/// 工作流执行中间上下文
pub struct WorkflowContext {
    pub session_id: String,
    pub persona_id: String,
    pub user_message: String,
    pub variables: HashMap<String, String>,
    pub action_results: Vec<ActionResult>,
}

impl WorkflowContext {
    pub fn new(session_id: String, persona_id: String, user_message: String) -> Self {
        Self {
            session_id,
            persona_id,
            user_message,
            variables: HashMap::new(),
            action_results: Vec::new(),
        }
    }
}

/// 单个动作执行结果
pub struct ActionResult {
    pub action_id: String,
    pub action_type: String,
    pub success: bool,
    pub result: String,
}

/// 触发工作流的事件
pub enum WorkflowEvent {
    Message { content: String },
    Scheduled,
    Event { name: String },
}

/// 判断工作流触发器是否匹配给定事件
pub fn match_trigger(workflow: &Workflow, event: &WorkflowEvent) -> bool {
    let trigger: WorkflowTrigger = match serde_json::from_value(workflow.trigger.clone()) {
        Ok(t) => t,
        Err(_) => return false,
    };

    match (event, trigger) {
        (WorkflowEvent::Message { content }, WorkflowTrigger::Message { pattern }) => {
            if let Some(pattern) = pattern {
                match regex::Regex::new(&pattern) {
                    Ok(re) => re.is_match(content),
                    Err(_) => false,
                }
            } else {
                true
            }
        }
        (WorkflowEvent::Scheduled, WorkflowTrigger::Scheduled { .. }) => true,
        (WorkflowEvent::Event { name }, WorkflowTrigger::Event { event_name }) => name == &event_name,
        _ => false,
    }
}

/// 顺序执行工作流的所有动作节点
pub async fn execute_workflow(
    state: &AppState,
    workflow: &Workflow,
    ctx: &mut WorkflowContext,
) -> Result<(), String> {
    let actions: Vec<WorkflowAction> = serde_json::from_value(workflow.actions.clone())
        .map_err(|e| format!("parse workflow actions: {e}"))?;

    let mut skill_registry = SkillRegistry::new();
    let _ = skill_registry.load_from_db(&state.db);

    for action in actions {
        let result = execute_action(state, ctx, &skill_registry, &action).await;
        let success = result.is_ok();
        let result_str = result.unwrap_or_else(|e| e);

        ctx.action_results.push(ActionResult {
            action_id: action.id.clone(),
            action_type: action.action_type.clone(),
            success,
            result: result_str.clone(),
        });

        if !success {
            log::warn!(
                "Workflow action {} ({}) failed: {}",
                action.id,
                action.action_type,
                result_str
            );
        }
    }

    Ok(())
}

async fn execute_action(
    state: &AppState,
    ctx: &mut WorkflowContext,
    skill_registry: &SkillRegistry,
    action: &WorkflowAction,
) -> Result<String, String> {
    match action.action_type.as_str() {
        "retrieve_memory" => {
            let memories = crate::memory::long_term::recall(
                &state.db,
                &*state.vector_store,
                &ctx.persona_id,
                &ctx.user_message,
                5,
            )
            .await
            .map_err(|e| e.to_string())?;
            let text = serde_json::to_string(&memories).unwrap_or_default();
            ctx.variables.insert("memory".to_string(), text.clone());
            Ok(text)
        }
        "query_knowledge" => {
            let knowledge = crate::rag::retriever::search(
                &state.db,
                &*state.vector_store,
                &ctx.user_message,
                None,
                3,
            )
            .await
            .map_err(|e| e.to_string())?;
            let text = serde_json::to_string(&knowledge).unwrap_or_default();
            ctx.variables.insert("knowledge".to_string(), text.clone());
            Ok(text)
        }
        "web_search" => {
            let query = action
                .config
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or(&ctx.user_message);
            let args = serde_json::json!({
                "query": query,
                "top_n": action.config.get("top_n").and_then(|v| v.as_u64()).unwrap_or(5),
                "time_range": action.config.get("time_range").and_then(|v| v.as_str()).unwrap_or(""),
            });
            let result = crate::skill::executor::execute_skill(
                skill_registry,
                &state.db,
                &ctx.session_id,
                "web_search",
                args,
            )
            .await
            .map_err(|e| e.to_string())?;
            ctx.variables.insert("search".to_string(), result.clone());
            Ok(result)
        }
        "call_skill" => {
            let skill_name = action
                .config
                .get("skill_name")
                .and_then(|v| v.as_str())
                .ok_or("call_skill missing skill_name")?;
            let args = action
                .config
                .get("arguments")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            let result = crate::skill::executor::execute_skill(
                skill_registry,
                &state.db,
                &ctx.session_id,
                skill_name,
                args,
            )
            .await
            .map_err(|e| e.to_string())?;
            ctx.variables.insert(skill_name.to_string(), result.clone());
            Ok(result)
        }
        "send_message" => {
            let text = action.config.get("text").and_then(|v| v.as_str()).unwrap_or("");
            ctx.variables.insert("send_message".to_string(), text.to_string());
            Ok(text.to_string())
        }
        "set_context" => {
            let key = action
                .config
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("set_context missing key")?;
            let value = action.config.get("value").and_then(|v| v.as_str()).unwrap_or("");
            ctx.variables.insert(key.to_string(), value.to_string());
            Ok(value.to_string())
        }
        _ => Err(format!("unknown action type: {}", action.action_type)),
    }
}

/// 将工作流上下文变量追加到 prompt 后
pub fn inject_workflow_context(prompt: &str, ctx: &WorkflowContext) -> String {
    if ctx.variables.is_empty() {
        return prompt.to_string();
    }

    let mut context_text = String::from("\n\n[工作流上下文]\n");
    for (key, value) in &ctx.variables {
        context_text.push_str(&format!("- {}: {}\n", key, value));
    }

    format!("{}{}", prompt, context_text)
}
