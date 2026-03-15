---
name: audit
description: Run security audit on Anchor programs using the feynman-auditor and nemesis-auditor skills.
---

# /audit

Security audit of the ResQ Solana programs.

## Steps

1. Invoke the `feynman-auditor` skill for deep logic analysis of all instruction handlers.
2. Invoke the `state-inconsistency-auditor` skill to map all coupled account state pairs.
3. Invoke the `nemesis-auditor` skill for the full iterative combined audit.
4. For each finding: report instruction name, vulnerability type, attack scenario, and remediation.
5. Run `cargo audit` on the Rust workspace.
6. Run Clippy: `cargo clippy --all-targets -- -D warnings`.

## Priority

Treat any finding related to the following as CRITICAL:
- Missing signer validation
- Unchecked arithmetic on lamports or token amounts
- PDA bump seed not validated
- Missing ownership checks on accounts
