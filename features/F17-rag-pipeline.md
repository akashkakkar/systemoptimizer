# F17 — RAG Pipeline + Vector Store

**Phase:** 3 — AI
**Size:** M (1-2 sessions)
**Branch:** `feat/P3-rag-pipeline`
**Depends on:** F15

## Objective

Build a local RAG pipeline so the LLM can reference system documentation, man pages, and rule descriptions when generating explanations.

## Architecture

```
Documents (man pages, rule docs, OS guides)
    ↓ Chunking (512 tokens, 50 token overlap)
    ↓ Embedding (local model via ollama)
    ↓ Storage (LanceDB on disk)
    ↓
Query → Embed → Top-K search → Context → LLM prompt
```

## Deliverables

### 1. Document Ingestion (`lso-ai/src/rag/ingest.rs`)
- Chunk text documents by paragraph/section boundaries
- Generate embeddings via ollama embedding API (`/api/embeddings`)
- Store in LanceDB (on-disk, no server)

### 2. Knowledge Base Sources
- Rule TOML files (descriptions, recommendations)
- Bundled OS guides (curated, shipped with app)
- System man page summaries (extracted at build time or first run)

### 3. Retrieval (`lso-ai/src/rag/retrieve.rs`)
```rust
pub struct RagRetriever {
    db: LanceDb,
    embedder: Box<dyn LlmProvider>,
}

impl RagRetriever {
    pub async fn search(&self, query: &str, top_k: usize) -> Result<Vec<RetrievedChunk>>;
}
```

### 4. Integration with Explanation Service
- Before generating explanation, retrieve relevant chunks
- Inject as context into prompt
- Cite source in explanation ("According to the disk management guide...")

## Acceptance Criteria

- [ ] Documents chunked and embedded on first run
- [ ] Vector search returns relevant chunks for system queries
- [ ] LanceDB files stored in app data directory
- [ ] Embedding model configurable (default: nomic-embed-text)
- [ ] Works offline — no network except localhost ollama
- [ ] Graceful degradation if RAG index not built yet
