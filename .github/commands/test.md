---
name: test
description: Run all Anchor tests against a local validator.
---

# /test

Run tests for the ResQ Solana programs.

## Steps

1. Run `anchor test`.
2. This starts a local validator, deploys programs, runs TypeScript tests, and tears down the validator.
3. Report failing tests with the instruction name, test description, and error message.
4. If `solana-test-validator` fails to start, check for a stale `.anchor/` lock file and remove it.
