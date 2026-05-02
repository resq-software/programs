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

/// Airspace access permit granted to a specific drone by a property owner.
///
/// PDA seeds: `["permit", airspace_pda, drone_pda]`
///
/// Space breakdown:
/// ```text
///  8   discriminator
/// 32   airspace
/// 32   drone_pda
///  8   granted_at
///  8   expires_at
///  1   bump
/// ─────────────────
/// 89 bytes total
/// ```
#[account]
pub struct Permit {
    /// The AirspaceAccount this permit is valid for.
    pub airspace: Pubkey,
    /// The drone PDA that holds this permit.
    pub drone_pda: Pubkey,
    /// Unix timestamp (seconds) when the permit was issued.
    pub granted_at: i64,
    /// Unix timestamp (seconds) when the permit expires (0 = no expiry).
    pub expires_at: i64,
    /// PDA canonical bump.
    pub bump: u8,
}

impl Permit {
    /// Account size in bytes (discriminator + fields).
    pub const LEN: usize = 8 + 32 + 32 + 8 + 8 + 1;

    /// Returns `true` if the permit has not yet expired at `now` (Unix seconds).
    pub fn is_active(&self, now: i64) -> bool {
        self.expires_at == 0 || self.expires_at > now
    }
}