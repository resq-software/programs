# Solana Stack Alignment Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the carrier Dependabot PR into one coherent Anchor `0.32.1` / Solana `2.3.x` stack-alignment branch that passes the full workspace test suite.

**Architecture:** Keep the work isolated to the carrier PR branch, normalize manifest versions first, then refresh the lockfile and fix any code or test fallout exposed by the coherent dependency graph. Preserve the local `solana-invoke` override and current host validation harness unless the aligned stack makes a smaller fix possible.

**Tech Stack:** Rust workspace, Cargo, Anchor `0.32.1`, Solana `2.3.x`, `solana-program-test`, GitHub Actions

---

## Chunk 1: Manifest Alignment

### Task 1: Normalize workspace dependency versions

**Files:**
- Modify: `Cargo.toml`
- Test: `cargo test --workspace --tests`

- [ ] **Step 1: Confirm the current carrier-branch manifest state**

Run: `sed -n '20,60p' Cargo.toml`
Expected: `solana-sdk = "4.0"` is present and the workspace still uses `anchor-lang = "0.32.1"`.

- [ ] **Step 2: Write the minimal manifest change**

Update `Cargo.toml` so:
- `anchor-client` is aligned with `anchor-lang` on `0.32.1`
- `solana-sdk` is returned to the Solana `2.3.x` line
- `solana-program-test` stays or is normalized on the Solana `2.3.x` line

- [ ] **Step 3: Verify the manifest diff is coherent**

Run: `git diff -- Cargo.toml`
Expected: One coherent stack move, not another split major-version jump.

## Chunk 2: Lockfile Refresh

### Task 2: Refresh and inspect the resolved graph

**Files:**
- Modify: `Cargo.lock`
- Test: `cargo test --workspace --tests`

- [ ] **Step 1: Regenerate the lockfile through the real validation path**

Run: `cargo test --workspace --tests`
Expected: Cargo updates the lockfile and either passes or fails with a repo-local error instead of the previous `solana-keypair 3.1.2` / `DecodeError` mismatch.

- [ ] **Step 2: Inspect the resolved Solana and Anchor graph**

Run: `cargo tree -p resq-airspace --depth 1`
Expected: dev dependencies resolve to the Solana `2.3.x` line, and the split `solana-sdk 4.x` path is gone.

- [ ] **Step 3: Verify the old incompatibility is absent**

Run: `cargo test --workspace --tests 2>&1 | rg -n 'DecodeError|solana-keypair 3\\.1\\.2|solana-signature 3\\.3\\.0'`
Expected: No matches.

## Chunk 3: Code And Test Fallout

### Task 3: Fix any repo-local fallout exposed by the aligned graph

**Files:**
- Modify: `resq-airspace/src/lib.rs`
- Modify: `resq-airspace/src/instructions/mod.rs`
- Modify: `resq-airspace/tests/integration.rs`
- Modify: `resq-airspace/tests/host_init_regression.rs`
- Modify: `resq-delivery/src/lib.rs`
- Modify: `resq-delivery/tests/integration.rs`
- Modify: `vendor/solana-invoke/src/lib.rs`
- Modify: `vendor/solana-invoke/src/stable_instruction_borrowed.rs`
- Modify: other touched files only if the aligned graph requires it
- Test: `cargo test --workspace --tests`

- [ ] **Step 1: Reproduce any remaining repo-local failure in isolation**

Run the smallest failing command from the full workspace test output.
Expected: A concrete repo-local failure, if any remain.

- [ ] **Step 2: Write the minimal fix**

Only fix code or tests needed for the aligned dependency graph to pass. Do not introduce unrelated refactors.

- [ ] **Step 3: Re-run the full workspace tests**

Run: `cargo test --workspace --tests`
Expected: Pass.

- [ ] **Step 4: Re-run the warning scan**

Run: `cargo test --workspace --tests 2>&1 | rg -n 'warning:'`
Expected: No matches.

## Chunk 4: Branch Readiness

### Task 4: Confirm branch state and supersession path

**Files:**
- Review: `Cargo.toml`
- Review: `Cargo.lock`

- [ ] **Step 1: Inspect the final branch diff**

Run: `git status --short --branch`
Expected: Only intended carrier-branch changes are present.

- [ ] **Step 2: Summarize how the branch supersedes PR #6**

Capture:
- final Anchor versions
- final Solana versions
- confirmation that both `solana-sdk` and `solana-program-test` are handled on this branch

- [ ] **Step 3: Commit the carrier-branch work**

Run:
```bash
git add Cargo.toml Cargo.lock resq-airspace resq-delivery vendor docs/superpowers
git commit -m "fix: align anchor and solana stack"
```
Expected: Clean commit on `dependabot/cargo/solana-sdk-4.0.1`.
