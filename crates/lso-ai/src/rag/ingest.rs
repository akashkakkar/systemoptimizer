//! Document ingestion: chunk -> embed -> upsert.

use crate::rag::chunk::chunk_paragraphs;
use crate::rag::store::{Chunk, VectorStore};
use crate::EmbeddingProvider;
use lso_core::AiError;
use std::path::Path;

/// Counts returned from an ingest pass.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IngestStats {
    pub documents: usize,
    pub chunks: usize,
}

impl IngestStats {
    fn merge(&mut self, other: IngestStats) {
        self.documents += other.documents;
        self.chunks += other.chunks;
    }
}

/// Chunk `text`, embed each chunk, and upsert into `store`. The `source`
/// label is recorded on every chunk and shown to the user when an
/// explanation cites a reference.
pub async fn ingest_text(
    source: &str,
    text: &str,
    embedder: &dyn EmbeddingProvider,
    store: &dyn VectorStore,
) -> Result<IngestStats, AiError> {
    let chunks = chunk_paragraphs(text);
    if chunks.is_empty() {
        return Ok(IngestStats::default());
    }
    let mut records = Vec::with_capacity(chunks.len());
    for (i, chunk_text) in chunks.iter().enumerate() {
        let embedding = embedder.embed(chunk_text).await?;
        records.push(Chunk {
            id: format!("{}#{}", source, i),
            source: source.to_string(),
            text: chunk_text.clone(),
            embedding,
        });
    }
    let n = records.len();
    store.upsert(records).await?;
    Ok(IngestStats {
        documents: 1,
        chunks: n,
    })
}

/// Read a rule TOML file, synthesise a doc-style description, and
/// ingest it under the source `rule:<id>`.
pub async fn ingest_rule_toml(
    path: &Path,
    embedder: &dyn EmbeddingProvider,
    store: &dyn VectorStore,
) -> Result<IngestStats, AiError> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| AiError::RequestFailed(format!("read {}: {}", path.display(), e)))?;
    let value: toml::Value = toml::from_str(&content)
        .map_err(|e| AiError::ParseError(format!("toml {}: {}", path.display(), e)))?;

    let rule = value
        .get("rule")
        .ok_or_else(|| AiError::ParseError(format!("{} missing [rule]", path.display())))?;
    let id = rule.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let name = rule.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let description = rule.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let category = rule.get("category").and_then(|v| v.as_str()).unwrap_or("");
    let rec = rule.get("recommendation");
    let rec_title = rec
        .and_then(|r| r.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let rec_description = rec
        .and_then(|r| r.get("description"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let synthesised = format!(
        "Rule {id} ({category}): {name}\n\n{description}\n\nRecommendation: {rec_title}\n{rec_description}",
    );

    ingest_text(&format!("rule:{}", id), &synthesised, embedder, store).await
}

/// Walk `dir` and ingest every `.toml` (as a rule), `.md`, and `.txt`
/// file found at the top level.
pub async fn ingest_directory(
    dir: &Path,
    embedder: &dyn EmbeddingProvider,
    store: &dyn VectorStore,
) -> Result<IngestStats, AiError> {
    let mut stats = IngestStats::default();
    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| AiError::RequestFailed(format!("read_dir {}: {}", dir.display(), e)))?;
    loop {
        let entry = entries
            .next_entry()
            .await
            .map_err(|e| AiError::RequestFailed(e.to_string()))?;
        let Some(entry) = entry else { break };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase());
        let result = match ext.as_deref() {
            Some("toml") => ingest_rule_toml(&path, embedder, store).await,
            Some("md") | Some("txt") => {
                let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
                    AiError::RequestFailed(format!("read {}: {}", path.display(), e))
                })?;
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("doc");
                ingest_text(&format!("guide:{}", stem), &content, embedder, store).await
            }
            _ => continue,
        };
        match result {
            Ok(s) => stats.merge(s),
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "ingest skipped");
            }
        }
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::store::OnDiskVectorStore;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct StubEmbedder {
        dim: usize,
        calls: AtomicUsize,
    }
    #[async_trait]
    impl EmbeddingProvider for StubEmbedder {
        async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
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
    async fn ingest_text_creates_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let store = OnDiskVectorStore::open(dir.path().join("v.json"))
            .await
            .unwrap();
        let embedder = StubEmbedder {
            dim: 8,
            calls: AtomicUsize::new(0),
        };
        let stats = ingest_text("guide:test", "para one\n\npara two", &embedder, &store)
            .await
            .unwrap();
        assert_eq!(stats.documents, 1);
        assert!(stats.chunks >= 1);
        assert_eq!(store.count().await.unwrap(), stats.chunks);
    }

    #[tokio::test]
    async fn ingest_rule_toml_synthesises_doc() {
        let dir = tempfile::tempdir().unwrap();
        let rule_path = dir.path().join("disk.toml");
        std::fs::write(
            &rule_path,
            r#"
[rule]
id = "disk.usage_high"
name = "High disk usage"
description = "Volume is using more than 85% of capacity."
category = "disk"

[rule.recommendation]
title = "Free up disk space"
description = "Remove caches, large old files, and downloads."
"#,
        )
        .unwrap();

        let store = OnDiskVectorStore::open(dir.path().join("v.json"))
            .await
            .unwrap();
        let embedder = StubEmbedder {
            dim: 8,
            calls: AtomicUsize::new(0),
        };
        let stats = ingest_rule_toml(&rule_path, &embedder, &store)
            .await
            .unwrap();
        assert_eq!(stats.documents, 1);
        assert!(stats.chunks >= 1);

        let hits = store
            .search(&embedder.embed("free disk space").await.unwrap(), 1)
            .await
            .unwrap();
        assert!(hits[0].0.source.starts_with("rule:disk"));
    }

    #[tokio::test]
    async fn ingest_directory_picks_up_supported_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.toml"),
            "[rule]\nid = \"x.y\"\nname = \"x\"\ndescription = \"d\"\ncategory = \"x\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("b.md"), "# Heading\n\nbody one\n\nbody two").unwrap();
        std::fs::write(dir.path().join("c.bin"), b"ignored").unwrap();

        let store = OnDiskVectorStore::open(dir.path().join("v.json"))
            .await
            .unwrap();
        let embedder = StubEmbedder {
            dim: 8,
            calls: AtomicUsize::new(0),
        };
        let stats = ingest_directory(dir.path(), &embedder, &store).await.unwrap();
        assert_eq!(stats.documents, 2);
        assert!(stats.chunks >= 2);
    }
}
