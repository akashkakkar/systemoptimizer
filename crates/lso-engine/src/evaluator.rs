//! Rule engine: loads rules from disk, evaluates probe data, produces recommendations.

use std::path::Path;

use chrono::Utc;
use lso_core::{Platform, ProbeResult, Recommendation, RecommendationStatus, SystemMetric};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::rule::{parse_rules_from_file, Operator, Rule};
use crate::EngineError;

/// A metric with its extracted target (the portion before `::` in the name).
struct TargetedMetric<'a> {
    target: &'a str,
    value: f64,
}

/// The deterministic rule engine.
///
/// Loads rules from TOML files and evaluates them against probe results
/// to produce actionable recommendations.
pub struct RuleEngine {
    rules: Vec<Rule>,
    platform: Platform,
}

impl RuleEngine {
    /// Load all `.toml` rule files from `rules_dir`.
    ///
    /// Files are read in alphabetical order. Rules are sorted by priority
    /// (highest first) after loading. Returns an error on the first file
    /// that fails to parse or validate.
    pub fn load_rules(rules_dir: &Path) -> Result<Self, EngineError> {
        let platform = Platform::detect().unwrap_or(Platform::Linux);
        Self::load_rules_for_platform(rules_dir, platform)
    }

    /// Load rules with an explicit platform (useful for testing).
    pub fn load_rules_for_platform(
        rules_dir: &Path,
        platform: Platform,
    ) -> Result<Self, EngineError> {
        let mut entries: Vec<_> = std::fs::read_dir(rules_dir)
            .map_err(|e| EngineError::ReadDir {
                path: rules_dir.display().to_string(),
                source: e,
            })?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        entries.sort();

        let mut rules = Vec::new();
        for path in &entries {
            let file_rules = parse_rules_from_file(path)?;
            debug!(
                path = %path.display(),
                count = file_rules.len(),
                "loaded rules from file"
            );
            rules.extend(file_rules);
        }

        rules.sort_by_key(|r| std::cmp::Reverse(r.metadata.priority));

        Ok(Self { rules, platform })
    }

    /// Create an engine from pre-loaded rules (useful for testing).
    pub fn from_rules(rules: Vec<Rule>, platform: Platform) -> Self {
        let mut rules = rules;
        rules.sort_by_key(|r| std::cmp::Reverse(r.metadata.priority));
        Self { rules, platform }
    }

    /// Returns the loaded rules.
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Evaluate all enabled, platform-matching rules against the given probe data.
    ///
    /// Returns recommendations sorted by priority (highest first).
    pub fn evaluate(&self, data: &[ProbeResult]) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                debug!(rule_id = %rule.id, "skipping disabled rule");
                continue;
            }

            if !self.matches_platform(rule) {
                debug!(rule_id = %rule.id, platform = ?self.platform, "skipping rule for other platform");
                continue;
            }

            let matching_probes: Vec<&ProbeResult> = data
                .iter()
                .filter(|p| p.probe_id == rule.condition.probe)
                .collect();

            if matching_probes.is_empty() {
                debug!(
                    rule_id = %rule.id,
                    probe = %rule.condition.probe,
                    "no probe data for rule"
                );
                continue;
            }

            for probe in &matching_probes {
                self.evaluate_rule_against_probe(rule, probe, &mut recommendations);
            }
        }

        recommendations
    }

    fn matches_platform(&self, rule: &Rule) -> bool {
        rule.metadata.platforms.is_empty() || rule.metadata.platforms.contains(&self.platform)
    }

    /// Evaluate a single rule against a probe result.
    ///
    /// Sensor metrics are named `"{target}::{metric_name}"` (e.g. `"/::usage_percent"`).
    /// The engine matches the suffix after `::` against `condition.metric` and extracts
    /// the prefix as the target for template resolution.
    fn evaluate_rule_against_probe(
        &self,
        rule: &Rule,
        probe: &ProbeResult,
        recommendations: &mut Vec<Recommendation>,
    ) {
        let matching_metrics = find_matching_metrics(&probe.metrics, &rule.condition.metric);

        match rule.condition.operator {
            Operator::Exists => {
                for m in &matching_metrics {
                    recommendations.push(self.build_recommendation(rule, m.target, m.value));
                }
            }
            Operator::NotExists => {
                if matching_metrics.is_empty() {
                    recommendations.push(self.build_recommendation(rule, &probe.probe_id, 0.0));
                }
            }
            Operator::Contains | Operator::NotContains => {
                warn!(
                    rule_id = %rule.id,
                    "contains/not_contains operators require string metrics; \
                     SystemMetric values are f64 — skipping"
                );
            }
            op => {
                for m in &matching_metrics {
                    if compare_numeric(m.value, op, rule.condition.threshold) {
                        recommendations
                            .push(self.build_recommendation(rule, m.target, m.value));
                    }
                }
            }
        }
    }

    fn build_recommendation(&self, rule: &Rule, target: &str, value: f64) -> Recommendation {
        let title = resolve_template(
            &rule.recommendation.title,
            target,
            value,
            rule.condition.threshold,
        );
        let description = resolve_template(
            &rule.recommendation.description,
            target,
            value,
            rule.condition.threshold,
        );

        Recommendation {
            id: Uuid::new_v4(),
            rule_id: rule.id.clone(),
            title,
            description,
            risk_level: rule.recommendation.risk_level,
            category: rule.recommendation.category.clone(),
            target: target.to_string(),
            rollback_plan: None,
            status: RecommendationStatus::Pending,
            rejection_reason: None,
            created_at: Utc::now(),
            resolved_at: None,
        }
    }
}

/// Find all metrics whose name ends with `::{suffix}` and extract target + value.
fn find_matching_metrics<'a>(metrics: &'a [SystemMetric], suffix: &str) -> Vec<TargetedMetric<'a>> {
    let expected_suffix = format!("::{suffix}");
    metrics
        .iter()
        .filter_map(|m| {
            m.name.strip_suffix(&expected_suffix).map(|target| TargetedMetric {
                target,
                value: m.value,
            })
        })
        .collect()
}

fn compare_numeric(value: f64, operator: Operator, threshold: f64) -> bool {
    match operator {
        Operator::GreaterThan => value > threshold,
        Operator::LessThan => value < threshold,
        Operator::EqualTo => (value - threshold).abs() < f64::EPSILON,
        Operator::GreaterThanOrEqual => {
            value > threshold || (value - threshold).abs() < f64::EPSILON
        }
        Operator::LessThanOrEqual => {
            value < threshold || (value - threshold).abs() < f64::EPSILON
        }
        _ => false,
    }
}

/// Replace `{target}`, `{value}`, and `{threshold}` in a template string.
fn resolve_template(template: &str, target: &str, value: f64, threshold: f64) -> String {
    template
        .replace("{target}", target)
        .replace("{value}", &format!("{value:.1}"))
        .replace("{threshold}", &format!("{threshold:.0}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;
    use lso_core::RiskLevel;

    use crate::rule::{Condition, RuleMetadata, RuleRecommendation};

    fn make_rule(id: &str, metric: &str, op: Operator, threshold: f64, priority: u32) -> Rule {
        Rule {
            id: id.to_string(),
            name: format!("Rule {id}"),
            description: "test".to_string(),
            category: "disk".to_string(),
            enabled: true,
            condition: Condition {
                probe: "disk.usage".to_string(),
                metric: metric.to_string(),
                operator: op,
                threshold,
            },
            recommendation: RuleRecommendation {
                title: "Action needed on {target}".to_string(),
                description: "{target} is at {value} (threshold: {threshold})".to_string(),
                risk_level: RiskLevel::Low,
                category: "cleanup".to_string(),
            },
            metadata: RuleMetadata {
                platforms: vec![],
                tags: vec![],
                priority,
            },
        }
    }

    /// Build a ProbeResult with metrics in the sensor's `"{target}::{name}"` format.
    fn make_probe(probe_id: &str, metrics: Vec<(&str, &str, f64)>) -> ProbeResult {
        let now = Utc::now();
        ProbeResult {
            probe_id: probe_id.to_string(),
            metrics: metrics
                .into_iter()
                .map(|(target, name, value)| SystemMetric {
                    id: Uuid::new_v4(),
                    probe_id: probe_id.to_string(),
                    name: format!("{target}::{name}"),
                    value,
                    unit: None,
                    collected_at: now,
                    platform: "macos".to_string(),
                })
                .collect(),
            collected_at: now,
            platform: Platform::MacOS,
        }
    }

    #[test]
    fn evaluate_triggers_on_threshold_exceeded() {
        let rules = vec![make_rule(
            "disk.high",
            "usage_percent",
            Operator::GreaterThan,
            85.0,
            50,
        )];
        let engine = RuleEngine::from_rules(rules, Platform::MacOS);

        let data = vec![make_probe(
            "disk.usage",
            vec![("/", "usage_percent", 92.0)],
        )];

        let recs = engine.evaluate(&data);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].target, "/");
        assert!(recs[0].title.contains("/"));
    }

    #[test]
    fn evaluate_no_trigger_below_threshold() {
        let rules = vec![make_rule(
            "disk.high",
            "usage_percent",
            Operator::GreaterThan,
            85.0,
            50,
        )];
        let engine = RuleEngine::from_rules(rules, Platform::MacOS);

        let data = vec![make_probe(
            "disk.usage",
            vec![("/", "usage_percent", 50.0)],
        )];

        assert!(engine.evaluate(&data).is_empty());
    }

    #[test]
    fn evaluate_disabled_rule_skipped() {
        let mut rule = make_rule("disk.high", "usage_percent", Operator::GreaterThan, 85.0, 50);
        rule.enabled = false;
        let engine = RuleEngine::from_rules(vec![rule], Platform::MacOS);

        let data = vec![make_probe(
            "disk.usage",
            vec![("/", "usage_percent", 99.0)],
        )];

        assert!(engine.evaluate(&data).is_empty());
    }

    #[test]
    fn evaluate_platform_filtering() {
        let mut rule = make_rule("disk.high", "usage_percent", Operator::GreaterThan, 85.0, 50);
        rule.metadata.platforms = vec![Platform::Linux];
        let engine = RuleEngine::from_rules(vec![rule], Platform::MacOS);

        let data = vec![make_probe(
            "disk.usage",
            vec![("/", "usage_percent", 99.0)],
        )];

        assert!(engine.evaluate(&data).is_empty());
    }

    #[test]
    fn evaluate_platform_empty_matches_all() {
        let rule = make_rule("disk.high", "usage_percent", Operator::GreaterThan, 85.0, 50);
        let engine = RuleEngine::from_rules(vec![rule], Platform::Windows);

        let data = vec![make_probe(
            "disk.usage",
            vec![("C:\\", "usage_percent", 99.0)],
        )];

        assert_eq!(engine.evaluate(&data).len(), 1);
    }

    #[test]
    fn evaluate_no_data_no_recommendations() {
        let rules = vec![make_rule(
            "disk.high",
            "usage_percent",
            Operator::GreaterThan,
            85.0,
            50,
        )];
        let engine = RuleEngine::from_rules(rules, Platform::MacOS);
        assert!(engine.evaluate(&[]).is_empty());
    }

    #[test]
    fn evaluate_all_pass_below_threshold() {
        let rules = vec![
            make_rule("r1", "usage_percent", Operator::GreaterThan, 85.0, 50),
            make_rule("r2", "usage_percent", Operator::GreaterThan, 50.0, 40),
        ];
        let engine = RuleEngine::from_rules(rules, Platform::MacOS);

        let data = vec![make_probe(
            "disk.usage",
            vec![("/", "usage_percent", 10.0)],
        )];

        assert!(engine.evaluate(&data).is_empty());
    }

    #[test]
    fn evaluate_all_fail_above_threshold() {
        let rules = vec![
            make_rule("r1", "usage_percent", Operator::GreaterThan, 5.0, 80),
            make_rule("r2", "usage_percent", Operator::GreaterThan, 3.0, 60),
        ];
        let engine = RuleEngine::from_rules(rules, Platform::MacOS);

        let data = vec![make_probe(
            "disk.usage",
            vec![("/", "usage_percent", 10.0)],
        )];

        let recs = engine.evaluate(&data);
        assert_eq!(recs.len(), 2);
        // Priority ordering preserved: r1 (80) before r2 (60)
        assert!(recs[0].title.contains("/"));
        assert!(recs[1].title.contains("/"));
    }

    #[test]
    fn template_resolution() {
        let result = resolve_template(
            "Volume {target} is {value}% full (limit: {threshold}%)",
            "/dev/disk1",
            92.3,
            85.0,
        );
        assert_eq!(result, "Volume /dev/disk1 is 92.3% full (limit: 85%)");
    }

    #[test]
    fn evaluate_less_than() {
        let rules = vec![make_rule(
            "disk.low",
            "free_gb",
            Operator::LessThan,
            10.0,
            50,
        )];
        let engine = RuleEngine::from_rules(rules, Platform::MacOS);

        let data = vec![make_probe("disk.usage", vec![("/", "free_gb", 5.0)])];

        assert_eq!(engine.evaluate(&data).len(), 1);
    }

    #[test]
    fn evaluate_equal_to() {
        let rules = vec![make_rule("exact", "count", Operator::EqualTo, 42.0, 50)];
        let engine = RuleEngine::from_rules(rules, Platform::MacOS);

        let data = vec![make_probe("disk.usage", vec![("/", "count", 42.0)])];

        assert_eq!(engine.evaluate(&data).len(), 1);
    }

    #[test]
    fn evaluate_gte_lte() {
        let gte_rule = make_rule("gte", "v", Operator::GreaterThanOrEqual, 10.0, 50);
        let lte_rule = make_rule("lte", "v", Operator::LessThanOrEqual, 10.0, 40);
        let engine = RuleEngine::from_rules(vec![gte_rule, lte_rule], Platform::MacOS);

        let data = vec![make_probe("disk.usage", vec![("/", "v", 10.0)])];

        let recs = engine.evaluate(&data);
        assert_eq!(recs.len(), 2);
    }

    #[test]
    fn evaluate_exists() {
        let rule = make_rule("ex", "some_flag", Operator::Exists, 0.0, 50);
        let engine = RuleEngine::from_rules(vec![rule], Platform::MacOS);

        let data = vec![make_probe(
            "disk.usage",
            vec![("/", "some_flag", 1.0)],
        )];

        assert_eq!(engine.evaluate(&data).len(), 1);
    }

    #[test]
    fn evaluate_not_exists() {
        let rule = make_rule("nex", "missing_metric", Operator::NotExists, 0.0, 50);
        let engine = RuleEngine::from_rules(vec![rule], Platform::MacOS);

        let data = vec![make_probe(
            "disk.usage",
            vec![("/", "other_metric", 1.0)],
        )];

        assert_eq!(engine.evaluate(&data).len(), 1);
    }

    #[test]
    fn evaluate_missing_metric_no_trigger() {
        let rules = vec![make_rule(
            "disk.high",
            "usage_percent",
            Operator::GreaterThan,
            85.0,
            50,
        )];
        let engine = RuleEngine::from_rules(rules, Platform::MacOS);

        let data = vec![make_probe(
            "disk.usage",
            vec![("/", "wrong_metric", 99.0)],
        )];

        assert!(engine.evaluate(&data).is_empty());
    }

    #[test]
    fn evaluate_multiple_targets_in_single_probe() {
        let rules = vec![make_rule(
            "disk.high",
            "usage_percent",
            Operator::GreaterThan,
            85.0,
            50,
        )];
        let engine = RuleEngine::from_rules(rules, Platform::MacOS);

        let data = vec![make_probe(
            "disk.usage",
            vec![
                ("/", "usage_percent", 30.0),
                ("/home", "usage_percent", 92.0),
            ],
        )];

        let recs = engine.evaluate(&data);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].target, "/home");
    }

    #[test]
    fn evaluate_recommendation_fields_populated() {
        let rules = vec![make_rule(
            "disk.high",
            "usage_percent",
            Operator::GreaterThan,
            85.0,
            50,
        )];
        let engine = RuleEngine::from_rules(rules, Platform::MacOS);

        let data = vec![make_probe(
            "disk.usage",
            vec![("/", "usage_percent", 92.0)],
        )];

        let recs = engine.evaluate(&data);
        assert_eq!(recs.len(), 1);
        let rec = &recs[0];
        assert_eq!(rec.target, "/");
        assert_eq!(rec.risk_level, RiskLevel::Low);
        assert_eq!(rec.category, "cleanup");
        assert_eq!(rec.status, RecommendationStatus::Pending);
        assert!(rec.resolved_at.is_none());
        assert!(rec.rollback_plan.is_none());
    }

    #[test]
    fn load_rules_from_directory() {
        let dir = tempfile::tempdir().unwrap();
        let rule_content = r#"
[[rules]]
id = "test.a"
name = "Test A"
description = "desc"
category = "test"

[rules.condition]
probe = "test.probe"
metric = "val"
operator = "greater_than"
threshold = 10.0

[rules.recommendation]
title = "Fix {target}"
description = "Value is {value}"
risk_level = "low"
category = "info"
"#;
        std::fs::write(dir.path().join("test.toml"), rule_content).unwrap();
        std::fs::write(dir.path().join("readme.md"), "ignore me").unwrap();

        let engine = RuleEngine::load_rules_for_platform(dir.path(), Platform::MacOS).unwrap();
        assert_eq!(engine.rules().len(), 1);
        assert_eq!(engine.rules()[0].id, "test.a");
    }

    #[test]
    fn load_rules_nonexistent_dir() {
        let result = RuleEngine::load_rules(Path::new("/nonexistent/path/rules"));
        assert!(result.is_err());
    }
}
