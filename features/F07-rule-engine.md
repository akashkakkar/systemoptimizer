# F07 — Rule Engine + TOML Schema

**Phase:** 1 — Intelligence
**Size:** M (1-2 sessions)
**Branch:** `feat/P1-rule-engine`
**Depends on:** F03, F05

## Objective

Build the deterministic rule engine that evaluates sensor data against defined rules and produces recommendations. Rules defined in TOML — no code changes needed to add new rules.

## TOML Rule Schema

```toml
[rule]
id = "disk.usage_high"
name = "High disk usage detected"
description = "A volume is using more than {threshold}% of capacity"
category = "disk"
enabled = true

[rule.condition]
probe = "disk.usage"
metric = "disk.usage_percent"
operator = "greater_than"
threshold = 85.0

[rule.recommendation]
title = "Free up disk space on {target}"
description = "Volume {target} is {value}% full. Consider removing unused files, emptying trash, or moving data to external storage."
risk_level = "low"
action_type = "cleanup"

[rule.metadata]
platforms = ["macos", "linux", "windows"]  # empty = all platforms
tags = ["disk", "storage", "cleanup"]
priority = 50  # Higher = evaluated first
```

### Supported Operators
- `greater_than`, `less_than`, `equal_to`
- `greater_than_or_equal`, `less_than_or_equal`
- `contains`, `not_contains` (for string metrics)
- `exists`, `not_exists` (for boolean checks)

## Deliverables

### 1. Rule Parser (`lso-engine/src/rule.rs`)
- Parse TOML rule files from `rules/` directory
- Validate schema on load (report errors with file + line)
- Hot-reloadable: watch `rules/` for changes (optional, Phase 4)

### 2. Rule Evaluator (`lso-engine/src/evaluator.rs`)
```rust
pub struct RuleEngine {
    rules: Vec<Rule>,
}

impl RuleEngine {
    pub fn load_rules(rules_dir: &Path) -> Result<Self, EngineError>;
    pub fn evaluate(&self, data: &[ProbeResult]) -> Vec<Recommendation>;
}
```

### 3. Built-in Rules (ship with app)
- `disk.usage_high` — disk > 85% full
- `disk.usage_critical` — disk > 95% full
- `disk.tmp_large` — temp directory > 1GB

### 4. Rule Directory Structure
```
rules/
├── disk.toml        # Disk-related rules
├── security.toml    # Security rules (Phase 4)
├── startup.toml     # Startup items (Phase 4)
└── custom/          # User-added rules
```

## Steps

1. Define Rust types for Rule, Condition, Operator
2. Implement TOML parser with validation
3. Implement evaluator that matches probe results against conditions
4. Create 3 built-in disk rules
5. Wire into sensor pipeline: sensor → engine → recommendations
6. Write tests: rule parsing, evaluation, edge cases (no data, all pass, all fail)

## Acceptance Criteria

- [ ] Rules load from TOML without code changes
- [ ] Invalid TOML gives clear error messages with file/line info
- [ ] Evaluator correctly matches metrics against conditions
- [ ] Platform filtering works (macOS rule skipped on Linux)
- [ ] Recommendations include the specific target (mount point, etc.)
- [ ] Template variables (`{target}`, `{value}`) resolved in output
- [ ] Disabled rules skipped
- [ ] Rules sorted by priority before evaluation
