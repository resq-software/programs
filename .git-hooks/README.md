# Git Hooks — ResQ Programs

This directory contains the project's git hooks. They enforce code quality, security, and workflow conventions for the ResQ Solana/Anchor programs (Rust/Bun).

## Installation

```bash
# Configure git to use these hooks
git config core.hooksPath .git-hooks

# Or run the setup script (recommended)
./scripts/setup.sh
```

The setup script sets `core.hooksPath` to `.git-hooks` and makes all hooks executable.

## Active Hooks

| Hook | Purpose |
|------|---------|
| `pre-commit` | Large file guard (1 MB limit), secrets scan (gitleaks / grep fallback), `cargo fmt --all -- --check` on staged `.rs` files |
| `commit-msg` | Conventional Commits format validation; blocks `fixup!`/`squash!`/WIP on `main` |
| `prepare-commit-msg` | Prepends ticket reference (e.g., `[PROJ-123]`) extracted from branch name |
| `pre-push` | Force-push guard on `main`, branch naming convention, `cargo check --workspace` on changed Rust/Anchor files |
| `post-checkout` | Notifies on `Cargo.lock` changes; auto `bun install` when `bun.lockb` changes between branches |
| `post-merge` | Notifies on `Cargo.lock` changes; auto `bun install` when `bun.lockb` changes after a merge |

## Bypassing Hooks

Use `--no-verify` to skip `pre-commit` and `commit-msg` hooks:

```bash
git commit --no-verify -m "wip: quick save"
git push --no-verify
```

> **Note:** `post-checkout`, `post-merge`, and `prepare-commit-msg` cannot be bypassed with `--no-verify`.
> Set `GIT_HOOKS_SKIP=1` to skip all custom hook logic.

## Environment Variables

| Variable | Effect | Scope |
|----------|--------|-------|
| `GIT_HOOKS_SKIP=1` | Skip all custom hook logic | `pre-commit`, `commit-msg`, `pre-push`, `post-checkout`, `post-merge` |
| `SKIP_BUN_INSTALL=1` | Skip the automatic `bun install` step | `post-checkout`, `post-merge` |

## Adding a New Hook

1. Create the hook file in `.git-hooks/` (no extension).
2. Start with `#!/usr/bin/env bash` and add the Apache-2.0 license header.
3. Make it executable: `chmod +x .git-hooks/<hook-name>`
4. Test with: `bash -n .git-hooks/<hook-name>`
5. Update this README with the new hook's purpose.
