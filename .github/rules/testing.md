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

- Rust integration tests use `solana-program-test` with `#[tokio::test]`.
- Integration tests live under each crate's `tests/` directory (for example, `resq-airspace/tests/integration.rs`).
- Keep reusable harness helpers small and deterministic.
- Create fresh keypairs per test to avoid state bleed between cases.

## Error Assertion

```rust
let err = banks_client.process_transaction(tx).await.unwrap_err();
assert!(
    err.unwrap().to_string().contains("EmptyPropertyId")
        || format!("{err:?}").contains("Custom(6001)")
);
```

## Automated Gate

- `bash ./scripts/test.sh` is the default repository validation command.
- It must build the workspace, compile integration targets, and keep library tests green on every PR.

## Runtime Harnesses

- Validator-backed or SBF execution belongs to an explicitly maintained harness, not the default CI path.
- If a change depends on runtime behavior beyond the default gate, document the extra command and run it explicitly.
