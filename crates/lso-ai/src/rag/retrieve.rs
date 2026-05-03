//! [`RagRetriever`] — embed a query and pull the top-k matching chunks.

use crate::rag::store::VectorStore;
use crate::EmbeddingProvider;
use lso_core::AiError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// One chunk returned by a similarity search, decorated with its
/// score in `[0.0, 1.0]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievedChunk {
    pub id: String,
    pub source: String,
    pub text: String,
    pub score: f32,
}

pub struct RagRetriever {
    store: Arc<dyn VectorStore>,
    embedder: Arc<dyn EmbeddingProvider>,
}

impl RagRetriever {
    pub fn new(store: Arc<dyn VectorStore>, embedder: Arc<dyn EmbeddingProvider>) -> Self {
        Self { store, embedder }
    }

    /// Returns the `top_k` most relevant chunks. If the store is empty
    /// (e.g. the index hasn't been built yet) returns `Ok(vec![])` —
    /// callers should treat that as graceful degradation, not an
    /// error. See F17 acceptance criteria.
    pub async fn search(&self, query: &str, top_k: usize) -> Result<Vec<RetrievedChunk>, AiError> {
        if top_k == 0 || self.store.count().await? == 0 {
            return Ok(Vec::new());
        }
        let q = self.embedder.embed(query).await?;
        let results = self.store.search(&q, top_k).await?;
        Ok(results
            .into_iter()
            .map(|(c, score)| RetrievedChunk {
                id: c.id,
                source: c.source,
                text: c.text,
                score,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::ingest::ingest_text;
    use crate::rag::store::OnDiskVectorStore;
    use async_trait::async_trait;

    struct DeterministicEmbedder {
        dim: usize,
    }
    #[async_trait]
    impl EmbeddingProvider for DeterministicEmbedder {
        async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError> {
            let mut v = vec![0.0f32; self.dim];
            for (i, b) in text.bytes().enumerate() {
                v[i % self.dim] += b as f32;
            }
            let n = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1.0);
            for x in &mut v {
                *x /= n;
            }
            Ok(v)
        }
        fn embedding_dim(&self) -> usize {
            self.dim
        }
        fn embedding_model(&self) -> &str {
            "stub"
        }
    }

    #[tokio::test]
    async fn empty_store_returns_no_results() {
        let dir = tempfile::tempdir().unwrap();
        let store: Arc<dyn VectorStore> = Arc::new(
            OnDiskVectorStore::open(dir.path().join("v.json"))
                .await
                .unwrap(),
        );
        let embedder: Arc<dyn EmbeddingProvider> = Arc::new(DeterministicEmbedder { dim: 8 });
        let r = RagRetriever::new(store, embedder);
        let hits = r.search("anything", 3).await.unwrap();
        assert!(hits.is_empty());
    }

    #[tokio::test]
    async fn search_returns_top_k() {
        let dir = tempfile::tempdir().unwrap();
        let store: Arc<dyn VectorStore> = Arc::new(
            OnDiskVectorStore::open(dir.path().join("v.json"))
                .await
                .unwrap(),
        );
        let embedder = DeterministicEmbedder { dim: 8 };
        ingest_text(
            "guide:disk",
            "free disk space by clearing caches\n\nlarge downloads accumulate over time",
            &embedder,
            store.as_ref(),
        )
        .await
        .unwrap();
        ingest_text(
            "guide:cpu",
            "cpu temperature should remain below thermal limits",
            &embedder,
            store.as_ref(),
        )
        .await
        .unwrap();

        let embedder: Arc<dyn EmbeddingProvider> = Arc::new(DeterministicEmbedder { dim: 8 });
        let r = RagRetriever::new(store, embedder);
        let hits = r.search("clearing caches and downloads", 2).await.unwrap();
        assert!(hits.len() <= 2);
        assert!(!hits.is_empty());
    }
}
