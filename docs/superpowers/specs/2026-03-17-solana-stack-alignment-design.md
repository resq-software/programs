# Solana Stack Alignment Design

**Date:** 2026-03-17

**Problem**

The carrier Dependabot PR upgrades `solana-sdk` to `4.0.1` in isolation. That pulls a split Solana 3.x/4.x crate graph which fails to compile under the current Anchor `0.32.1` stack, most visibly in `solana-keypair 3.1.2` with the `DecodeError: std::error::Error` trait-bound failure.

**Goal**

Repurpose PR `#5` into one coherent, reviewable Solana and Anchor alignment PR that stays on the newest stable line officially compatible with Anchor `0.32.1`, while superseding PR `#6`.

**Design**

- Keep the carrier branch as `dependabot/cargo/solana-sdk-4.0.1`.
- Normalize workspace dependencies onto the Anchor `0.32.1` / Solana `2.3.x` line rather than forcing Solana `4.x`.
- Fold the `solana-program-test` side into the same branch so the PR reflects one coherent stack move.
- Preserve the host-side runtime harness fix already proven on `main`, including the local `solana-invoke` override that restores off-chain CPI behavior in `ProgramTest`.

**Compatibility Rule**

- `anchor-lang` and `anchor-client` should be aligned to the same Anchor line.
- `solana-sdk` and `solana-program-test` should stay on the Solana `2.3.x` line for this repo.
- Any documentation or tooling metadata that claims a different default validation path or incompatible toolchain should be updated only if touched by the dependency work.

**Validation**

- `cargo test --workspace --tests`
- `cargo test --workspace --tests 2>&1 | rg -n 'warning:'`

**Outcome**

If the carrier branch goes green with the coherent stack, PR `#6` should be treated as superseded.
