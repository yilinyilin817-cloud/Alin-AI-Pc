use crate::plugin::registry::PluginRegistry;
use crate::storage::Database;
use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;

/// 技能定义
#[derive(Debug, Clone, Serialize)]
pub struct SkillDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub permissions: Vec<String>,
    pub approval_mode: String,
    pub executor_type: String,
    pub enabled: bool,
    pub plugin_id: Option<String>,
}

/// 技能注册表
pub struct SkillRegistry {
    skills: HashMap<String, SkillDefinition>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// 从 DB 加载技能
    pub fn load_from_db(&mut self, db: &Database) -> Result<()> {
        let rows: Vec<(String, String, bool, String, String)> = db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, enabled, definition_yaml, config_json FROM skill WHERE enabled = 1"
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()
        })?;

        for (_id, name, enabled, yaml_str, _config) in rows {
            if !enabled { continue; }
            if let Ok(def) = parse_yaml_skill(&name, &yaml_str) {
                self.skills.insert(name, def);
            }
        }
        Ok(())
    }

    /// 从已启用的插件中注册技能
    pub fn load_plugins(&mut self, plugin_registry: &PluginRegistry) {
        for plugin in plugin_registry.list_plugins() {
            if !plugin.enabled {
                continue;
            }
            if let Some(skills) = &plugin.manifest.skills {
                for skill in skills {
                    let parameters = skill.arguments.clone().unwrap_or_else(|| {
                        serde_json::json!({
                            "type": "object",
                            "properties": {},
                            "required": []
                        })
                    });
                    let permissions: Vec<String> = plugin
                        .manifest
                        .permissions
                        .iter()
                        .map(|p| p.as_str().to_string())
                        .collect();
                    self.skills.insert(
                        skill.id.clone(),
                        SkillDefinition {
                            name: skill.id.clone(),
                            description: skill.description.clone(),
                            parameters,
                            permissions,
                            approval_mode: "once".to_string(),
                            executor_type: "plugin".to_string(),
                            enabled: true,
                            plugin_id: Some(plugin.id.clone()),
                        },
                    );
                }
            }
        }
    }

    /// 加载 DB 技能与插件技能
    pub fn load_all(&mut self, db: &Database, plugin_registry: &PluginRegistry) -> Result<()> {
        self.load_from_db(db)?;
        self.load_plugins(plugin_registry);
        Ok(())
    }

    /// 手动注册技能
    pub fn register(&mut self, def: SkillDefinition) {
        self.skills.insert(def.name.clone(), def);
    }

    /// 获取技能定义
    pub fn get(&self, name: &str) -> Option<&SkillDefinition> {
        self.skills.get(name)
    }

    /// 获取角色已启用的技能 schema（用于 LLM function calling）
    pub fn enabled_schemas(&self, skill_names: &[String]) -> Vec<crate::model_bus::provider::ToolSchema> {
        skill_names
            .iter()
            .filter_map(|name| {
                self.skills.get(name).map(|def| crate::model_bus::provider::ToolSchema {
                    name: def.name.clone(),
                    description: def.description.clone(),
                    parameters: def.parameters.clone(),
                })
            })
            .collect()
    }

    /// 列出所有技能名
    pub fn list_names(&self) -> Vec<String> {
        self.skills.keys().cloned().collect()
    }
}

fn parse_yaml_skill(name: &str, yaml_str: &str) -> Result<SkillDefinition> {
    // 简易 YAML 解析（无 serde_yaml 依赖时用简易语法）
    // 支持 miniyaml 格式：key: value\n  - list
    // 完整 YAML 解析需 serde_yaml 或 yaml-rust2
    let mut def = SkillDefinition {
        name: name.to_string(),
        description: String::new(),
        parameters: serde_json::json!({}),
        permissions: vec![],
        approval_mode: "once".to_string(),
        executor_type: "python".to_string(),
        enabled: true,
        plugin_id: None,
    };

    let mut in_params = false;
    let mut param_indent = 0usize;
    let mut current_param: Option<std::collections::HashMap<String, String>> = None;
    let mut param_items: Vec<std::collections::HashMap<String, String>> = Vec::new();

    for raw_line in yaml_str.lines() {
        let line = raw_line.trim_end();
        if line.trim().is_empty() || line.trim().starts_with('#') {
            continue;
        }
        let leading_spaces = line.len() - line.trim_start().len();
        let trimmed = line.trim();

        if trimmed.starts_with("parameters:") {
            if let Some(p) = current_param.take() {
                param_items.push(p);
            }
            in_params = true;
            continue;
        }

        // 顶层其他字段：退出 parameters 解析
        let is_top_level_field = trimmed.starts_with("name:")
            || trimmed.starts_with("description:")
            || trimmed.starts_with("permissions:")
            || trimmed.starts_with("approval_mode:")
            || trimmed.starts_with("executor_type:");

        if in_params && is_top_level_field {
            if let Some(p) = current_param.take() {
                param_items.push(p);
            }
            in_params = false;
        }

        if in_params {
            if trimmed.starts_with("- name:") {
                if let Some(p) = current_param.take() {
                    param_items.push(p);
                }
                let mut p = std::collections::HashMap::new();
                let v = trimmed.strip_prefix("- name:").unwrap().trim().trim_matches('"').trim_matches('\'').to_string();
                p.insert("name".to_string(), v);
                param_indent = leading_spaces;
                current_param = Some(p);
            } else if current_param.is_some() && leading_spaces > param_indent {
                if let Some((k, v)) = trimmed.split_once(':') {
                    let k = k.trim().to_string();
                    let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
                    current_param.as_mut().unwrap().insert(k, v);
                }
            }
            continue;
        }

        if let Some(desc) = trimmed.strip_prefix("description:") {
            def.description = desc.trim().trim_matches('"').trim_matches('\'').to_string();
        } else if let Some(perm) = trimmed.strip_prefix("permissions: [") {
            if let Some(end) = perm.find(']') {
                def.permissions = perm[..end]
                    .split(',')
                    .map(|s| s.trim().trim_matches('\'').trim_matches('"').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        } else if let Some(mode) = trimmed.strip_prefix("approval_mode:") {
            def.approval_mode = mode.trim().trim_matches('"').trim_matches('\'').to_string();
        } else if let Some(et) = trimmed.strip_prefix("executor_type:") {
            def.executor_type = et.trim().trim_matches('"').trim_matches('\'').to_string();
        }
    }

    if in_params {
        if let Some(p) = current_param.take() {
            param_items.push(p);
        }
    }

    def.parameters = build_json_schema(&param_items);
    Ok(def)
}

fn build_json_schema(items: &[std::collections::HashMap<String, String>]) -> serde_json::Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    for item in items {
        let name = item.get("name").cloned().unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let typ = item.get("type").cloned().unwrap_or_else(|| "string".to_string());
        let desc = item.get("description").cloned().unwrap_or_default();
        let is_required = item.get("required").map(|s| s == "true").unwrap_or(false);

        let json_type = match typ.as_str() {
            "integer" => "integer",
            "number" => "number",
            "boolean" => "boolean",
            "array" => "array",
            "object" => "object",
            _ => "string",
        };

        let mut prop = serde_json::json!({ "type": json_type });
        if !desc.is_empty() {
            prop["description"] = serde_json::Value::String(desc);
        }
        properties.insert(name.clone(), prop);

        if is_required {
            required.push(serde_json::Value::String(name));
        }
    }

    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}
