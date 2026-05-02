# F06 — Tauri Shell + Dashboard UI

**Phase:** 0 — Foundation
**Size:** M (1-2 sessions)
**Branch:** `feat/P0-tauri-shell`
**Depends on:** F02, F05

## Objective

Scaffold the Tauri v2 application with a React/TypeScript frontend. Display disk usage data from F05 in a basic dashboard. Establishes the full stack data flow: sensor → backend → IPC → frontend.

## Deliverables

### 1. Tauri v2 Project Init
```bash
cargo tauri init
# Configure: app name "LSO", window title "Local System Optimizer"
# Frontend: React + TypeScript + Vite
```

### 2. Tauri Security Config
- CSP: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'`
- No external URLs allowed
- Disable auto-updater
- Disable shell:open for arbitrary URLs

### 3. Tauri Commands (`src-tauri/src/commands.rs`)
```rust
#[tauri::command]
async fn get_disk_usage() -> Result<Vec<DiskUsageReport>, String> { ... }

#[tauri::command]
async fn get_app_version() -> String { ... }
```

### 4. Frontend Structure
```
src/
├── App.tsx              # Root layout
├── main.tsx             # Entry point
├── components/
│   ├── DashboardView.tsx    # Main dashboard
│   ├── DiskUsageCard.tsx    # Single disk metric card
│   └── MetricBar.tsx        # Usage bar component
├── hooks/
│   └── useSystemMetrics.ts  # Tauri IPC hook
├── stores/
│   └── metricsStore.ts      # Zustand store
├── lib/
│   └── tauri.ts             # IPC helpers
└── styles/
    └── index.css            # Tailwind entry
```

### 5. Dashboard Features
- System overview header (hostname, OS, uptime)
- Disk usage cards for each mount point
- Usage bar with color coding (green < 70%, yellow < 90%, red >= 90%)
- Last scan timestamp
- "Scan Now" button to re-run probes
- Loading and error states

### 6. Accessibility
- All interactive elements keyboard-navigable
- Color not the sole indicator (add text labels to usage bars)
- ARIA labels on all controls
- Minimum contrast ratios (WCAG 2.1 AA)

## Steps

1. Run `cargo tauri init` in project root
2. Set up Vite + React + TypeScript + Tailwind
3. Install Zustand: `npm install zustand`
4. Create Tauri commands that call sensor layer
5. Build frontend components
6. Wire up IPC: frontend calls backend, displays data
7. Test end-to-end: `cargo tauri dev`

## Acceptance Criteria

- [ ] `cargo tauri dev` launches app window
- [ ] Dashboard displays real disk usage data
- [ ] "Scan Now" refreshes data
- [ ] Loading spinner shown during scan
- [ ] Error state displayed if scan fails
- [ ] No external network requests (verify in dev tools)
- [ ] Window title shows "Local System Optimizer"
- [ ] Keyboard navigation works for all controls
