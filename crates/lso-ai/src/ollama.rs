//! Ollama HTTP backend — connects to a local Ollama instance at localhost:11434.

use async_trait::async_trait;
use futures::StreamExt;
use lso_core::AiError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{AiConfig, EmbeddingProvider, LlmProvider, ModelInfo};

/// Default embedding model. nomic-embed-text returns 768-dim vectors;
/// override via [`OllamaBackend::with_embedding_model`].
const DEFAULT_EMBED_MODEL: &str = "nomic-embed-text";
/// Default embedding dimensionality for `nomic-embed-text`.
const DEFAULT_EMBED_DIM: usize = 768;

/// Ollama HTTP API client. Restricted to localhost connections only.
pub struct OllamaBackend {
    client: Client,
    config: AiConfig,
    embedding_model: String,
    embedding_dim: usize,
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    options: GenerateOptions,
}

#[derive(Serialize)]
struct GenerateOptions {
    num_predict: u32,
}

#[derive(Deserialize)]
struct GenerateChunk {
    response: String,
    done: bool,
}

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
    size: u64,
    modified_at: String,
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

impl OllamaBackend {
    /// Create a new Ollama backend from the given config.
    ///
    /// Validates that the host is a loopback address to enforce the
    /// "no network egress" policy.
    pub fn new(config: AiConfig) -> Result<Self, AiError> {
        if !is_loopback(&config.host) {
            return Err(AiError::RequestFailed(format!(
                "refusing non-localhost host: {}",
                config.host
            )));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| AiError::RequestFailed(e.to_string()))?;

        Ok(Self {
            client,
            config,
            embedding_model: DEFAULT_EMBED_MODEL.into(),
            embedding_dim: DEFAULT_EMBED_DIM,
        })
    }

    /// Override the embedding model. The dim must match the model's
    /// output (nomic-embed-text → 768, mxbai-embed-large → 1024, etc.).
    pub fn with_embedding_model(mut self, model: impl Into<String>, dim: usize) -> Self {
        self.embedding_model = model.into();
        self.embedding_dim = dim;
        self
    }

    /// List models available on the local Ollama instance.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, AiError> {
        let url = format!("{}/api/tags", self.config.base_url());
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AiError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(AiError::RequestFailed(format!(
                "list models returned {}",
                resp.status()
            )));
        }

        let tags: TagsResponse = resp
            .json()
            .await
            .map_err(|e| AiError::ParseError(e.to_string()))?;

        Ok(tags
            .models
            .into_iter()
            .map(|m| ModelInfo {
                name: m.name,
                size: m.size,
                modified_at: m.modified_at,
            })
            .collect())
    }

    /// Generate with streaming, collecting the full response.
    /// For real-time display, use `generate_stream` instead.
    async fn generate_streaming(
        &self,
        prompt: &str,
        max_tokens: u32,
    ) -> Result<String, AiError> {
        let url = format!("{}/api/generate", self.config.base_url());
        let body = GenerateRequest {
            model: &self.config.model,
            prompt,
            stream: true,
            options: GenerateOptions {
                num_predict: max_tokens,
            },
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AiError::Timeout
                } else {
                    AiError::RequestFailed(e.to_string())
                }
            })?;

        if !resp.status().is_success() {
            return Err(AiError::RequestFailed(format!(
                "generate returned {}",
                resp.status()
            )));
        }

        let mut result = String::new();
        let mut stream = resp.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| AiError::RequestFailed(e.to_string()))?;
            let text = String::from_utf8_lossy(&bytes);

            for line in text.lines() {
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_str::<GenerateChunk>(line) {
                    Ok(parsed) => {
                        result.push_str(&parsed.response);
                        if parsed.done {
                            return Ok(result);
                        }
                    }
                    Err(e) => {
                        tracing::debug!(line = line, error = %e, "skipping unparseable chunk");
                    }
                }
            }
        }

        Ok(result)
    }

    /// Generate with streaming, calling the callback for each token chunk.
    pub async fn generate_stream<F>(
        &self,
        prompt: &str,
        max_tokens: u32,
        on_token: F,
    ) -> Result<String, AiError>
    where
        F: Fn(&str) + Send,
    {
        let url = format!("{}/api/generate", self.config.base_url());
        let body = GenerateRequest {
            model: &self.config.model,
            prompt,
            stream: true,
            options: GenerateOptions {
                num_predict: max_tokens,
            },
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AiError::Timeout
                } else {
                    AiError::RequestFailed(e.to_string())
                }
            })?;

        if !resp.status().is_success() {
            return Err(AiError::RequestFailed(format!(
                "generate returned {}",
                resp.status()
            )));
        }

        let mut result = String::new();
        let mut stream = resp.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| AiError::RequestFailed(e.to_string()))?;
            let text = String::from_utf8_lossy(&bytes);

            for line in text.lines() {
                if line.is_empty() {
                    continue;
                }
                if let Ok(parsed) = serde_json::from_str::<GenerateChunk>(line) {
                    on_token(&parsed.response);
                    result.push_str(&parsed.response);
                    if parsed.done {
                        return Ok(result);
                    }
                }
            }
        }

        Ok(result)
    }
}

#[async_trait]
impl LlmProvider for OllamaBackend {
    async fn generate(&self, prompt: &str, max_tokens: u32) -> Result<String, AiError> {
        self.generate_streaming(prompt, max_tokens).await
    }

    async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.config.base_url());
        self.client.get(&url).send().await.is_ok()
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaBackend {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError> {
        let url = format!("{}/api/embeddings", self.config.base_url());
        let body = EmbedRequest {
            model: &self.embedding_model,
            prompt: text,
        };
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AiError::Timeout
                } else {
                    AiError::RequestFailed(e.to_string())
                }
            })?;
        if !resp.status().is_success() {
            return Err(AiError::RequestFailed(format!(
                "embeddings returned {}",
                resp.status()
            )));
        }
        let parsed: EmbedResponse = resp
            .json()
            .await
            .map_err(|e| AiError::ParseError(e.to_string()))?;
        if parsed.embedding.len() != self.embedding_dim {
            return Err(AiError::ParseError(format!(
                "expected {} dims, got {}",
                self.embedding_dim,
                parsed.embedding.len()
            )));
        }
        Ok(parsed.embedding)
    }

    fn embedding_dim(&self) -> usize {
        self.embedding_dim
    }

    fn embedding_model(&self) -> &str {
        &self.embedding_model
    }
}

/// Enforce localhost-only connections.
fn is_loopback(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_localhost() {
        let mut config = AiConfig::default();
        config.host = "evil.com".into();
        let result = OllamaBackend::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn accepts_localhost_variants() {
        for host in ["127.0.0.1", "localhost", "::1"] {
            let mut config = AiConfig::default();
            config.host = host.into();
            let result = OllamaBackend::new(config);
            assert!(result.is_ok(), "should accept {host}");
        }
    }

    #[test]
    fn is_loopback_check() {
        assert!(is_loopback("127.0.0.1"));
        assert!(is_loopback("localhost"));
        assert!(is_loopback("::1"));
        assert!(!is_loopback("192.168.1.1"));
        assert!(!is_loopback("example.com"));
    }

    #[test]
    fn generate_chunk_deserializes() {
        let json = r#"{"response":"hello","done":false}"#;
        let chunk: GenerateChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.response, "hello");
        assert!(!chunk.done);
    }

    #[test]
    fn generate_chunk_done() {
        let json = r#"{"response":"","done":true}"#;
        let chunk: GenerateChunk = serde_json::from_str(json).unwrap();
        assert!(chunk.done);
    }

    #[test]
    fn tags_response_deserializes() {
        let json = r#"{"models":[{"name":"llama3.2:latest","size":4000000000,"modified_at":"2024-01-01T00:00:00Z"}]}"#;
        let tags: TagsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(tags.models.len(), 1);
        assert_eq!(tags.models[0].name, "llama3.2:latest");
    }

    #[tokio::test]
    async fn unavailable_ollama_returns_false() {
        let mut config = AiConfig::default();
        config.port = 19999; // unlikely to be running
        config.timeout_secs = 1;
        let backend = OllamaBackend::new(config).unwrap();
        assert!(!backend.is_available().await);
    }
}
