use crate::vector::store::VectorStore;
use crate::vector::sqlite_vec::BlobVecStore;
use crate::vector::qdrant::QdrantStore;
use std::sync::Arc;

/// 根据配置构造默认向量存储
pub fn create_vector_store(db: Arc<crate::storage::Database>, backend: &str) -> Arc<dyn VectorStore + Send + Sync> {
    match backend {
        "qdrant" => {
            Arc::new(QdrantStore::new("http://127.0.0.1:6333"))
        },
        _ => { // default: sqlite-vec blob storage
            let store = BlobVecStore::new(db);
            let _ = store.initialize();
            Arc::new(store)
        }
    }
}
