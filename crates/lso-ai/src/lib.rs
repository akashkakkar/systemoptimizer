//! LSO AI — local LLM integration for generating explanations.
//!
//! Connects to a local Ollama instance (localhost:11434) to provide
//! human-readable explanations for system recommendations. Gracefully
//! degrades to raw rule text when no LLM is available.

pub mod context;
pub mod explanation;
pub mod ollama;
pub mod prompts;
pub mod rag;

pub use explanation::ExplanationService;
pub use rag::{
    chunk_paragraphs, ingest_directory, ingest_rule_toml, ingest_text, Chunk, IngestStats,
    OnDiskVectorStore, RagRetriever, RetrievedChunk, VectorStore,
};

use async_trait::async_trait;
use lso_core::{AiError, Recommendation};
use serde::{Deserialize, Serialize};

/// Configuration for the AI subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub enabled: bool,
    pub model: String,
    pub host: String,
    pub port: u16,
    pub max_tokens: u32,
    pub timeout_secs: u64,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model: "llama3.2:latest".into(),
            host: "127.0.0.1".into(),
            port: 11434,
            max_tokens: 256,
            timeout_secs: 30,
        }
    }
}

impl AiConfig {
    /// Base URL for the Ollama API. Always localhost.
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

/// Trait for LLM providers. Implementations must be Send + Sync for use
/// across async task boundaries.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Generate a completion for the given prompt.
    async fn generate(&self, prompt: &str, max_tokens: u32) -> Result<String, AiError>;

    /// Check whether the LLM backend is reachable and ready.
    async fn is_available(&self) -> bool;

    /// The name of the model currently configured.
    fn model_name(&self) -> &str;
}

/// Trait for embedding providers used by the RAG pipeline. Kept
/// separate from [`LlmProvider`] because not every text-generation
/// backend exposes embeddings.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Embed `text` into a fixed-dimension dense vector.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError>;

    /// Length of vectors returned by [`embed`].
    ///
    /// [`embed`]: EmbeddingProvider::embed
    fn embedding_dim(&self) -> usize;

    /// The embedding model identifier (for diagnostics).
    fn embedding_model(&self) -> &str;
}

/// High-level AI service that wraps a provider and handles fallback.
pub struct AiService {
    provider: Box<dyn LlmProvider>,
    config: AiConfig,
}

impl AiService {
    pub fn new(provider: Box<dyn LlmProvider>, config: AiConfig) -> Self {
        Self { provider, config }
    }

    /// Generate a human-readable explanation for a recommendation.
    /// Falls back to `recommendation.description` if AI is disabled or unavailable.
    pub async fn explain(&self, recommendation: &Recommendation, context: &str) -> String {
        if !self.config.enabled {
            return recommendation.description.clone();
        }

        if !self.provider.is_available().await {
            tracing::warn!("LLM unavailable, falling back to raw description");
            return recommendation.description.clone();
        }

        let prompt = prompts::explanation_prompt(recommendation, context);
        match self
            .provider
            .generate(&prompt, self.config.max_tokens)
            .await
        {
            Ok(explanation) => explanation,
            Err(e) => {
                tracing::error!(error = %e, "LLM generation failed, using fallback");
                recommendation.description.clone()
            }
        }
    }

    /// Check if the AI backend is available.
    pub async fn is_available(&self) -> bool {
        self.config.enabled && self.provider.is_available().await
    }

    /// The currently configured model name.
    pub fn model_name(&self) -> &str {
        self.provider.model_name()
    }

    /// Current AI configuration.
    pub fn config(&self) -> &AiConfig {
        &self.config
    }

    /// Update the enabled state.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }
}

/// Available models reported by an Ollama instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: u64,
    pub modified_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use lso_core::{RecommendationStatus, RiskLevel};
    use uuid::Uuid;

    struct MockProvider {
        available: bool,
        response: String,
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn generate(&self, _prompt: &str, _max_tokens: u32) -> Result<String, AiError> {
            if self.available {
                Ok(self.response.clone())
            } else {
                Err(AiError::Unavailable("mock unavailable".into()))
            }
        }

        async fn is_available(&self) -> bool {
            self.available
        }

        fn model_name(&self) -> &str {
            "mock-model"
        }
    }

    fn sample_recommendation() -> Recommendation {
        Recommendation {
            id: Uuid::new_v4(),
            rule_id: "disk.usage_high".into(),
            title: "Clean up /home".into(),
            description: "/home at 92% usage".into(),
            risk_level: RiskLevel::Low,
            category: "cleanup".into(),
            target: "/home".into(),
            rollback_plan: None,
            status: RecommendationStatus::Pending,
            rejection_reason: None,
            created_at: Utc::now(),
            resolved_at: None,
        }
    }

    #[tokio::test]
    async fn explain_returns_ai_response_when_available() {
        let provider = MockProvider {
            available: true,
            response: "Your home directory is almost full.".into(),
        };
        let service = AiService::new(Box::new(provider), AiConfig::default());
        let rec = sample_recommendation();

        let result = service.explain(&rec, "92% of 500 GB used").await;
        assert_eq!(result, "Your home directory is almost full.");
    }

    #[tokio::test]
    async fn explain_falls_back_when_unavailable() {
        let provider = MockProvider {
            available: false,
            response: "should not see this".into(),
        };
        let service = AiService::new(Box::new(provider), AiConfig::default());
        let rec = sample_recommendation();

        let result = service.explain(&rec, "context").await;
        assert_eq!(result, "/home at 92% usage");
    }

    #[tokio::test]
    async fn explain_falls_back_when_disabled() {
        let provider = MockProvider {
            available: true,
            response: "should not see this".into(),
        };
        let mut config = AiConfig::default();
        config.enabled = false;
        let service = AiService::new(Box::new(provider), config);
        let rec = sample_recommendation();

        let result = service.explain(&rec, "context").await;
        assert_eq!(result, "/home at 92% usage");
    }

    #[tokio::test]
    async fn is_available_respects_enabled_flag() {
        let provider = MockProvider {
            available: true,
            response: String::new(),
        };
        let mut config = AiConfig::default();
        config.enabled = false;
        let service = AiService::new(Box::new(provider), config);

        assert!(!service.is_available().await);
    }

    #[test]
    fn default_config_is_localhost() {
        let config = AiConfig::default();
        assert_eq!(config.base_url(), "http://127.0.0.1:11434");
        assert!(config.enabled);
    }

    #[test]
    fn llm_provider_trait_is_object_safe() {
        fn _accepts_provider(_p: &dyn LlmProvider) {}
    }
}
