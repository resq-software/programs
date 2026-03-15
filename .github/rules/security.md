---
name: security
description: On-chain security rules for the ResQ Solana programs. Critical — violations block merges.
---

# On-Chain Security Rules

These rules are non-negotiable. Any violation is a CRITICAL finding and blocks PR merge.

## Account Validation

- Every account that is written to must have its ownership validated. Use Anchor's `Account<T>` or explicit `constraint` checks.
- Signers must be validated for all mutating instructions. Never use `UncheckedAccount` for signers.
- All PDAs must store their canonical bump seed and validate it on subsequent use (prevents bump seed substitution).

## Arithmetic

- Never use raw `+`, `-`, `*` on lamport amounts or token balances. Use `checked_add`, `checked_sub`, `checked_mul` and propagate `Result`.
- Division truncates in integer arithmetic — ensure rounding direction is intentional and documented.

## Cross-Program Invocations (CPI)

- Always validate the program ID of the target program before CPI (e.g., `anchor_spl::token::ID`).
- Do not pass writable accounts to programs that don't need write access.
- Validate all accounts returned by CPI.

## Re-entrancy

- Solana programs do not have traditional re-entrancy (no callbacks mid-instruction), but flash loan vectors via atomic multi-instruction transactions must be considered.
- If an instruction can be combined with another in a single transaction to produce a harmful sequence, document and guard against it.

## Compute Units

- No instruction should exceed 400,000 CU. Add a CU benchmark test for any instruction approaching this limit.
- `msg!()` macro calls are expensive — remove all debug logging before mainnet deployment.

## Upgrade Authority

- Upgrade authority for mainnet programs is held by a Squads v4 multisig. Never transfer upgrade authority to a single keypair.
- Do not include upgrade authority in test fixtures.
