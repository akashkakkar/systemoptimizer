# F08 — Recommendation Generator

**Phase:** 1 — Intelligence
**Size:** S (one session)
**Branch:** `feat/P1-recommendations`
**Depends on:** F07, F04

## Objective

Transform raw rule evaluations into actionable, stored recommendations with risk scoring, deduplication, and lifecycle management.

## Deliverables

### 1. Recommendation Manager (`lso-engine/src/recommendation.rs`)
```rust
pub struct RecommendationManager {
    db: Database,
    engine: RuleEngine,
}

impl RecommendationManager {
    /// Run all probes, evaluate rules, generate and store recommendations
    pub async fn scan(&self) -> Result<Vec<Recommendation>, EngineError>;

    /// Deduplicate: same rule + same target = update, don't duplicate
    pub fn deduplicate(&self, new: &[Recommendation]) -> Vec<Recommendation>;

    /// Expire recommendations whose underlying condition resolved
    pub async fn expire_resolved(&self, current_data: &[ProbeResult]) -> Result<usize>;
}
```

### 2. Recommendation Lifecycle
```
Created → Pending → [Approved | Rejected | Expired]
                         ↓
                     Executing → [Succeeded | Failed]
                                      ↓ (if failed)
                                  RolledBack
```

### 3. Risk Score Calculation
- Base risk from rule definition
- Modifiers: system drive → +1 level, boot partition → Critical (blocked)
- Final score determines approval type (single/double confirm)

### 4. Tauri Command
```rust
#[tauri::command]
async fn scan_system() -> Result<Vec<Recommendation>, String>;

#[tauri::command]
async fn get_recommendations() -> Result<Vec<Recommendation>, String>;

#[tauri::command]
async fn dismiss_recommendation(id: String) -> Result<(), String>;
```

## Steps

1. Implement `RecommendationManager` with deduplication logic
2. Add lifecycle state machine to `Recommendation` type
3. Implement risk score modifiers
4. Store/retrieve from database (F04)
5. Expose via Tauri commands
6. Add recommendation list to frontend dashboard
7. Test: scan produces correct recommendations from mock probe data

## Acceptance Criteria

- [ ] Scan produces recommendations from sensor data + rules
- [ ] Duplicate recommendations merged (not duplicated on re-scan)
- [ ] Expired recommendations auto-resolved when condition clears
- [ ] Risk level correctly elevated for system-critical targets
- [ ] Recommendations persist across app restarts (stored in DB)
- [ ] Frontend displays recommendation list with risk badges
