//! Vector store abstraction and a small on-disk implementation.
//!
//! [`OnDiskVectorStore`] persists every chunk + embedding as a single
//! JSON file under the app data directory. It is intentionally simple:
//! the knowledge bases used by LSO are small (rule descriptions and a
//! handful of bundled OS guides — typically a few hundred chunks), and
//! a JSON-backed vector + cosine search comfortably handles that.
//!
//! The [`VectorStore`] trait is the seam for swapping in a
//! higher-throughput vector DB (e.g. LanceDB) later. Call sites depend
//! only on the trait.

use async_trait::async_trait;
use lso_core::AiError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

/// A document chunk plus its dense embedding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    /// Logical source identifier, e.g. `rule:disk.usage_high` or
    /// `guide:disk_management`.
    pub source: String,
    pub text: String,
    pub embedding: Vec<f32>,
}

/// Storage and similarity search over [`Chunk`]s.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Insert or replace `chunks`, keyed by [`Chunk::id`].
    async fn upsert(&self, chunks: Vec<Chunk>) -> Result<(), AiError>;

    /// Return the `top_k` chunks ranked by cosine similarity, plus
    /// their scores.
    async fn search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<(Chunk, f32)>, AiError>;

    async fn count(&self) -> Result<usize, AiError>;
    async fn clear(&self) -> Result<(), AiError>;
}

/// JSON-file-backed vector store. Reads the full corpus into memory on
/// open and writes the file atomically on every mutation.
pub struct OnDiskVectorStore {
    path: PathBuf,
    inner: RwLock<Vec<Chunk>>,
}

impl OnDiskVectorStore {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, AiError> {
        let path = path.as_ref().to_path_buf();
        let chunks: Vec<Chunk> = if path.exists() {
            let data = tokio::fs::read(&path)
                .await
                .map_err(|e| AiError::RequestFailed(format!("read {}: {}", path.display(), e)))?;
            if data.is_empty() {
                Vec::new()
            } else {
                serde_json::from_slice(&data)
                    .map_err(|e| AiError::ParseError(format!("parse {}: {}", path.display(), e)))?
            }
        } else {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    tokio::fs::create_dir_all(parent).await.map_err(|e| {
                        AiError::RequestFailed(format!(
                            "mkdir {}: {}",
                            parent.display(),
                            e
                        ))
                    })?;
                }
            }
            Vec::new()
        };
        Ok(Self {
            path,
            inner: RwLock::new(chunks),
        })
    }

    async fn flush(&self, chunks: &[Chunk]) -> Result<(), AiError> {
        let data = serde_json::to_vec(chunks)
            .map_err(|e| AiError::ParseError(e.to_string()))?;
        let tmp = PathBuf::from(format!("{}.tmp", self.path.display()));
        tokio::fs::write(&tmp, &data)
            .await
            .map_err(|e| AiError::RequestFailed(format!("write {}: {}", tmp.display(), e)))?;
        tokio::fs::rename(&tmp, &self.path)
            .await
            .map_err(|e| AiError::RequestFailed(format!("rename: {}", e)))?;
        Ok(())
    }
}

#[async_trait]
impl VectorStore for OnDiskVectorStore {
    async fn upsert(&self, chunks: Vec<Chunk>) -> Result<(), AiError> {
        let mut guard = self.inner.write().await;
        for c in chunks {
            if let Some(existing) = guard.iter_mut().find(|x| x.id == c.id) {
                *existing = c;
            } else {
                guard.push(c);
            }
        }
        self.flush(&guard).await
    }

    async fn search(
        &self,
        query: &[f32],
        top_k: usize,
    ) -> Result<Vec<(Chunk, f32)>, AiError> {
        if top_k == 0 {
            return Ok(Vec::new());
        }
        let guard = self.inner.read().await;
        let mut scored: Vec<(Chunk, f32)> = guard
            .iter()
            .map(|c| (c.clone(), cosine_similarity(query, &c.embedding)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        Ok(scored)
    }

    async fn count(&self) -> Result<usize, AiError> {
        Ok(self.inner.read().await.len())
    }

    async fn clear(&self) -> Result<(), AiError> {
        let mut guard = self.inner.write().await;
        guard.clear();
        self.flush(&guard).await
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0_f32;
    let mut na = 0.0_f32;
    let mut nb = 0.0_f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(id: &str, src: &str, text: &str, emb: Vec<f32>) -> Chunk {
        Chunk {
            id: id.into(),
            source: src.into(),
            text: text.into(),
            embedding: emb,
        }
    }

    #[test]
    fn cosine_basic() {
        assert!((cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert!((cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]) + 1.0).abs() < 1e-6);
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 1.0]), 0.0);
    }

    #[tokio::test]
    async fn roundtrip_upsert_search_persist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vectors.json");

        let store = OnDiskVectorStore::open(&path).await.unwrap();
        store
            .upsert(vec![
                ch("a", "rule:disk", "disk usage above 85% is concerning", vec![1.0, 0.0, 0.0]),
                ch("b", "rule:cpu", "cpu usage at idle should be low", vec![0.0, 1.0, 0.0]),
                ch("c", "guide:net", "network adapter info", vec![0.0, 0.0, 1.0]),
            ])
            .await
            .unwrap();
        assert_eq!(store.count().await.unwrap(), 3);

        let hits = store.search(&[0.9, 0.1, 0.0], 2).await.unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].0.id, "a");

        drop(store);
        let store2 = OnDiskVectorStore::open(&path).await.unwrap();
        assert_eq!(store2.count().await.unwrap(), 3);

        store2
            .upsert(vec![ch("a", "rule:disk", "updated", vec![1.0, 0.0, 0.0])])
            .await
            .unwrap();
        assert_eq!(store2.count().await.unwrap(), 3);
        let hits = store2.search(&[1.0, 0.0, 0.0], 1).await.unwrap();
        assert_eq!(hits[0].0.text, "updated");

        store2.clear().await.unwrap();
        assert_eq!(store2.count().await.unwrap(), 0);
    }
}
