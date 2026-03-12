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

/// On-chain state for a single drone delivery.
///
/// PDA seeds: `["delivery", drone_pda, delivered_at_le_bytes]`
///
/// Space breakdown:
/// ```text
///  8  discriminator
/// 32  drone_pda
/// 32  airspace_pda
/// 64  ipfs_cid       (base58, null-padded)
///  8  lat            (× 1e7 fixed-point)
///  8  lon            (× 1e7 fixed-point)
///  4  alt_m
///  8  delivered_at   (Unix seconds)
///  1  bump
/// ─────────────────
/// 165 bytes total
/// ```
#[account]
pub struct DeliveryRecord {
    /// Base58 drone Program-Derived Address that authorised this delivery.
    pub drone_pda: Pubkey,
    /// Base58 AirspaceAccount PDA of the target property.
    pub airspace_pda: Pubkey,
    /// IPFS CID of the delivery photo / evidence (base58, null-padded to 64 bytes).
    pub ipfs_cid: [u8; 64],
    /// Latitude × 1e7 (fixed-point, signed).
    pub lat: i64,
    /// Longitude × 1e7 (fixed-point, signed).
    pub lon: i64,
    /// Altitude in metres above mean sea level.
    pub alt_m: u32,
    /// Unix timestamp (seconds) when the delivery was confirmed.
    pub delivered_at: i64,
    /// PDA canonical bump for this account.
    pub bump: u8,
}

impl DeliveryRecord {
    /// Account size in bytes (discriminator + fields).
    pub const LEN: usize = 8 + 32 + 32 + 64 + 8 + 8 + 4 + 8 + 1;
}