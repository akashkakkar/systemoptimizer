//! SQLCipher-backed [`ExplanationCache`] for F16.
//!
//! SQLite work is synchronous, so each async trait method dispatches the
//! actual query through `tokio::task::spawn_blocking`. The trait is
//! defined in `lso-core` so consumers depend on the contract, not on
//! `lso-db`.

use async_trait::async_trait;
use lso_core::ExplanationCache;
use std::sync::Arc;

use crate::Database;

/// SQLCipher-backed explanation cache. Holds an `Arc<Database>` so the
/// underlying connection can be shared with the rest of the
/// application.
pub struct SqlCipherExplanationCache {
    db: Arc<Database>,
}

impl SqlCipherExplanationCache {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ExplanationCache for SqlCipherExplanationCache {
    async fn get(&self, rec_id: &str, data_hash: &str) -> Option<String> {
        let db = self.db.clone();
        let rec_id = rec_id.to_string();
        let data_hash = data_hash.to_string();
        tokio::task::spawn_blocking(move || db.get_explanation(&rec_id, &data_hash))
            .await
            .ok()
            .and_then(|res| match res {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(error = %e, "explanation cache get failed");
                    None
                }
            })
    }

    async fn put(&self, rec_id: &str, data_hash: &str, explanation: &str) {
        let db = self.db.clone();
        let rec_id = rec_id.to_string();
        let data_hash = data_hash.to_string();
        let explanation = explanation.to_string();
        let result = tokio::task::spawn_blocking(move || {
            db.put_explanation(&rec_id, &data_hash, &explanation)
        })
        .await;
        if let Ok(Err(e)) = result {
            tracing::warn!(error = %e, "explanation cache put failed");
        }
    }

    async fn invalidate(&self, rec_id: &str) {
        let db = self.db.clone();
        let rec_id = rec_id.to_string();
        let result =
            tokio::task::spawn_blocking(move || db.invalidate_explanation(&rec_id)).await;
        if let Ok(Err(e)) = result {
            tracing::warn!(error = %e, "explanation cache invalidate failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_db() -> (tempfile::TempDir, Arc<Database>) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db"), "key").unwrap();
        db.migrate().unwrap();
        (dir, Arc::new(db))
    }

    #[tokio::test]
    async fn put_get_roundtrip() {
        let (_dir, db) = open_db();
        let cache = SqlCipherExplanationCache::new(db);

        assert_eq!(cache.get("r1", "h1").await, None);
        cache.put("r1", "h1", "hello").await;
        assert_eq!(cache.get("r1", "h1").await.as_deref(), Some("hello"));
        // mismatched hash misses
        assert_eq!(cache.get("r1", "h2").await, None);
    }

    #[tokio::test]
    async fn put_replaces_existing_row() {
        let (_dir, db) = open_db();
        let cache = SqlCipherExplanationCache::new(db);

        cache.put("r1", "h1", "v1").await;
        cache.put("r1", "h2", "v2").await;
        assert_eq!(cache.get("r1", "h1").await, None);
        assert_eq!(cache.get("r1", "h2").await.as_deref(), Some("v2"));
    }

    #[tokio::test]
    async fn invalidate_removes_entry() {
        let (_dir, db) = open_db();
        let cache = SqlCipherExplanationCache::new(db);

        cache.put("r1", "h1", "v1").await;
        cache.invalidate("r1").await;
        assert_eq!(cache.get("r1", "h1").await, None);
    }

    #[tokio::test]
    async fn survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");

        {
            let db = Database::open(&path, "key").unwrap();
            db.migrate().unwrap();
            let cache = SqlCipherExplanationCache::new(Arc::new(db));
            cache.put("r1", "h1", "persist me").await;
        }

        let db = Database::open(&path, "key").unwrap();
        let cache = SqlCipherExplanationCache::new(Arc::new(db));
        assert_eq!(
            cache.get("r1", "h1").await.as_deref(),
            Some("persist me")
        );
    }
}
