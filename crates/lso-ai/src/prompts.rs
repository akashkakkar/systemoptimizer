//! Prompt templates for LLM-powered explanations.
//!
//! All prompts follow a strict pattern: the LLM explains findings to the user.
//! LLM output is never executed as commands — it is display-only text.

use lso_core::Recommendation;

/// Build a prompt asking the LLM to explain a recommendation in plain language.
pub fn explanation_prompt(recommendation: &Recommendation, context: &str) -> String {
    format!(
        "You are a system administrator explaining an issue to a non-technical user.\n\
         System finding: {title}\n\
         Technical details: {context}\n\
         Risk level: {risk}\n\
         Explain what this means, why it matters, and what the recommended action does.\n\
         Keep it under 3 sentences. No jargon.",
        title = recommendation.title,
        context = context,
        risk = recommendation.risk_level,
    )
}

/// Build a prompt asking the LLM to summarize a set of findings.
pub fn summary_prompt(titles: &[&str]) -> String {
    let findings = titles
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{}. {}", i + 1, t))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "You are a system administrator writing a brief summary for a non-technical user.\n\
         The system scan found the following issues:\n\
         {findings}\n\
         Write a 2-3 sentence overview of the system health. Be direct and reassuring where appropriate.",
    )
}

/// Build a prompt asking the LLM to explain why a specific action is safe.
pub fn safety_prompt(recommendation: &Recommendation) -> String {
    format!(
        "You are a system administrator explaining why an action is safe.\n\
         Proposed action: {title}\n\
         Target: {target}\n\
         Risk level: {risk}\n\
         Rollback plan: {rollback}\n\
         Explain in 2 sentences why this action is safe and reversible. \
         Be specific about the rollback mechanism.",
        title = recommendation.title,
        target = recommendation.target,
        risk = recommendation.risk_level,
        rollback = recommendation
            .rollback_plan
            .as_deref()
            .unwrap_or("No rollback plan specified"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use lso_core::{RecommendationStatus, RiskLevel};
    use uuid::Uuid;

    fn sample_rec() -> Recommendation {
        Recommendation {
            id: Uuid::new_v4(),
            rule_id: "disk.usage_high".into(),
            title: "Clean up /home".into(),
            description: "/home at 92% usage".into(),
            risk_level: RiskLevel::Low,
            category: "cleanup".into(),
            target: "/home".into(),
            rollback_plan: Some("Restore from snapshot".into()),
            status: RecommendationStatus::Pending,
            rejection_reason: None,
            created_at: Utc::now(),
            resolved_at: None,
        }
    }

    #[test]
    fn explanation_prompt_contains_title() {
        let rec = sample_rec();
        let prompt = explanation_prompt(&rec, "92% of 500 GB used");
        assert!(prompt.contains("Clean up /home"));
        assert!(prompt.contains("92% of 500 GB used"));
        assert!(prompt.contains("Low"));
    }

    #[test]
    fn explanation_prompt_has_guardrails() {
        let rec = sample_rec();
        let prompt = explanation_prompt(&rec, "context");
        assert!(prompt.contains("non-technical user"));
        assert!(prompt.contains("3 sentences"));
        assert!(prompt.contains("No jargon"));
    }

    #[test]
    fn summary_prompt_lists_findings() {
        let prompt = summary_prompt(&["High disk usage on /home", "Stale temp files"]);
        assert!(prompt.contains("1. High disk usage on /home"));
        assert!(prompt.contains("2. Stale temp files"));
    }

    #[test]
    fn safety_prompt_includes_rollback() {
        let rec = sample_rec();
        let prompt = safety_prompt(&rec);
        assert!(prompt.contains("Restore from snapshot"));
        assert!(prompt.contains("/home"));
    }

    #[test]
    fn safety_prompt_handles_no_rollback() {
        let mut rec = sample_rec();
        rec.rollback_plan = None;
        let prompt = safety_prompt(&rec);
        assert!(prompt.contains("No rollback plan specified"));
    }
}
