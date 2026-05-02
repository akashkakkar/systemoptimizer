//! Rule types and TOML parsing with validation.

use std::path::Path;

use lso_core::{Platform, RiskLevel};
use serde::Deserialize;

use crate::EngineError;

/// Comparison operator for evaluating a metric against a threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    GreaterThan,
    LessThan,
    EqualTo,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    NotContains,
    Exists,
    NotExists,
}

/// The condition that triggers a rule.
#[derive(Debug, Clone, Deserialize)]
pub struct Condition {
    /// Which probe this condition applies to (e.g. "disk.usage").
    pub probe: String,
    /// The metric name suffix to match (e.g. "usage_percent").
    ///
    /// Matched against the portion after `::` in `SystemMetric.name`.
    /// For example, a sensor metric named `"/::usage_percent"` matches
    /// a condition with `metric = "usage_percent"`.
    pub metric: String,
    /// How to compare the metric value against the threshold.
    pub operator: Operator,
    /// Threshold value for numeric comparisons.
    #[serde(default)]
    pub threshold: f64,
}

/// Template for the recommendation that a triggered rule produces.
#[derive(Debug, Clone, Deserialize)]
pub struct RuleRecommendation {
    /// Title template (may contain `{target}`, `{value}`, `{threshold}`).
    pub title: String,
    /// Description template (may contain `{target}`, `{value}`, `{threshold}`).
    pub description: String,
    /// Risk level of the recommended action.
    pub risk_level: RiskLevel,
    /// Category string for grouping (e.g. "cleanup", "security").
    pub category: String,
}

/// Optional metadata attached to a rule.
#[derive(Debug, Clone, Deserialize)]
pub struct RuleMetadata {
    /// Platforms this rule applies to. Empty means all platforms.
    #[serde(default)]
    pub platforms: Vec<Platform>,
    /// Freeform tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Evaluation priority. Higher values are evaluated first.
    #[serde(default = "default_priority")]
    pub priority: u32,
}

fn default_priority() -> u32 {
    50
}

impl Default for RuleMetadata {
    fn default() -> Self {
        Self {
            platforms: Vec::new(),
            tags: Vec::new(),
            priority: default_priority(),
        }
    }
}

/// A complete rule definition as parsed from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    /// Unique rule identifier (e.g. "disk.usage_high").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of what this rule detects.
    pub description: String,
    /// Category for grouping (e.g. "disk", "security").
    pub category: String,
    /// Whether this rule is active.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// The condition to evaluate.
    pub condition: Condition,
    /// Template for the recommendation.
    pub recommendation: RuleRecommendation,
    /// Optional metadata.
    #[serde(default)]
    pub metadata: RuleMetadata,
}

fn default_enabled() -> bool {
    true
}

/// Wrapper for TOML deserialization — a file contains one `[rule]` table.
#[derive(Debug, Deserialize)]
struct SingleRuleFile {
    rule: Rule,
}

/// Wrapper for TOML deserialization — a file contains multiple `[[rules]]` entries.
#[derive(Debug, Deserialize)]
struct MultiRuleFile {
    rules: Vec<Rule>,
}

/// Parse a TOML file that contains either a single `[rule]` or multiple `[[rules]]`.
pub fn parse_rules_from_file(path: &Path) -> Result<Vec<Rule>, EngineError> {
    let path_str = path.display().to_string();
    let content = std::fs::read_to_string(path).map_err(|e| EngineError::ReadFile {
        path: path_str.clone(),
        source: e,
    })?;

    parse_rules_from_str(&content, &path_str)
}

/// Parse rules from a TOML string, using `path_str` for error messages.
pub fn parse_rules_from_str(content: &str, path_str: &str) -> Result<Vec<Rule>, EngineError> {
    // Try multi-rule format first ([[rules]]), then single-rule ([rule]).
    if let Ok(multi) = toml::from_str::<MultiRuleFile>(content) {
        validate_rules(&multi.rules, path_str)?;
        return Ok(multi.rules);
    }

    match toml::from_str::<SingleRuleFile>(content) {
        Ok(single) => {
            validate_rules(std::slice::from_ref(&single.rule), path_str)?;
            Ok(vec![single.rule])
        }
        Err(e) => Err(EngineError::TomlParse {
            path: path_str.to_string(),
            source: e,
        }),
    }
}

fn validate_rules(rules: &[Rule], path: &str) -> Result<(), EngineError> {
    for rule in rules {
        if rule.id.is_empty() {
            return Err(EngineError::Validation {
                path: path.to_string(),
                message: "rule.id must not be empty".to_string(),
            });
        }
        if rule.name.is_empty() {
            return Err(EngineError::Validation {
                path: path.to_string(),
                message: format!("rule '{}': name must not be empty", rule.id),
            });
        }
        if rule.condition.probe.is_empty() {
            return Err(EngineError::Validation {
                path: path.to_string(),
                message: format!("rule '{}': condition.probe must not be empty", rule.id),
            });
        }
        if rule.condition.metric.is_empty() {
            return Err(EngineError::Validation {
                path: path.to_string(),
                message: format!("rule '{}': condition.metric must not be empty", rule.id),
            });
        }
        if rule.recommendation.title.is_empty() {
            return Err(EngineError::Validation {
                path: path.to_string(),
                message: format!("rule '{}': recommendation.title must not be empty", rule.id),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_SINGLE_RULE: &str = r#"
[rule]
id = "disk.usage_high"
name = "High disk usage"
description = "Disk usage exceeds {threshold}%"
category = "disk"
enabled = true

[rule.condition]
probe = "disk.usage"
metric = "usage_percent"
operator = "greater_than"
threshold = 85.0

[rule.recommendation]
title = "Free up space on {target}"
description = "Volume {target} is {value}% full."
risk_level = "low"
category = "cleanup"

[rule.metadata]
platforms = ["macos", "linux", "windows"]
tags = ["disk", "storage"]
priority = 50
"#;

    #[test]
    fn parse_valid_single_rule() {
        let rules = parse_rules_from_str(VALID_SINGLE_RULE, "test.toml").unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "disk.usage_high");
        assert_eq!(rules[0].condition.operator, Operator::GreaterThan);
        assert!((rules[0].condition.threshold - 85.0).abs() < f64::EPSILON);
        assert_eq!(rules[0].metadata.platforms.len(), 3);
        assert_eq!(rules[0].metadata.priority, 50);
    }

    #[test]
    fn parse_valid_multi_rule() {
        let toml = r#"
[[rules]]
id = "a"
name = "Rule A"
description = "desc"
category = "disk"

[rules.condition]
probe = "disk.usage"
metric = "pct"
operator = "greater_than"
threshold = 50.0

[rules.recommendation]
title = "Title"
description = "Desc"
risk_level = "low"
category = "cleanup"

[[rules]]
id = "b"
name = "Rule B"
description = "desc"
category = "disk"

[rules.condition]
probe = "disk.usage"
metric = "pct"
operator = "less_than"
threshold = 10.0

[rules.recommendation]
title = "Title"
description = "Desc"
risk_level = "medium"
category = "info"
"#;
        let rules = parse_rules_from_str(toml, "multi.toml").unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].id, "a");
        assert_eq!(rules[1].id, "b");
    }

    #[test]
    fn parse_error_empty_id() {
        let toml = r#"
[rule]
id = ""
name = "Bad rule"
description = "desc"
category = "disk"

[rule.condition]
probe = "disk.usage"
metric = "m"
operator = "greater_than"

[rule.recommendation]
title = "T"
description = "D"
risk_level = "low"
category = "info"
"#;
        let err = parse_rules_from_str(toml, "bad.toml").unwrap_err();
        assert!(err.to_string().contains("id must not be empty"));
    }

    #[test]
    fn parse_error_invalid_toml() {
        let err = parse_rules_from_str("not valid toml {{{{", "garbage.toml").unwrap_err();
        assert!(err.to_string().contains("garbage.toml"));
    }

    #[test]
    fn parse_error_missing_fields() {
        let toml = r#"
[rule]
id = "test"
"#;
        let err = parse_rules_from_str(toml, "incomplete.toml").unwrap_err();
        assert!(err.to_string().contains("incomplete.toml"));
    }

    #[test]
    fn disabled_rule_parses() {
        let toml = r#"
[rule]
id = "test.disabled"
name = "Disabled"
description = "desc"
category = "disk"
enabled = false

[rule.condition]
probe = "disk.usage"
metric = "m"
operator = "exists"

[rule.recommendation]
title = "T"
description = "D"
risk_level = "low"
category = "info"
"#;
        let rules = parse_rules_from_str(toml, "test.toml").unwrap();
        assert!(!rules[0].enabled);
    }

    #[test]
    fn defaults_applied() {
        let toml = r#"
[rule]
id = "test.defaults"
name = "Defaults"
description = "desc"
category = "disk"

[rule.condition]
probe = "disk.usage"
metric = "m"
operator = "exists"

[rule.recommendation]
title = "T"
description = "D"
risk_level = "low"
category = "info"
"#;
        let rules = parse_rules_from_str(toml, "test.toml").unwrap();
        assert!(rules[0].enabled);
        assert_eq!(rules[0].metadata.priority, 50);
        assert!(rules[0].metadata.platforms.is_empty());
    }
}
