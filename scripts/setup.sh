#!/usr/bin/env bash

# Copyright 2026 ResQ
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# Sets up the ResQ Programs (Solana/Anchor) development environment.
#
# Usage:
#   ./scripts/setup.sh [--check] [--yes] [--skip-keygen]
#
# Options:
#   --check        Verify the environment without making changes.
#   --yes          Auto-confirm all prompts (CI mode).
#   --skip-keygen  Skip wallet keypair generation.
#
# What this does:
#   1. Installs Nix with flakes support (if missing).
#   2. Re-enters inside `nix develop` — provides Rust, Node 22, Bun.
#   3. Installs Docker (if missing).
#   4. Installs Solana CLI via the official Anza installer (if missing).
#   5. Installs Anchor CLI via AVM + cargo (if missing).
#   6. Installs JS dependencies via bun.
#   7. Generates a local wallet keypair (if missing and not --skip-keygen).
#
# Requirements:
#   curl, git, bash 4+, cargo (provided by nix develop)
#
# Exit codes:
#   0  Success.
#   1  A required step failed.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# shellcheck source=lib/shell-utils.sh
source "${SCRIPT_DIR}/lib/shell-utils.sh"

# Pinned versions — bump here when upgrading
SOLANA_VERSION="${SOLANA_VERSION:-2.1.0}"
ANCHOR_VERSION="${ANCHOR_VERSION:-0.30.1}"

# ── Argument parsing ──────────────────────────────────────────────────────────
CHECK_ONLY=false
SKIP_KEYGEN=false
for arg in "$@"; do
    case "$arg" in
        --check)        CHECK_ONLY=true ;;
        --yes)          export YES=1 ;;
        --skip-keygen)  SKIP_KEYGEN=true ;;
        --help|-h)
            sed -n '/^# Usage/,/^$/p' "$0"
            exit 0
            ;;
    esac
done

# ── Helpers ───────────────────────────────────────────────────────────────────

# Installs Solana CLI via the official Anza installer.
#
# Args:
#   (none — uses $SOLANA_VERSION)
#
# Returns:
#   0 if already installed or successfully installed.
#   1 on failure.
install_solana() {
    if command_exists solana; then
        log_success "Solana CLI already installed: $(solana --version)"
        return 0
    fi

    log_info "Installing Solana CLI v${SOLANA_VERSION}..."
    sh -c "$(curl -sSfL "https://release.anza.xyz/v${SOLANA_VERSION}/install")"

    # Add to PATH for the current session
    export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

    if command_exists solana; then
        log_success "Solana CLI installed: $(solana --version)"
    else
        log_error "Solana CLI install succeeded but binary not found in PATH."
        log_info "Add this to your shell profile:"
        echo "  export PATH=\"\$HOME/.local/share/solana/install/active_release/bin:\$PATH\""
        return 1
    fi
}

# Installs Anchor CLI via AVM (Anchor Version Manager).
#
# Args:
#   (none — uses $ANCHOR_VERSION)
#
# Returns:
#   0 if already installed or successfully installed.
#   1 on failure.
install_anchor() {
    if command_exists anchor; then
        log_success "Anchor CLI already installed: $(anchor --version)"
        return 0
    fi

    if ! command_exists cargo; then
        log_error "cargo not found — Anchor requires Rust. Enter the nix dev shell first."
        return 1
    fi

    log_info "Installing AVM (Anchor Version Manager)..."
    cargo install --git https://github.com/coral-xyz/anchor avm --locked

    export PATH="$HOME/.cargo/bin:$PATH"

    log_info "Installing Anchor CLI v${ANCHOR_VERSION} via AVM..."
    avm install "${ANCHOR_VERSION}"
    avm use "${ANCHOR_VERSION}"

    if command_exists anchor; then
        log_success "Anchor CLI installed: $(anchor --version)"
    else
        log_error "Anchor install completed but binary not found in PATH."
        log_info "Add this to your shell profile:"
        echo "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
        return 1
    fi
}

# ── Check mode ────────────────────────────────────────────────────────────────
if [ "$CHECK_ONLY" = true ]; then
    log_info "Checking ResQ Programs environment..."
    ERRORS=0

    command_exists nix    || { log_error "nix not found";    ERRORS=$((ERRORS+1)); }
    command_exists rustc  || { log_warning "rustc not found (run: nix develop)"; }
    command_exists cargo  || { log_warning "cargo not found (run: nix develop)"; }
    command_exists node   || { log_warning "node not found (run: nix develop)"; }
    command_exists bun    || { log_warning "bun not found (run: nix develop)"; }
    command_exists solana || { log_warning "solana not found (run: scripts/setup.sh)"; }
    command_exists anchor || { log_warning "anchor not found (run: scripts/setup.sh)"; }
    command_exists docker || { log_warning "docker not found"; }

    if [ -f "$HOME/.config/solana/id.json" ]; then
        log_success "Wallet keypair: $HOME/.config/solana/id.json"
    else
        log_warning "No wallet keypair found at ~/.config/solana/id.json"
    fi

    [ $ERRORS -eq 0 ] && log_success "Environment looks good." || exit 1
    exit 0
fi

# ── Main setup ────────────────────────────────────────────────────────────────
echo "╔══════════════════════════════════════════╗"
echo "║  ResQ Programs — Environment Setup       ║"
echo "╚══════════════════════════════════════════╝"
echo ""
log_info "Solana v${SOLANA_VERSION}  |  Anchor v${ANCHOR_VERSION}"
echo ""

# 1. Nix
install_nix

# 2. Re-enter inside nix develop (Rust stable + bpfel target, Node 22, Bun)
ensure_nix_env "$@"

# 3. Docker (for CI builds of .so artifacts)
install_docker

# 4. Solana CLI  (not in nixpkgs, must install from official source)
install_solana

# 5. Anchor CLI via AVM
install_anchor

# 6. JS dependencies
if command_exists bun; then
    log_info "Installing JS dependencies..."
    cd "$PROJECT_ROOT" && bun install 2>/dev/null || bun install --no-frozen-lockfile
    log_success "JS dependencies installed."
fi

# 7. Wallet keypair — required for anchor test and localnet deploys
if [ "$SKIP_KEYGEN" = false ]; then
    KEYPAIR="$HOME/.config/solana/id.json"
    if [ ! -f "$KEYPAIR" ]; then
        log_info "No keypair found at $KEYPAIR."
        if [ "${YES:-0}" -eq 1 ] || prompt "Generate a new local keypair?"; then
            solana-keygen new --no-bip39-passphrase --silent --outfile "$KEYPAIR"
            log_success "Keypair generated: $KEYPAIR"
            log_warning "This is a LOCAL dev keypair — never fund it with real SOL."
        fi
    else
        log_success "Wallet keypair: $KEYPAIR"
    fi
fi

# 8. Set cluster to localnet by default
if command_exists solana; then
    solana config set --url localhost >/dev/null 2>&1 || true
fi

# 9. Configure git hooks
if [ -d "$PROJECT_ROOT/.git-hooks" ]; then
    log_info "Configuring git hooks..."
    git -C "$PROJECT_ROOT" config core.hooksPath .git-hooks
    chmod +x "$PROJECT_ROOT"/.git-hooks/* 2>/dev/null || true
    log_success "Git hooks configured (.git-hooks/)."
else
    log_warning ".git-hooks/ not found — skipping hook setup."
fi

echo ""
echo "╔══════════════════════════════════════════════╗"
echo "║  ✓ ResQ Programs setup complete              ║"
echo "╚══════════════════════════════════════════════╝"
echo ""
echo "Next steps:"
echo "  nix develop                                  # Enter dev shell"
echo "  anchor build                                 # Compile programs"
echo "  anchor test                                  # Run tests (starts localnet)"
echo "  anchor deploy --provider.cluster devnet      # Deploy to devnet"
echo "  docker build -t resq-programs .              # Build artifacts via Docker"
