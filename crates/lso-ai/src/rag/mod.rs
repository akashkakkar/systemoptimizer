//! Retrieval-augmented generation pipeline.
//!
//! ```text
//! Documents -> chunk_paragraphs -> embed (ollama) -> VectorStore
//!                                                       |
//!                                          query -> embed -> top-k
//! ```
//!
//! The default [`OnDiskVectorStore`] persists chunks and embeddings as
//! a JSON file in the app data directory. The [`VectorStore`] trait is
//! the seam for swapping in a higher-throughput vector DB later
//! without touching the retriever or ingest code.

pub mod chunk;
pub mod ingest;
pub mod retrieve;
pub mod store;

pub use chunk::{chunk_paragraphs, CHUNK_OVERLAP_TOKENS, CHUNK_TOKEN_SIZE};
pub use ingest::{ingest_directory, ingest_rule_toml, ingest_text, IngestStats};
pub use retrieve::{RagRetriever, RetrievedChunk};
pub use store::{Chunk, OnDiskVectorStore, VectorStore};
