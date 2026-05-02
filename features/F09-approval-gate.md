# F09 — Approval Gate UI

**Phase:** 1 — Intelligence
**Size:** S (one session)
**Branch:** `feat/P1-approval-gate`
**Depends on:** F08, F06

## Objective

Build the UI flow where users review, approve, or reject recommendations. This is the critical trust boundary — no action executes without passing through this gate.

## Deliverables

### 1. Approval Dialog (`ApprovalDialog.tsx`)
Displays for each recommendation:
- What will change (plain English)
- Why (rule that triggered it)
- Risk level badge (color + text)
- What will be backed up/snapshotted
- How to rollback
- Approve / Reject buttons
- For High risk: "Are you sure?" second confirmation step

### 2. Recommendation List View (`RecommendationList.tsx`)
- Grouped by category (disk, security, startup)
- Sorted by risk level (critical first)
- Filter: All / Pending / Approved / Rejected
- Bulk reject (but NEVER bulk approve — per ADR-004)

### 3. Tauri Commands
```rust
#[tauri::command]
async fn approve_recommendation(id: String) -> Result<ApprovalResult, String>;

#[tauri::command]
async fn reject_recommendation(id: String, reason: Option<String>) -> Result<(), String>;
```

### 4. Approval Flow State Machine (Frontend)
```
Viewing List → Click Approve → Show Detail Dialog → Confirm
                                                    ↓ (High risk)
                                              Double Confirm
                                                    ↓
                                              Approved (queued for execution)
```

## Steps

1. Build `RecommendationList` component with mock data
2. Build `ApprovalDialog` component
3. Implement double-confirm flow for High risk
4. Wire to Tauri commands
5. Store approval/rejection in audit log (F04)
6. Test: approve flow, reject flow, double-confirm flow

## Acceptance Criteria

- [ ] Each recommendation shows full context before approval
- [ ] Single confirm for Low/Medium risk
- [ ] Double confirm for High risk
- [ ] Critical risk blocked entirely (shown but not approvable unless expert mode)
- [ ] Reject stores optional reason in audit log
- [ ] No "approve all" button exists
- [ ] Keyboard accessible (Enter to confirm, Escape to cancel)
- [ ] Dialog is modal — cannot interact with app while open
