---
name: build
description: Build all Anchor programs and regenerate IDL + TypeScript types.
---

# /build

Build the ResQ Solana programs.

## Steps

1. Run `anchor build`.
2. Confirm `target/idl/resq_airspace.json`, `target/idl/resq_delivery.json` were regenerated.
3. Confirm `target/types/resq_airspace.ts`, `target/types/resq_delivery.ts` were regenerated.
4. Report any Rust compilation errors or warnings.
5. If the IDL changed, note which instructions or accounts changed so tests can be updated.
