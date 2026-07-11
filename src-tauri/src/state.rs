use crate::plugin::registry::PluginRegistry;
use crate::storage::Database;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

/// 应用全局状态，由 Tauri 管理
pub struct AppState {
    pub db: Arc<Database>,
    pub data_dir: PathBuf,
    pub model_scheduler: Arc<crate::model_bus::scheduler::ModelScheduler>,
    pub worker_pool: Arc<crate::worker::pool::WorkerPool>,
    pub vector_store: Arc<dyn crate::vector::store::VectorStore + Send + Sync>,
    pub wechat_manager: Arc<crate::wechat::WeChatManager>,
    pub plugin_registry: Arc<RwLock<PluginRegistry>>,
    pub memory_extractor: Arc<crate::memory::extractor::MemoryExtractor>,
    /// 正在运行的下载进程：model_id -> child PID
    pub active_downloads: Arc<Mutex<HashMap<String, u32>>>,
}
