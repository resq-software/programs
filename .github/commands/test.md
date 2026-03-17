---
name: test
description: Run the repository validation workflow.
---

# /test

Run tests for the ResQ Solana programs.

## Steps

1. Run `bash ./scripts/test.sh`.
2. This builds the workspace, compiles integration targets, and runs library tests.
3. Report compiler failures and failing tests with the crate, test name, and error message.
4. Treat validator-backed execution as a separate harness issue unless the task explicitly asks for it.
