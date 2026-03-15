---
name: deploy
description: Deploy programs to devnet. Never run against mainnet.
---

# /deploy

Deploy ResQ programs to Solana devnet.

## Usage

```
/deploy [program]
```

## Steps

1. Confirm `Anchor.toml` cluster is set to `Devnet` (or `devnet`).
2. Run `anchor deploy --provider.cluster devnet` (or `anchor deploy -p <program> --provider.cluster devnet`).
3. Report deployed program IDs.
4. **NEVER deploy to mainnet** — mainnet upgrades require multisig approval through Squads v4.
5. After deployment, run a smoke test with `anchor idl fetch <program-id> --provider.cluster devnet` to confirm the IDL is accessible.
