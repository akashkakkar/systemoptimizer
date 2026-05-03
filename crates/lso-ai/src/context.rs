//! Prompt and context construction for [`crate::ExplanationService`].
//!
//! Token counting is approximated by character length (≈ 4 chars/token).
//! That's intentionally conservative: a real tokenizer would tie us to
//! a specific model, and we want to stay safely below the model's
//! context window across the full set of supported local models.

use lso_core::{ProbeResult, Recommendation, SystemMetric};
use sha2::{Digest, Sha256};

const APPROX_CHARS_PER_TOKEN: usize = 4;
/// Hard cap on prompt size. Stays well below the 4k window of small
/// instruction-tuned models like llama3.2:3b after the model's own
/// system prompt and response budget are accounted for.
pub const CONTEXT_TOKEN_LIMIT: usize = 2048;

const MAX_METRIC_LINES: usize = 20;
const MAX_REFERENCE_BLOCKS: usize = 3;

/// Structured pieces of the explanation prompt. Kept separate so each
/// section can be trimmed independently when over the token budget.
#[derive(Debug, Clone)]
pub struct ExplanationContext {
    pub recommendation: String,
    pub metrics: String,
    pub references: String,
}

impl ExplanationContext {
    /// Approximate token count of all sections combined.
    pub fn approx_tokens(&self) -> usize {
        (self.recommendation.len() + self.metrics.len() + self.references.len())
            / APPROX_CHARS_PER_TOKEN
    }
}

/// Assemble the context block for a recommendation. Selects metrics
/// related to the recommendation's category or target, then trims
/// references then metrics until the result fits in
/// [`CONTEXT_TOKEN_LIMIT`].
pub fn build_context(
    rec: &Recommendation,
    probes: &[ProbeResult],
    references: &[String],
) -> ExplanationContext {
    let recommendation = format!(
        "id: {}\nrule: {}\ntitle: {}\ndescription: {}\nrisk: {}\ncategory: {}\ntarget: {}\n",
        rec.id,
        rec.rule_id,
        rec.title,
        rec.description,
        rec.risk_level,
        rec.category,
        rec.target,
    );

    let mut relevant: Vec<&SystemMetric> = probes
        .iter()
        .flat_map(|p| p.metrics.iter())
        .filter(|m| metric_matches(m, rec))
        .collect();
    if relevant.is_empty() {
        relevant = probes
            .iter()
            .flat_map(|p| p.metrics.iter())
            .take(8)
            .collect();
    }

    let mut metrics = String::from("metrics:\n");
    for m in relevant.iter().take(MAX_METRIC_LINES) {
        use std::fmt::Write;
        let unit = m.unit.as_deref().unwrap_or("");
        let _ = writeln!(
            metrics,
            "  - {} = {} {} (probe: {})",
            m.name, m.value, unit, m.source_probe_or_id()
        );
    }

    let references_block = if references.is_empty() {
        String::new()
    } else {
        let mut s = String::from("references:\n");
        for r in references.iter().take(MAX_REFERENCE_BLOCKS) {
            s.push_str("- ");
            s.push_str(r);
            if !r.ends_with('\n') {
                s.push('\n');
            }
        }
        s
    };

    let mut ctx = ExplanationContext {
        recommendation,
        metrics,
        references: references_block,
    };

    while ctx.approx_tokens() > CONTEXT_TOKEN_LIMIT && !ctx.references.is_empty() {
        trim_last_line(&mut ctx.references);
    }
    while ctx.approx_tokens() > CONTEXT_TOKEN_LIMIT && !ctx.metrics.is_empty() {
        trim_last_line(&mut ctx.metrics);
    }

    ctx
}

fn metric_matches(m: &SystemMetric, rec: &Recommendation) -> bool {
    m.probe_id.starts_with(&rec.category)
        || (!rec.target.is_empty() && m.probe_id.contains(&rec.target))
        || m.name.contains(&rec.category)
}

fn trim_last_line(s: &mut String) {
    let trimmed_len = s.trim_end_matches('\n').len();
    let cut = s[..trimmed_len].rfind('\n').map(|i| i + 1).unwrap_or(0);
    s.truncate(cut);
}

/// Render the context as a complete prompt string ready for the LLM.
/// The prompt instructs the model to stay non-technical and to never
/// propose new commands — the deterministic action plan comes from
/// the rule engine, not the LLM.
pub fn build_prompt(ctx: &ExplanationContext) -> String {
    format!(
        "You are LSO, a local system optimizer. Explain the following recommendation \
         in 2-3 plain-English sentences. Be concrete, reference the metrics, and stay non-technical. \
         Do not propose new commands; the system already has a deterministic action plan.\n\n\
         RECOMMENDATION:\n{}\n\n{}{}\nEXPLANATION:\n",
        ctx.recommendation, ctx.metrics, ctx.references,
    )
}

/// Stable hash of the inputs that determine the explanation. Used as
/// the cache key alongside the recommendation id so an explanation is
/// reused only while the underlying probe data is unchanged.
pub fn data_hash(rec: &Recommendation, probes: &[ProbeResult]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(rec.id.as_bytes());
    hasher.update(b"\0");
    hasher.update(rec.rule_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(rec.description.as_bytes());
    hasher.update(b"\0");
    hasher.update(rec.target.as_bytes());
    for p in probes {
        hasher.update(b"\0");
        hasher.update(p.probe_id.as_bytes());
        for m in &p.metrics {
            hasher.update(b"\0");
            hasher.update(m.name.as_bytes());
            hasher.update(m.value.to_le_bytes());
        }
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(out, "{:02x}", b);
    }
    out
}

// SystemMetric in develop has `probe_id` (no `source_probe`); shim for
// readability in build_context.
trait MetricProbeId {
    fn source_probe_or_id(&self) -> &str;
}

impl MetricProbeId for SystemMetric {
    fn source_probe_or_id(&self) -> &str {
        &self.probe_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use lso_core::{Platform, RecommendationStatus, RiskLevel};
    use uuid::Uuid;

    fn rec() -> Recommendation {
        Recommendation {
            id: Uuid::new_v4(),
            rule_id: "disk.usage_high".into(),
            title: "Free up disk space on /".into(),
            description: "Volume / is 92% full".into(),
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

    fn metric(probe: &str, name: &str, v: f64) -> SystemMetric {
        SystemMetric {
            id: Uuid::new_v4(),
            probe_id: probe.into(),
            name: name.into(),
            value: v,
            unit: Some("%".into()),
            collected_at: Utc::now(),
            platform: "macos".into(),
        }
    }

    #[test]
    fn context_includes_matching_metrics() {
        let probe = ProbeResult {
            probe_id: "disk.usage".into(),
            metrics: vec![metric("disk.usage", "disk.usage_percent", 92.0)],
            collected_at: Utc::now(),
            platform: Platform::MacOS,
        };
        let ctx = build_context(&rec(), &[probe], &[]);
        assert!(ctx.recommendation.contains("disk.usage_high"));
        assert!(ctx.metrics.contains("disk.usage_percent"));
    }

    #[test]
    fn context_falls_back_when_no_match() {
        let probe = ProbeResult {
            probe_id: "cpu.usage".into(),
            metrics: vec![metric("cpu.usage", "cpu.load", 0.5)],
            collected_at: Utc::now(),
            platform: Platform::MacOS,
        };
        let ctx = build_context(&rec(), &[probe], &[]);
        assert!(ctx.metrics.contains("cpu.load"));
    }

    #[test]
    fn context_stays_under_token_limit() {
        let big_metrics: Vec<SystemMetric> = (0..500)
            .map(|i| metric("disk.usage", &format!("metric_{}", i), i as f64))
            .collect();
        let probe = ProbeResult {
            probe_id: "disk.usage".into(),
            metrics: big_metrics,
            collected_at: Utc::now(),
            platform: Platform::Linux,
        };
        let huge_refs: Vec<String> = (0..50)
            .map(|i| format!("ref-{} {}", i, "x".repeat(2000)))
            .collect();
        let ctx = build_context(&rec(), &[probe], &huge_refs);
        assert!(
            ctx.approx_tokens() <= CONTEXT_TOKEN_LIMIT,
            "got {} tokens",
            ctx.approx_tokens()
        );
    }

    #[test]
    fn data_hash_stable_and_input_sensitive() {
        let r = rec();
        let p = ProbeResult {
            probe_id: "disk.usage".into(),
            metrics: vec![metric("disk.usage", "x", 1.0)],
            collected_at: Utc::now(),
            platform: Platform::MacOS,
        };
        let h1 = data_hash(&r, std::slice::from_ref(&p));
        let h2 = data_hash(&r, std::slice::from_ref(&p));
        assert_eq!(h1, h2);

        let mut p2 = p.clone();
        p2.metrics[0].value = 2.0;
        let h3 = data_hash(&r, std::slice::from_ref(&p2));
        assert_ne!(h1, h3);
    }

    #[test]
    fn prompt_includes_all_sections() {
        let probe = ProbeResult {
            probe_id: "disk.usage".into(),
            metrics: vec![metric("disk.usage", "disk.usage_percent", 92.0)],
            collected_at: Utc::now(),
            platform: Platform::MacOS,
        };
        let ctx = build_context(&rec(), &[probe], &["[guide:disk] keep below 90%".into()]);
        let prompt = build_prompt(&ctx);
        assert!(prompt.contains("RECOMMENDATION"));
        assert!(prompt.contains("metrics:"));
        assert!(prompt.contains("references:"));
        assert!(prompt.contains("EXPLANATION:"));
    }
}
