use crate::plugin::manifest::{parse_manifest, validate_manifest, PluginManifest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 已安装的插件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub id: String,
    pub manifest: PluginManifest,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub installed_at: String,
    pub updated_at: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginStateEntry {
    enabled: bool,
    config: serde_json::Value,
    installed_at: String,
    updated_at: String,
}

/// 插件注册表：管理插件目录、状态持久化
pub struct PluginRegistry {
    plugins_dir: PathBuf,
    state_path: PathBuf,
    plugins: Vec<InstalledPlugin>,
}

impl PluginRegistry {
    pub fn new(plugins_dir: PathBuf) -> Self {
        let state_path = plugins_dir.join("plugins.json");
        Self {
            plugins_dir,
            state_path,
            plugins: Vec::new(),
        }
    }

    /// 扫描 `plugins/` 目录并加载已安装插件
    pub fn load(&mut self) -> Result<(), String> {
        self.plugins.clear();
        fs::create_dir_all(&self.plugins_dir)
            .map_err(|e| format!("创建插件目录失败: {e}"))?;

        let mut state: HashMap<String, PluginStateEntry> = if self.state_path.exists() {
            let content = fs::read_to_string(&self.state_path)
                .map_err(|e| format!("读取插件状态失败: {e}"))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("解析插件状态失败: {e}"))?
        } else {
            HashMap::new()
        };

        for entry in fs::read_dir(&self.plugins_dir)
            .map_err(|e| format!("读取插件目录失败: {e}"))?
        {
            let entry = entry.map_err(|e| format!("读取插件目录项失败: {e}"))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let id = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let manifest = match parse_manifest(&path) {
                Ok(m) => m,
                Err(e) => {
                    log::warn!("跳过插件目录 '{}': {}", path.display(), e);
                    continue;
                }
            };

            if let Err(e) = validate_manifest(&manifest, &path) {
                log::warn!("跳过插件 '{}': {}", id, e);
                continue;
            }

            let mut state_entry = state.remove(&id).unwrap_or_else(|| {
                let now = now_rfc3339();
                PluginStateEntry {
                    enabled: true,
                    config: default_config(&manifest),
                    installed_at: now.clone(),
                    updated_at: now,
                }
            });

            // 如果清单新增了配置字段，补充默认值
            state_entry.config = merge_with_defaults(state_entry.config, &manifest);

            self.plugins.push(InstalledPlugin {
                id,
                manifest,
                enabled: state_entry.enabled,
                config: state_entry.config,
                installed_at: state_entry.installed_at,
                updated_at: state_entry.updated_at,
                path: path.to_string_lossy().to_string(),
            });
        }

        self.save_state()?;
        Ok(())
    }

    pub fn list_plugins(&self) -> Vec<InstalledPlugin> {
        self.plugins.clone()
    }

    pub fn get_plugin(&self, plugin_id: &str) -> Option<InstalledPlugin> {
        self.plugins.iter().find(|p| p.id == plugin_id).cloned()
    }

    /// 安装插件：将源目录复制到 `plugins/{id}` 并持久化状态
    pub fn install_plugin(&mut self, source_dir: &Path) -> Result<InstalledPlugin, String> {
        let manifest = parse_manifest(source_dir)?;
        validate_manifest(&manifest, source_dir)?;

        let id = manifest.id.clone();
        let target_dir = self.plugins_dir.join(&id);

        if target_dir.exists() {
            fs::remove_dir_all(&target_dir)
                .map_err(|e| format!("删除旧版本插件目录失败: {e}"))?;
        }

        copy_dir_all(source_dir, &target_dir)
            .map_err(|e| format!("复制插件文件失败: {e}"))?;

        let now = now_rfc3339();
        let manifest = parse_manifest(&target_dir)?;
        let installed = InstalledPlugin {
            id: id.clone(),
            manifest: manifest.clone(),
            enabled: true,
            config: default_config(&manifest),
            installed_at: now.clone(),
            updated_at: now,
            path: target_dir.to_string_lossy().to_string(),
        };

        self.plugins.retain(|p| p.id != id);
        self.plugins.push(installed.clone());
        self.save_state()?;
        Ok(installed)
    }

    /// 卸载插件：删除 `plugins/{plugin_id}` 目录并移除状态
    pub fn uninstall_plugin(&mut self, plugin_id: &str) -> Result<(), String> {
        let target_dir = self.plugins_dir.join(plugin_id);
        if target_dir.exists() {
            fs::remove_dir_all(&target_dir)
                .map_err(|e| format!("删除插件目录失败: {e}"))?;
        }
        self.plugins.retain(|p| p.id != plugin_id);
        self.save_state()?;
        Ok(())
    }

    pub fn set_enabled(
        &mut self,
        plugin_id: &str,
        enabled: bool,
    ) -> Result<InstalledPlugin, String> {
        let plugin = self
            .plugins
            .iter_mut()
            .find(|p| p.id == plugin_id)
            .ok_or_else(|| format!("插件 '{}' 不存在", plugin_id))?;
        plugin.enabled = enabled;
        plugin.updated_at = now_rfc3339();
        let cloned = plugin.clone();
        self.save_state()?;
        Ok(cloned)
    }

    pub fn set_config(
        &mut self,
        plugin_id: &str,
        config: serde_json::Value,
    ) -> Result<InstalledPlugin, String> {
        let plugin = self
            .plugins
            .iter_mut()
            .find(|p| p.id == plugin_id)
            .ok_or_else(|| format!("插件 '{}' 不存在", plugin_id))?;

        plugin.config = if let Some(obj) = config.as_object() {
            serde_json::Value::Object(obj.clone())
        } else {
            return Err("配置必须是一个 JSON 对象".to_string());
        };
        plugin.config = merge_with_defaults(plugin.config.clone(), &plugin.manifest);
        plugin.updated_at = now_rfc3339();
        let cloned = plugin.clone();
        self.save_state()?;
        Ok(cloned)
    }

    fn save_state(&self) -> Result<(), String> {
        let mut map: HashMap<String, PluginStateEntry> = HashMap::new();
        for plugin in &self.plugins {
            map.insert(
                plugin.id.clone(),
                PluginStateEntry {
                    enabled: plugin.enabled,
                    config: plugin.config.clone(),
                    installed_at: plugin.installed_at.clone(),
                    updated_at: plugin.updated_at.clone(),
                },
            );
        }
        let content = serde_json::to_string_pretty(&map)
            .map_err(|e| format!("序列化插件状态失败: {e}"))?;
        fs::write(&self.state_path, content)
            .map_err(|e| format!("写入插件状态失败: {e}"))?;
        Ok(())
    }
}

fn default_config(manifest: &PluginManifest) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    if let Some(fields) = &manifest.config {
        for field in fields {
            if let Some(default) = &field.default {
                map.insert(field.key.clone(), default.clone());
            }
        }
    }
    serde_json::Value::Object(map)
}

fn merge_with_defaults(config: serde_json::Value, manifest: &PluginManifest) -> serde_json::Value {
    let mut map = if let Some(obj) = config.as_object() {
        obj.clone()
    } else {
        serde_json::Map::new()
    };
    if let Some(fields) = &manifest.config {
        for field in fields {
            if !map.contains_key(&field.key) {
                if let Some(default) = &field.default {
                    map.insert(field.key.clone(), default.clone());
                }
            }
        }
    }
    serde_json::Value::Object(map)
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
