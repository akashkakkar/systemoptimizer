# F14 — Audit Log + History UI

**Phase:** 2 — Action
**Size:** S (one session)
**Branch:** `feat/P2-audit-log`
**Depends on:** F04, F12

## Objective

Surface the audit trail in the UI. Every action taken (or rejected) is visible, searchable, and exportable.

## Deliverables

### 1. Audit Log View (`AuditLogView.tsx`)
- Chronological list of all actions
- Filter by: status (success/failed/rolled-back), risk level, category, date range
- Search by target name
- Each entry expandable: full detail including snapshot ID, timestamps, result

### 2. Rollback Action
- "Rollback" button on entries where `rollback_available: true`
- Triggers snapshot restore (F11)
- Creates new audit entry for the rollback itself

### 3. Export
- Export audit log as JSON or CSV
- Date range selection
- Opened via system file dialog (Tauri)

### 4. Tauri Commands
```rust
#[tauri::command]
async fn get_audit_log(filter: AuditFilter) -> Result<Vec<AuditEntry>, String>;

#[tauri::command]
async fn export_audit_log(format: ExportFormat, path: String) -> Result<(), String>;

#[tauri::command]
async fn rollback_action(audit_id: String) -> Result<(), String>;
```

## Acceptance Criteria

- [ ] All actions appear in audit log immediately
- [ ] Filters work correctly (AND logic)
- [ ] Rollback triggers snapshot restore and logs itself
- [ ] Export produces valid JSON/CSV
- [ ] Audit log is append-only (no delete from UI, only export+clear)
- [ ] Pagination for large histories (50 entries per page)
