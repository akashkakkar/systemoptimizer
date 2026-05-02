# F13 — First Actuator: Temp File Cleanup

**Phase:** 2 — Action
**Size:** S (one session)
**Branch:** `feat/P2-cleanup-actuator`
**Depends on:** F12

## Objective

Implement the first concrete actuator: cleaning temporary files. Low risk, highly reversible — ideal first action to validate the full pipeline.

## Targets

### Directories to Clean
- System temp (`/tmp`, `%TEMP%`)
- User cache (`~/Library/Caches`, `~/.cache`, `%LOCALAPPDATA%\Temp`)
- Application logs older than 30 days
- Trash/Recycle Bin (only with explicit approval)

### Safety Rules
- Never delete files modified in last 24 hours
- Never delete files currently open by a process (check via `lsof`/`handle.exe`)
- Calculate total size before execution, show to user
- Dry-run mode: report what would be deleted without deleting

## Deliverables

### 1. TempCleanupExecutor
```rust
impl ActionExecutor for TempCleanupExecutor {
    fn action_id(&self) -> &str { "cleanup.temp_files" }
    fn risk_level(&self) -> RiskLevel { RiskLevel::Low }
}
```

### 2. Preflight Report
Shows user:
- Number of files to delete
- Total size to reclaim
- Oldest/newest file in set
- Any files skipped (in use, too recent)

### 3. Tauri Command + UI
- "Clean Temp Files" card in recommendations
- Preflight summary dialog
- Progress bar during deletion
- Result summary (files deleted, space reclaimed)

## Acceptance Criteria

- [ ] Dry run correctly reports without deleting
- [ ] Snapshot created before actual deletion
- [ ] Files in use are skipped (not errored)
- [ ] Files < 24h old are never touched
- [ ] Progress reported to UI in real time
- [ ] Audit log entry with file count and bytes reclaimed
- [ ] Rollback restores deleted files from snapshot
