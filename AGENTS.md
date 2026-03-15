# ResQ Programs — Agent Guide

## Mission
Solana on-chain programs for the ResQ platform. Implements delivery coordination (`resq-delivery`) and airspace management (`resq-airspace`) as Anchor programs, with a shared Rust library and JavaScript test/client layer.

## Workspace Layout
- `resq-delivery/` — Delivery coordination program: job lifecycle, drone assignment, escrow.
- `resq-airspace/` — Airspace management program: zone registry, flight authorisation, conflict detection.
- `Cargo.toml` — Workspace root; shared dependencies and patch overrides.
- `Anchor.toml` — Program IDs, cluster config, and test script.
- `package.json` — JS test runner and Anchor client dependencies (Bun).

## Commands
```bash
anchor build                               # Compile all programs (.so artifacts)
anchor test                               # Run tests against localnet
anchor deploy --provider.cluster devnet  # Deploy to devnet
cargo fmt --all                           # Format Rust
cargo clippy --workspace -- -D warnings  # Lint
./agent-sync.sh --check                  # Verify AGENTS.md and CLAUDE.md are in sync
```

## Architecture
- **Framework**: Anchor 0.30; account validation via `#[account]` constraints.
- **Programs** are `no_std`-compatible BPF targets compiled to `target/deploy/*.so`.
- **Shared logic** (math, validation, seeds) lives in a `resq` lib crate within each program workspace.
- **Tests** use `@coral-xyz/anchor` TypeScript client against a local validator spun up by `anchor test`.
- **PDAs** follow the pattern `[b"domain", entity_key.as_ref()]` — seeds documented in `Anchor.toml`.

## Standards
- All instructions must validate signer authority and account ownership explicitly.
- Use `require!` / `require_eq!` macros for constraint checks — never panic.
- Arithmetic via `checked_*` methods or `anchor_lang::prelude::*` safe math.
- No `unsafe` blocks. No `unwrap()` in instruction handlers.
- All source files carry the Apache-2.0 license header.
- Keep `AGENTS.md` and `CLAUDE.md` in sync using `./agent-sync.sh`.

## Security Rules
- Re-validate all accounts even when Anchor generates checks — defence in depth.
- PDA bump seeds must be stored in account state and verified on every instruction.
- Never trust client-provided amounts for escrow — recompute from on-chain state.
- Run `cargo audit` before every deploy (`./agent-sync.sh` covers this via pre-commit).

## Repository Rules
- Do not commit `target/` or `node_modules/`.
- Program IDs in `Anchor.toml` and `declare_id!()` must match — verify before deploy.
- Devnet deploys require a funded keypair at `~/.config/solana/id.json`.

## References
- [Root README](README.md)
- [Anchor.toml](Anchor.toml)
- [Anchor Docs](https://anchor-lang.com)
