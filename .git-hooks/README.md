<!--
  Copyright 2026 ResQ Systems, Inc.

  Licensed under the Apache License, Version 2.0 (the "License");
  you may not use this file except in compliance with the License.
  You may obtain a copy of the License at

      http://www.apache.org/licenses/LICENSE-2.0

  Unless required by applicable law or agreed to in writing, software
  distributed under the License is distributed on an "AS IS" BASIS,
  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
  See the License for the specific language governing permissions and
  limitations under the License.
-->

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

These are the canonical ResQ hooks, owned by
[`resq-software/crates`](https://github.com/resq-software/crates/tree/master/crates/resq-cli/templates/git-hooks)
and installed by `resq hooks update`. They are canonical shims: each keeps the
validation and reporting specific to its own hook, hands the heavier checks to
the `resq` binary where there are any, and then runs an executable repo-owned
`local-*` override. Editing them here only produces drift.

| Hook | Purpose |
|------|---------|
| `pre-commit` | Delegates to `resq pre-commit` — copyright headers, large-file guard, secret scan, dependency audit, per-language formatting |
| `commit-msg` | Conventional Commits format validation; blocks `fixup!`/`squash!`/WIP on `main` |
| `prepare-commit-msg` | Prepends ticket reference (e.g., `[PROJ-123]`) extracted from branch name |
| `pre-push` | Force-push guard and branch-naming rule, both applied to the ref being **pushed to**; then runs `local-pre-push` |
| `post-checkout` | **Reports** changed `Cargo.lock` / `bun.lock` / `uv.lock` / `flake.lock` and the command to run; then runs `local-post-checkout` |
| `post-merge` | **Reports** the same lockfile changes after a merge; then runs `local-post-merge` |

| Local hook | Purpose |
|------|---------|
| `local-pre-push` | `cargo check --workspace` when Rust/Anchor files changed. Skip with `SKIP_CARGO_CHECK=1` |

The lockfile hooks report rather than installing for you. A hook that mutates
the working tree during a checkout is a surprise, and the dependency state you
want after switching branches is not always the one the lockfile names.

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
