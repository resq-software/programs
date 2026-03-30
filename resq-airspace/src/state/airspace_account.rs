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

/// Access policy for an airspace envelope.
///
/// Variants are serialised by their implicit discriminant (Open=0, Permit=1,
/// Deny=2, Auction=3). The explicit `= N` values were removed to satisfy
/// borsh 1.x which requires `#[borsh(use_discriminant)]` for enums with
/// explicit discriminants — an annotation that Anchor 1.x derive macros do
/// not yet forward correctly. Wire format is unchanged.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AccessPolicy {
    /// Any drone may transit without a permit or fee.  (discriminant = 0)
    Open,
    /// A drone must hold a valid `Permit` account to transit.  (discriminant = 1)
    Permit,
    /// No drone may transit under any circumstances.  (discriminant = 2)
    Deny,
    /// Crossing fee is determined by an on-chain auction (future feature).  (discriminant = 3)
    Auction,
}

impl Default for AccessPolicy {
    fn default() -> Self {
        AccessPolicy::Open
    }
}

/// Per-property 3D airspace envelope registered on-chain.
///
/// PDA seeds: `["airspace", property_id_bytes]`
///
/// Space breakdown:
/// ```text
///  8   discriminator
/// 32   owner
/// 32   property_id    (UTF-8 bytes, null-padded)
///  4   min_alt_m
///  4   max_alt_m
/// 128  poly           (8 vertices × 2 × 8 bytes)
///  1   vertex_count
///  1   policy
///  8   fee_lamports
/// 32   treasury
///  1   bump
/// ─────────────────
/// 251 bytes total
/// ```
#[account]
pub struct AirspaceAccount {
    /// Property owner — only this pubkey can modify the airspace or grant permits.
    pub owner: Pubkey,
    /// External property identifier (UTF-8, null-padded to 32 bytes).
    pub property_id: [u8; 32],
    /// Lower altitude bound in metres AGL (above ground level).
    pub min_alt_m: u32,
    /// Upper altitude bound in metres AGL.
    pub max_alt_m: u32,
    /// Polygon vertices: up to 8 pairs of (lat × 1e7, lon × 1e7).
    pub poly: [[i64; 2]; 8],
    /// Number of valid vertices in `poly` (1–8).
    pub vertex_count: u8,
    /// Access control policy for this airspace.
    pub policy: AccessPolicy,
    /// Per-crossing fee in lamports (0 = free).
    pub fee_lamports: u64,
    /// SOL treasury account that receives crossing fees.
    pub treasury: Pubkey,
    /// PDA canonical bump.
    pub bump: u8,
}

impl AirspaceAccount {
    /// Account size in bytes (discriminator + fields).
    pub const LEN: usize = 8 + 32 + 32 + 4 + 4 + 128 + 1 + 1 + 8 + 32 + 1;
}