#![allow(unexpected_cfgs)]

/*
 * Copyright 2026 ResQ
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod state;

pub use instructions::*;

declare_id!("DeL1v3ry111111111111111111111111111111111111");

/// Program ID of the resq-airspace program.  Used to verify that the `airspace`
/// account passed to `record_delivery` is genuinely owned by that program.
pub const AIRSPACE_PROGRAM_ID: Pubkey = pubkey!("A1rSpAcE111111111111111111111111111111111111");

#[program]
pub mod resq_delivery {
    use super::*;

    /// Record an immutable proof-of-delivery on the Solana blockchain.
    ///
    /// Creates a `DeliveryRecord` PDA seeded by the drone pubkey and
    /// delivery timestamp, storing the IPFS evidence CID and GPS coordinates.
    pub fn record_delivery(
        ctx: Context<RecordDelivery>,
        ipfs_cid: [u8; 64],
        lat: i64,
        lon: i64,
        alt_m: u32,
        delivered_at: i64,
    ) -> Result<()> {
        instructions::record_delivery::handler(ctx, ipfs_cid, lat, lon, alt_m, delivered_at)
    }
}
