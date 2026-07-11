pub mod sync;

use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::storage::repo;
use crate::wechat::sync::WeChatSync;

pub struct WeChatManager {
    pub syncs: Arc<Mutex<HashMap<String, Arc<WeChatSync>>>>,
}

impl WeChatManager {
    pub fn new() -> Self {
        Self {
            syncs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn ensure_started(&self, account_id: &str, db: Arc<crate::storage::Database>, app: AppHandle) -> Arc<WeChatSync> {
        let mut map = self.syncs.lock().await;
        if let Some(s) = map.get(account_id) {
            return s.clone();
        }
        let sync = Arc::new(WeChatSync::new(account_id.to_string(), db, app));
        sync.start().await;
        map.insert(account_id.to_string(), sync.clone());
        sync
    }

    pub async fn stop(&self, account_id: &str) {
        let mut map = self.syncs.lock().await;
        if let Some(s) = map.remove(account_id) {
            s.stop().await;
        }
    }

    pub async fn restart(&self, account_id: &str, db: Arc<crate::storage::Database>, app: AppHandle) -> Arc<WeChatSync> {
        self.stop(account_id).await;
        self.ensure_started(account_id, db, app).await
    }

    /// 应用启动时调用：把所有数据库中 status=online 且 bot_token 非空的账号自动拉起 sync_loop
    pub async fn start_all_online_accounts(
        &self,
        db: Arc<crate::storage::Database>,
        app: AppHandle,
    ) -> usize {
        let ids: Vec<String> = db
            .with_conn(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id FROM wechat_account WHERE status = 'online' AND bot_token IS NOT NULL AND bot_token != ''",
                )?;
                let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
                rows.collect::<Result<Vec<_>, _>>()
            })
            .unwrap_or_default();

        let mut started = 0usize;
        for id in ids {
            match repo::get_wechat_bot_token(&db, &id) {
                Ok(Some(_)) => {
                    self.ensure_started(&id, db.clone(), app.clone()).await;
                    log::info!("wechat[{id}] 自动恢复 sync_loop 成功");
                    started += 1;
                }
                Ok(None) => {
                    log::warn!("wechat[{id}] status=online 但无 bot_token，跳过");
                }
                Err(e) => {
                    log::warn!("wechat[{id}] 读 bot_token 失败: {e}");
                }
            }
        }
        started
    }
}

impl Default for WeChatManager {
    fn default() -> Self {
        Self::new()
    }
}
