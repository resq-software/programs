---
name: testing
description: Testing rules for the ResQ Solana Anchor programs.
---

# Testing Rules

## Coverage

- Every instruction handler must have:
  - At least one happy-path test
  - At least one test for each `#[error_code]` variant it can emit
  - Tests for edge cases (zero amounts, maximum values, self-referential accounts where applicable)

## Test Framework

- TypeScript tests using `@coral-xyz/anchor` and `mocha`/`chai`.
- Test files in `tests/` named after the program (e.g., `resq-airspace.ts`).
- Use `anchor.setProvider(anchor.AnchorProvider.env())` — never hardcode cluster URLs.
- Create fresh keypairs per test to avoid state bleed between tests.

## Error Assertion

```typescript
// Correct: assert the specific Anchor error code
await assert.rejects(
  program.methods.registerZone(...).rpc(),
  (err: AnchorError) => err.error.errorCode.number === 6001 // ZONE_ALREADY_EXISTS
);
```

## Local Validator

- `anchor test` must succeed with a fresh local validator — no dependency on devnet state.
- Tests must be idempotent — running them twice in a row should not fail due to state left from the first run.
