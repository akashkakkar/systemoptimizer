# F12 — Actuator Framework

**Phase:** 2 — Action
**Size:** S (one session)
**Branch:** `feat/P2-actuator`
**Depends on:** F09, F11

## Objective

Build the gated executor framework. Actuators perform system changes only after approval gate clearance and snapshot creation.

## Deliverables

### 1. Actuator Trait (`lso-core/src/traits.rs`)
```rust
#[async_trait]
pub trait ActionExecutor: Send + Sync {
    fn action_id(&self) -> &str;
    fn description(&self) -> &str;
    fn risk_level(&self) -> RiskLevel;
    fn required_privilege(&self) -> PrivilegeLevel;

    /// Validate preconditions before execution
    async fn preflight(&self, target: &str) -> Result<PreflightReport, ActuatorError>;

    /// Execute the action (only called after approval + snapshot)
    async fn execute(&self, target: &str) -> Result<ActionResult, ActuatorError>;

    /// Verify the action succeeded
    async fn verify(&self, target: &str) -> Result<VerifyResult, ActuatorError>;
}
```

### 2. Execution Pipeline (`lso-actuator/src/pipeline.rs`)
```
1. Check approval status (must be Approved)
2. Run preflight checks
3. Create snapshot (F11)
4. Execute action
5. Verify result
6. Log to audit (F04)
7. If verify fails → auto-rollback to snapshot
```

### 3. Actuator Registry
```rust
pub struct ActuatorRegistry {
    executors: HashMap<String, Box<dyn ActionExecutor>>,
}
```

## Acceptance Criteria

- [ ] Unapproved actions are rejected at step 1
- [ ] Snapshot always created before execution
- [ ] Failed verification triggers automatic rollback
- [ ] Full audit trail logged for every execution
- [ ] Pipeline handles errors at every step without panic
- [ ] Privilege escalation requested only when executor needs it
