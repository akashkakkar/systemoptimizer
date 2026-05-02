# F21 — Cross-Platform Test Matrix

**Phase:** 4 — Polish
**Size:** M (1-2 sessions)
**Branch:** `feat/P4-platform-testing`
**Depends on:** All previous features

## Objective

Verify all sensors, actuators, and UI work correctly on macOS, Linux, and Windows. Fill in platform stubs.

## Test Matrix

| Component | macOS | Linux | Windows |
|---|---|---|---|
| Disk sensor | ○ | ● | ○ |
| Process sensor | ○ | ● | ○ |
| Memory sensor | ○ | ● | ○ |
| CPU sensor | ○ | ● | ○ |
| Startup sensor | ○ | ○ | ○ |
| Security scanner | ○ | ○ | ○ |
| Snapshot (create) | ○ | ○ | ○ |
| Snapshot (restore) | ○ | ○ | ○ |
| Temp cleanup | ○ | ● | ○ |
| Database | ● | ● | ● |
| Tauri UI | ○ | ● | ○ |

● = Implemented  ○ = Needs implementation/testing

## Approach

1. Implement macOS-specific code for each sensor (dev has macOS)
2. Windows implementations via `windows` crate, tested in VM or CI
3. Integration tests per platform behind `#[cfg(target_os)]`
4. Shared tests via mock `PlatformProvider`

## Deliverables

### 1. Platform Test Harness
```rust
/// Mock platform provider for testing
pub struct MockPlatform {
    pub platform: Platform,
    pub mock_data: HashMap<String, Vec<u8>>,
}
```

### 2. Per-Platform Integration Tests
- `tests/macos/` — run on macOS only
- `tests/linux/` — run on Linux only
- `tests/windows/` — run on Windows only
- `tests/common/` — run everywhere with mocks

### 3. Platform Parity Checklist
- Fill all `PlatformNotSupported` stubs
- Verify path resolution on each OS
- Test privilege escalation prompts
- Verify Tauri IPC on each OS webview

## Acceptance Criteria

- [ ] All sensors return data on all 3 platforms
- [ ] All actuators execute on all 3 platforms
- [ ] Snapshot create/restore works on all 3 platforms
- [ ] UI renders correctly on all 3 platform webviews
- [ ] No `PlatformNotSupported` errors for supported features
- [ ] Test matrix fully green
