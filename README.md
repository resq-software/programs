# ResQ Programs

[![CI](https://img.shields.io/github/actions/workflow/status/resq-software/programs/ci.yml?branch=main&label=ci&style=flat-square)](https://github.com/resq-software/programs/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg?style=flat-square)](./LICENSE)

ResQ Programs is the decentralized coordination layer for autonomous aerospace and delivery operations. Built using [Anchor 0.32](https://www.anchor-lang.com/), these Solana on-chain programs enforce geofencing, permit issuance, and mission lifecycle management through trust-minimized, atomic state transitions.

---

## Overview

ResQ Programs provide the trust-minimized substrate for physical automation. By offloading rule enforcement to the Solana blockchain, the ResQ ecosystem ensures that air traffic protocols and delivery missions are immutable, transparent, and verifiable.

### Key Components
* **`resq-airspace`**: Governs physical airspace access. It manages zone registration, access policies (Open, Permit, Deny, Auction), and cryptographic permit issuance.
* **`resq-delivery`**: Manages the mission-critical state of autonomous delivery vehicles, including immutable proof-of-delivery logging and coordinate validation.

---

## Features

* **Proof-of-Permit**: Cryptographically enforce that only authorized drones operate in restricted airspace.
* **Autonomous Lifecycle**: Atomic state transitions for delivery missions.
* **Policy-as-Code**: Airspace policies (altitude, geofencing, proximity) are enforced by on-chain logic.
* **Auditable History**: Every crossing and delivery is recorded on-chain for regulatory compliance.

---

## Architecture

The system utilizes Program-Derived Addresses (PDAs) for state management. Permissions are granted via PDA-based `Permit` accounts, which are checked by the `resq-airspace` program before processing crossing events.

```mermaid
c4Context
    title System Context Diagram
    Person(operator, "Authority/Operator")
    System_Boundary(solana, "Solana Network") {
        System(airspace, "resq-airspace", "Zone & Permit Logic")
        System(delivery, "resq-delivery", "Mission & Proof Logic")
        System_Ext(state, "Solana State Accounts", "PDAs & Account Data")
    }
    Rel(operator, airspace, "Initializes Property/Grants Permit")
    Rel(operator, delivery, "Records Delivery Mission")
    Rel(airspace, state, "Writes AirspaceAccount/Permit")
    Rel(delivery, state, "Writes DeliveryRecord")
    Rel(delivery, airspace, "Verifies Permit via CPI")
```

---

## Installation

### Prerequisites
* **Rust**: `stable` (via `rustup`)
* **Solana CLI**: `2.1.0`
* **Anchor CLI**: `0.30.1`

### Execution
```bash
# Clone the repository
git clone https://github.com/resq-software/programs.git
cd programs

# Bootstrap environment (installs dependencies/hooks)
./bootstrap.sh

# Build programs
anchor build
```

---

## Quick Start

Execute integration tests to verify the deployment state:

```bash
# Compile and run test suite against solana-test-validator
anchor test
```

---

## Usage

### Registering Airspace
To initialize a restricted zone, an owner must provide a unique 32-byte identifier and define altitude bounds.

```typescript
const propertyId = [...Buffer.from("zone-nyc-01").padEnd(32, '\0')];
const tx = await program.methods
  .initializeProperty(propertyId, 10, 150, [[...]], 1, { permit: {} }, 0, treasuryKey)
  .accounts({ owner: wallet.publicKey })
  .rpc();
```

### Recording a Delivery
Finalize a mission by logging the proof-of-delivery CID and GPS coordinates.

```typescript
await delivery.methods
  .recordDelivery(cidBytes, lat, lon, alt, Date.now())
  .accounts({ drone: pilot.publicKey })
  .rpc();
```

---

## Configuration

Settings are managed in `Anchor.toml`. 

* **`[programs.localnet]`**: Defines custom Program IDs. Ensure these match the `declare_id!` macro in your Rust code.
* **`[provider]`**: Configures the default wallet and cluster.
* **Environment Overrides**:
    * `SOLANA_VERSION`: Ensure the CLI version matches the `Cargo.toml` dependencies.
    * `ANCHOR_VERSION`: Uses `AVM` to manage cross-version compatibility for the `0.30.x` lineage.

---

## API Reference

### `resq-airspace`
* **`initialize_property`**: Sets up the geometry and access policy for a region.
* **`grant_permit`**: Creates a `Permit` PDA for a specific drone key.
* **`record_crossing`**: Validates a transit attempt. If `AccessPolicy::Permit` is active, it performs a cross-program check on the drone's `Permit` account.

### `resq-delivery`
* **`record_delivery`**: Creates an immutable `DeliveryRecord`.
* **Constraint Logic**: Validates coordinate ranges (lat: -90° to 90°, lon: -180° to 180°) and ensures `delivered_at` is a positive timestamp.

---

## Development

### Error Handling
Errors are defined via `#[error_code]` enums in each program (e.g., `AirspaceError`, `DeliveryError`). 
- **Constraint Violations**: Anchor macros (`has_one`, `seeds`, etc.) return standard `Account` errors if validation fails.
- **Custom Logic**: Use the `require!` and `require_keys_eq!` macros to return descriptive errors to the client.

### Cross-Program Interaction
Interaction between programs is strictly unidirectional or via CPI. The `resq-delivery` program holds a reference to the `airspace` account address, enabling auditing tools to reconstruct the mission context without coupling the state machine itself.

---

## Contributing

We strictly follow [Conventional Commits](https://www.conventionalcommits.org/).

1. **Feature branch**: `feat/` or `fix/`.
2. **Audit**: Run `cargo audit` and the custom `.git-hooks/` pre-commit checks.
3. **Pull Request**: Must pass CI coverage and validation checks.

---

## License

Copyright 2026 ResQ. Distributed under the Apache License, Version 2.0. See [LICENSE](./LICENSE) for details.