use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 插件权限
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    FsRead,
    FsWrite,
    Network,
    Shell,
    Clipboard,
}

impl PluginPermission {
    /// 将权限序列化为 snake_case 字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginPermission::FsRead => "fs_read",
            PluginPermission::FsWrite => "fs_write",
            PluginPermission::Network => "network",
            PluginPermission::Shell => "shell",
            PluginPermission::Clipboard => "clipboard",
        }
    }
}

/// 插件配置项定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfigField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub options: Option<Vec<PluginConfigOption>>,
    #[serde(default)]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfigOption {
    pub label: String,
    pub value: String,
}

/// 插件提供的技能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
}

/// 插件提供的命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub shortcut: Option<String>,
}

/// 插件清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub permissions: Vec<PluginPermission>,
    #[serde(default)]
    pub config: Option<Vec<PluginConfigField>>,
    #[serde(default)]
    pub skills: Option<Vec<PluginSkill>>,
    #[serde(default)]
    pub commands: Option<Vec<PluginCommand>>,
}

/// 解析插件目录下的 `plugin.json` 或 `manifest.json`
pub fn parse_manifest(path: &Path) -> Result<PluginManifest, String> {
    let plugin_json = path.join("plugin.json");
    let manifest_json = path.join("manifest.json");

    let manifest_path = if plugin_json.exists() {
        plugin_json
    } else if manifest_json.exists() {
        manifest_json
    } else {
        return Err(format!(
            "插件目录 '{}' 中未找到 plugin.json 或 manifest.json",
            path.display()
        ));
    };

    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("读取清单文件失败: {e}"))?;

    let manifest: PluginManifest = serde_json::from_str(&content)
        .map_err(|e| format!("解析清单文件失败: {e}"))?;

    Ok(manifest)
}

/// 校验插件清单的必填字段、权限、配置类型以及入口文件存在性
pub fn validate_manifest(manifest: &PluginManifest, plugin_dir: &Path) -> Result<(), String> {
    if manifest.id.trim().is_empty() {
        return Err("插件 id 不能为空".to_string());
    }
    if manifest.name.trim().is_empty() {
        return Err("插件 name 不能为空".to_string());
    }
    if manifest.version.trim().is_empty() {
        return Err("插件 version 不能为空".to_string());
    }
    if manifest.description.trim().is_empty() {
        return Err("插件 description 不能为空".to_string());
    }

    // id 只允许字母、数字、下划线、中划线
    if !manifest
        .id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!("插件 id '{}' 包含非法字符", manifest.id));
    }

    // 配置字段类型白名单
    let allowed_types = ["string", "number", "boolean", "select", "password"];
    if let Some(config) = &manifest.config {
        for field in config {
            if !allowed_types.contains(&field.r#type.as_str()) {
                return Err(format!(
                    "配置字段 '{}' 的类型 '{}' 不合法，必须是 {:?} 之一",
                    field.key, field.r#type, allowed_types
                ));
            }
        }
    }

    // 入口文件存在性校验
    if let Some(entry) = &manifest.entry {
        let entry_path = plugin_dir.join(entry);
        if !entry_path.exists() {
            return Err(format!(
                "入口文件 '{}' 不存在",
                entry_path.display()
            ));
        }
    }

    Ok(())
}
