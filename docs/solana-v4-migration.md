<!--
Copyright 2026 ResQ Software
SPDX-License-Identifier: Apache-2.0
-->

# Solana v4 Consolidation — Investigation & Migration Plan

**Status:** investigation (no dependency changes made)
**Date:** 2026-06-02
**Scope:** the multi-version `solana-*` split in this workspace and whether a
full v4 consolidation is feasible.

> Security note: this is **independent** of the osv-scanner work. PR #33
> already removed the vendored lockfile that produced the false-positive CVEs,
> so the version split below has **no outstanding security impact** — it is a
> maintainability / upgrade-hygiene question only.

---

## 1. Current state (facts)

### Declared (root `Cargo.toml`)
| Crate | Pinned | Major |
|-------|--------|-------|
| `anchor-lang` / `anchor-client` | `1.0.0-rc.2` | pre-release |
| `solana-sdk` | `4.0.1` | v4 |
| `solana-pubkey` | `4.1.0` | **v4** |
| `solana-account` | `3.2.0` | v3 |
| `solana-instruction` | `3.2.0` | v3 |
| `solana-program-test` | `3.1.10` (vendored, `agave-unstable-api`) | v3 |
| `solana-account-info`, `-keypair`, `-program-entrypoint`, `-signer`, `-system-interface`, `-transaction` | `3.x` | v3 |

### Resolved (`Cargo.lock`) — the split
`solana-*` crates resolve across **all majors simultaneously**:

```
major v0:   3 crates      major v3: 143 crates   (the bulk)
major v1:   2 crates      major v4:  13 crates
major v2:   5 crates      major v5:   1 crate
                          major v6:   1 crate (solana-loader-v3-interface)
```

**This is expected, not corruption.** Since Solana's monorepo split, each
`solana-*` crate versions independently; a v4 `solana-sdk` legitimately
depends on v3 sub-crates. The spread is *wide* here because the stack is
mid-migration.

---

## 2. Is there a newer release?

Yes:

| Component | In repo | Latest stable | Notes |
|-----------|---------|---------------|-------|
| `solana-program-test` | 3.1.10 (vendored) | **4.0.0** | 3.x line tops out at 3.1.12; `4.1.0-beta.2` exists |
| Agave validator | — | **v4.0.1** | current stable line |
| `solana-sdk` | 4.0.1 | 4.0.1 | already current |
| `solana-pubkey` | 4.1.0 | 4.2.0 | minor behind |
| `solana-account` | 3.2.0 | 4.3.0 | major behind |
| `anchor-lang` | **1.0.0-rc.2** (2026-01-10) | **1.0.2** (2026-05-02) | stable line shipped: 1.0.0, 1.0.1, 1.0.2 |

---

## 3. Why `solana-program-test` is vendored

`vendor/solana-program-test/` is **plain `cargo vendor` output**, not a
functional fork:
- `Cargo.toml.orig` is present alongside the cargo-normalised `Cargo.toml`.
- The only delta vs upstream is an added Apache license header (`diff` of
  `.orig` vs `Cargo.toml` shows just the header + cargo normalisation).
- It is wired in as a `[patch.crates-io]` path dependency.

The reason for vendoring is the **`agave-unstable-api`** feature, which the
crate gates a large part of its surface behind and which is awkward to enable
on the published crate from a downstream workspace. Upgrading is therefore a
**re-vendor**, not a re-patch — mechanically simple.

---

## 4. The blocker: Anchor, not program-test

A naïve "bump `solana-program-test` to 4.0.0" does **not** collapse the split,
because the split is driven by **Anchor's** dependency pins, and there is a
**hard major-version conflict** on `solana-pubkey`:

| Shared crate | `solana-program-test` 4.0.0 | `anchor-lang` **1.0.2** (stable) | `anchor-lang` **1.0.0-rc.2** (current) |
|--------------|------------------------------|----------------------------------|----------------------------------------|
| `solana-pubkey` | `~4.1.0` (**v4**) | `^3.0.0` (**v3 only**) | `^4.0.0` (**v4**) |
| `solana-account-info` | `^3.1.0` | `^3.1.0` | `^3.0.0` |
| `solana-cpi` | `^3.1.0` | `^3.0.0` | `^3.0.0` |
| `solana-instruction` | `>=3.2,<3.4` | `^3.0.0` | `^3.0.0` |
| `solana-stake-interface` | `^2.0.2` | `^2.0.2` | `^2.0.0` |

### The surprising finding
**The repo is on anchor `1.0.0-rc.2` *on purpose*.** rc.2 allows
`solana-pubkey ^4.0.0`; the later **stable** `1.0.2` *narrowed* it back to
`^3.0.0`. So:

- Moving anchor `rc.2 → 1.0.2` **would regress** `solana-pubkey` to v3 and
  **conflict** with `solana-program-test 4.0.0` (which needs pubkey `~4.1.0`)
  and with the root's own `solana-pubkey = "4.1.0"` pin.
- This is almost certainly **why the upgrade stalled at rc.2** in commit
  `1200aa7e feat: upgrade anchor and solana sdk stack` (2026-03-17): rc.2 is
  the only Anchor release in the 1.0 line that admits pubkey v4.

**Conclusion:** a clean, single-major v4 consolidation is **blocked upstream
by Anchor**, not by anything in this repo. It cannot be done today without
either staying on an Anchor pre-release or forcing incompatible versions.

### Empirical verification (Option A was attempted and failed)

This was not left as theory — Option A (re-vendor `solana-program-test` →
`4.0.0`, keep anchor `rc.2`) was actually carried out on a scratch branch and
**fails to resolve**:

1. Hand-vendored `solana-program-test 4.0.0` (cargo-vendor tarball + license
   header), bumped the root pins to `solana-program-test = "4.0.0"` and
   `solana-account = "3.4.0"`.
2. A blanket `cargo update` *appeared* to succeed — but only because it
   silently drifted `anchor-lang` `rc.2 → 1.0.2` (the `^1.0.0-rc.2` range
   permits it). `cargo check` then failed to compile with anchor 1.0.2:
   `the trait AnchorDeserialize is not implemented for ...` across the program
   account/enum types (anchor 1.0.2 derive-macro incompatibility).
3. Hard-pinning anchor with `=1.0.0-rc.2` to prevent the drift exposes the true
   conflict at resolve time:

   ```
   failed to select a version for `solana-sysvar`.
     ... required by solana-program-test v4.0.0 (vendored)
     ... which conflicts with solana-sysvar = "~3.0.0"
         required by anchor-lang v1.0.0-rc.2
   ```

   `solana-program-test 4.0.0`'s transitive `solana-sysvar` requirement is
   incompatible with anchor rc.2's `solana-sysvar = "~3.0.0"`. No version
   satisfies both.

So Option A cannot ship: program-test 4.0.0 needs an `anchor` that does not
exist (one on the 4.x sysvar line *and* on `solana-pubkey` v4). The scratch
branch was fully reverted; this document is the result.

> Tooling note: a real `anchor test` / BPF build could not be run in the
> investigation environment (`anchor` and `cargo-build-sbf` are not installed),
> so the check above is `cargo`'s host-target resolution + `cargo check`. The
> resolve-time `solana-sysvar` conflict is decisive regardless of toolchain.

---

## 5. Options

### Option A — Re-vendor program-test to 4.0.0, keep anchor rc.2 *(ATTEMPTED — INFEASIBLE)*
- Hand-vendored `solana-program-test 4.0.0` (`agave-unstable-api`), bumped root
  pins to `program-test = "4.0.0"` + `solana-account = "3.4.0"`.
- **Result: resolution fails.** `solana-program-test 4.0.0`'s transitive
  `solana-sysvar` requirement conflicts with anchor `rc.2`'s
  `solana-sysvar = "~3.0.0"` (see §4 "Empirical verification"). A blanket
  `cargo update` only *looked* like it worked because it drifted anchor to
  1.0.2, which then fails `cargo check` (AnchorDeserialize macro breakage).
- **Do not pursue** until Anchor ships a release compatible with program-test
  4.x's sysvar line. Collapsed into Option C.

### Option B — Patch-bump within 3.x (lowest risk)
- Re-vendor `solana-program-test 3.1.10 → 3.1.12`.
- Pure patch movement, no major churn, picks up upstream fixes.
- Does **not** reduce the v3/v4 split, but is safe and quick.

### Option C — Full v4 + Anchor stable *(blocked — track upstream)*
- Requires an Anchor stable release that admits `solana-pubkey` v4 (1.0.0–1.0.2
  do not). Watch the Anchor changelog for a 1.x that re-widens the pubkey
  bound, then move anchor + program-test + sdk to v4 together.
- Until then this path forces incompatible versions and should not be attempted.

### Option D — Document only
- Land this doc so the intentional split + the Anchor blocker are recorded and
  not mistaken for a bug. No dependency changes.

---

## 6. Recommendation

1. **Now:** land this doc (Option D) so the rc.2 pin and the Anchor `pubkey` /
   `sysvar` conflict are understood. The pin is deliberate; without this note it
   reads like an oversight.
2. **Optional, low-risk:** Option B (3.1.10 → 3.1.12 re-vendor) for upstream
   patch fixes — the only dependency move available today.
3. **Ruled out:** Option A — proven infeasible at resolve time (see §4); folded
   into Option C.
4. **Track upstream:** Option C (true consolidation) is gated on Anchor
   shipping a stable release that admits `solana-pubkey` v4. Re-evaluate when
   that lands.

## 7. Verification commands

```bash
# version spread in the resolved tree
grep -A1 '^name = "solana-' Cargo.lock | grep version | sort | uniq -c

# confirm program-test is plain cargo-vendor output (not a fork)
diff vendor/solana-program-test/Cargo.toml.orig vendor/solana-program-test/Cargo.toml

# upstream pins for any candidate version
curl -s -H 'User-Agent: x' \
  https://crates.io/api/v1/crates/anchor-lang/1.0.2/dependencies
```
