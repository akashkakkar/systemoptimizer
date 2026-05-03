#!/usr/bin/env bash
set -euo pipefail

# LSO Release Build Script
# Builds a distributable package for the current platform.
#
# Usage:
#   ./scripts/build-release.sh           # build for current platform
#   ./scripts/build-release.sh --check   # preflight checks only

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

cd "$PROJECT_ROOT"

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
info "Building LSO v${VERSION} for $(uname -s) ($(uname -m))"

# ── Preflight checks ──────────────────────────────────────────────────
info "Running preflight checks..."

command -v cargo >/dev/null 2>&1 || { error "cargo not found"; exit 1; }
command -v npm   >/dev/null 2>&1 || { error "npm not found";   exit 1; }

info "Checking Rust toolchain..."
cargo --version
rustc --version

info "Checking Node.js..."
node --version
npm --version

info "Installing frontend dependencies..."
npm ci --prefer-offline 2>/dev/null || npm install

info "Running frontend type check..."
npm run typecheck

info "Running Rust clippy..."
cargo clippy --workspace -- -D warnings

info "Running Rust tests..."
cargo test --workspace

if [ "${1:-}" = "--check" ]; then
    info "Preflight checks passed."
    exit 0
fi

# ── Build ──────────────────────────────────────────────────────────────
info "Building release..."
cargo tauri build --release 2>&1

# ── Post-build ─────────────────────────────────────────────────────────
BUNDLE_DIR="$PROJECT_ROOT/src-tauri/target/release/bundle"

if [ -d "$BUNDLE_DIR" ]; then
    info "Build artifacts:"
    find "$BUNDLE_DIR" -maxdepth 2 -type f \( -name "*.dmg" -o -name "*.AppImage" -o -name "*.deb" -o -name "*.msi" \) -exec ls -lh {} \;

    # Check binary size
    case "$(uname -s)" in
        Darwin)
            APP=$(find "$BUNDLE_DIR" -name "*.app" -maxdepth 2 | head -1)
            if [ -n "$APP" ]; then
                SIZE_MB=$(du -sm "$APP" | cut -f1)
                if [ "$SIZE_MB" -lt 50 ]; then
                    info "Binary size: ${SIZE_MB}MB (< 50MB target: PASS)"
                else
                    warn "Binary size: ${SIZE_MB}MB (>= 50MB target)"
                fi
            fi
            ;;
        Linux)
            BIN=$(find "$BUNDLE_DIR" -name "*.AppImage" | head -1)
            if [ -n "$BIN" ]; then
                SIZE_MB=$(du -sm "$BIN" | cut -f1)
                info "AppImage size: ${SIZE_MB}MB"
            fi
            ;;
    esac
else
    warn "Bundle directory not found at $BUNDLE_DIR"
fi

info "Build complete."
