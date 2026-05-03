//! [`ExplanationService`] turns a deterministic recommendation plus
//! probe data into a human-readable explanation. The LLM call is
//! optional: if the provider is unavailable the rule's own description
//! is returned, so the UI always has something to display.
//!
//! **Trust boundary:** the LLM's output is text, returned as `String`.
//! It is never parsed back into a structured action and never reaches
//! the actuator. See CLAUDE.md "Deterministic logic for decisions, AI
//! for explanations."

use crate::context::{build_context, build_prompt, data_hash};
use crate::rag::RagRetriever;
use crate::LlmProvider;
use lso_core::{AiError, ExplanationCache, ProbeResult, Recommendation};
use std::sync::Arc;

const MAX_TOKENS: u32 = 256;
const RAG_TOP_K: usize = 3;

/// Produces explanations for recommendations, optionally enriched by
/// RAG retrieval and backed by an [`ExplanationCache`]. Coexists with
/// [`crate::AiService`] (which provides the simpler context-string
/// interface used by F15); this is the F16 implementation that pulls
/// probe data into the prompt and persists results.
pub struct ExplanationService {
    llm: Arc<dyn LlmProvider>,
    rag: Option<Arc<RagRetriever>>,
    cache: Option<Arc<dyn ExplanationCache>>,
}

impl ExplanationService {
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self {
            llm,
            rag: None,
            cache: None,
        }
    }

    pub fn with_rag(mut self, rag: Arc<RagRetriever>) -> Self {
        self.rag = Some(rag);
        self
    }

    pub fn with_cache(mut self, cache: Arc<dyn ExplanationCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Generate an explanation for `rec`. Returns the cached value if
    /// the recommendation+data hash matches; otherwise calls the LLM
    /// and caches the result. If the LLM is offline, returns the
    /// recommendation's own `description` as a fallback.
    pub async fn explain(
        &self,
        rec: &Recommendation,
        probes: &[ProbeResult],
    ) -> Result<String, AiError> {
        let rec_id = rec.id.to_string();
        let hash = data_hash(rec, probes);

        if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get(&rec_id, &hash).await {
                return Ok(cached);
            }
        }

        if !self.llm.is_available().await {
            return Ok(rec.description.clone());
        }

        let references = self.retrieve_references(rec).await;
        let ctx = build_context(rec, probes, &references);
        let prompt = build_prompt(&ctx);

        let raw = self.llm.generate(&prompt, MAX_TOKENS).await?;
        let explanation = raw.trim().to_string();

        if let Some(cache) = &self.cache {
            cache.put(&rec_id, &hash, &explanation).await;
        }
        Ok(explanation)
    }

    /// Generate explanations for many recommendations. Errors are
    /// collected per-item so a single failure doesn't poison the batch.
    pub async fn explain_batch(
        &self,
        recs: &[Recommendation],
        probes: &[ProbeResult],
    ) -> Vec<Result<String, AiError>> {
        let mut out = Vec::with_capacity(recs.len());
        for rec in recs {
            out.push(self.explain(rec, probes).await);
        }
        out
    }

    /// Drop any cached explanation for `rec` and produce a fresh one.
    /// Backs the "Regenerate" button in the UI.
    pub async fn regenerate(
        &self,
        rec: &Recommendation,
        probes: &[ProbeResult],
    ) -> Result<String, AiError> {
        if let Some(cache) = &self.cache {
            cache.invalidate(&rec.id.to_string()).await;
        }
        self.explain(rec, probes).await
    }

    async fn retrieve_references(&self, rec: &Recommendation) -> Vec<String> {
        let Some(rag) = &self.rag else {
            return Vec::new();
        };
        let query = format!("{} {} {}", rec.title, rec.category, rec.target);
        match rag.search(&query, RAG_TOP_K).await {
            Ok(chunks) => chunks
                .into_iter()
                .map(|c| format!("[{}] {}", c.source, c.text))
                .collect(),
            // RAG failures must not block explanation. The trust
            // boundary is "deterministic decisions, AI explanations" —
            // missing references just means a slightly thinner prompt.
            Err(err) => {
                tracing::warn!(error = %err, "rag retrieval failed, continuing without references");
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use lso_core::{Platform, RecommendationStatus, RiskLevel, SystemMetric};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use uuid::Uuid;

    struct MockLlm {
        available: bool,
        response: String,
        gen_calls: AtomicUsize,
    }
    impl MockLlm {
        fn new(response: &str, available: bool) -> Self {
            Self {
                available,
                response: response.into(),
                gen_calls: AtomicUsize::new(0),
            }
        }
    }
    #[async_trait]
    impl LlmProvider for MockLlm {
        async fn generate(&self, _prompt: &str, _max_tokens: u32) -> Result<String, AiError> {
            self.gen_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.response.clone())
        }
        async fn is_available(&self) -> bool {
            self.available
        }
        fn model_name(&self) -> &str {
            "mock"
        }
    }

    #[derive(Default)]
    struct MockCache {
        store: Mutex<std::collections::HashMap<String, (String, String)>>,
    }
    #[async_trait]
    impl ExplanationCache for MockCache {
        async fn get(&self, rec_id: &str, hash: &str) -> Option<String> {
            self.store
                .lock()
                .unwrap()
                .get(rec_id)
                .filter(|(h, _)| h == hash)
                .map(|(_, v)| v.clone())
        }
        async fn put(&self, rec_id: &str, hash: &str, exp: &str) {
            self.store
                .lock()
                .unwrap()
                .insert(rec_id.into(), (hash.into(), exp.into()));
        }
        async fn invalidate(&self, rec_id: &str) {
            self.store.lock().unwrap().remove(rec_id);
        }
    }

    fn rec() -> Recommendation {
        Recommendation {
            id: Uuid::new_v4(),
            rule_id: "disk.usage_high".into(),
            title: "t".into(),
            description: "fallback description".into(),
            risk_level: RiskLevel::Low,
            category: "disk".into(),
            target: "/".into(),
            rollback_plan: None,
            status: RecommendationStatus::Pending,
            rejection_reason: None,
            created_at: Utc::now(),
            resolved_at: None,
        }
    }
    fn probes() -> Vec<ProbeResult> {
        vec![ProbeResult {
            probe_id: "disk.usage".into(),
            metrics: vec![SystemMetric {
                id: Uuid::new_v4(),
                probe_id: "disk.usage".into(),
                name: "disk.usage_percent".into(),
                value: 92.0,
                unit: Some("%".into()),
                collected_at: Utc::now(),
                platform: "macos".into(),
            }],
            collected_at: Utc::now(),
            platform: Platform::MacOS,
        }]
    }

    #[tokio::test]
    async fn fallback_when_llm_unavailable() {
        let llm = Arc::new(MockLlm::new("never called", false));
        let svc = ExplanationService::new(llm.clone());
        let out = svc.explain(&rec(), &probes()).await.unwrap();
        assert_eq!(out, "fallback description");
        assert_eq!(llm.gen_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn caches_and_skips_second_call() {
        let llm = Arc::new(MockLlm::new("the explanation", true));
        let cache = Arc::new(MockCache::default());
        let svc = ExplanationService::new(llm.clone()).with_cache(cache.clone());

        let r = rec();
        let p = probes();
        let a = svc.explain(&r, &p).await.unwrap();
        let b = svc.explain(&r, &p).await.unwrap();
        assert_eq!(a, "the explanation");
        assert_eq!(a, b);
        assert_eq!(llm.gen_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn regenerate_invalidates_cache() {
        let llm = Arc::new(MockLlm::new("v1", true));
        let cache = Arc::new(MockCache::default());
        let svc = ExplanationService::new(llm.clone()).with_cache(cache.clone());

        let r = rec();
        let p = probes();
        let _ = svc.explain(&r, &p).await.unwrap();
        let _ = svc.regenerate(&r, &p).await.unwrap();
        assert_eq!(llm.gen_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn batch_collects_results_per_item() {
        let llm = Arc::new(MockLlm::new("ok", true));
        let svc = ExplanationService::new(llm);
        let r = rec();
        let recs = vec![r.clone(), r];
        let out = svc.explain_batch(&recs, &probes()).await;
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|r| r.is_ok()));
    }
}
