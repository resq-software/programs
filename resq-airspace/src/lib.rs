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

use instructions::*;
use state::airspace_account::AccessPolicy;

declare_id!("A1rSpAcE111111111111111111111111111111111111");

#[program]
pub mod resq_airspace {
    use super::*;

    /// Create and initialise a new `AirspaceAccount` for a property.
    pub fn initialize_property(
        ctx: Context<InitializeProperty>,
        property_id: [u8; 32],
        min_alt_m: u32,
        max_alt_m: u32,
        poly: [[i64; 2]; 8],
        vertex_count: u8,
        policy: AccessPolicy,
        fee_lamports: u64,
        treasury: Pubkey,
    ) -> Result<()> {
        instructions::initialize_property::handler(
            ctx,
            property_id,
            min_alt_m,
            max_alt_m,
            poly,
            vertex_count,
            policy,
            fee_lamports,
            treasury,
        )
    }

    /// Update the access policy and/or per-crossing fee.  Owner-only.
    pub fn update_policy(
        ctx: Context<UpdatePolicy>,
        policy: AccessPolicy,
        fee_lamports: u64,
    ) -> Result<()> {
        instructions::update_policy::handler(ctx, policy, fee_lamports)
    }

    /// Issue an airspace access permit to a drone PDA.  Owner-only.
    pub fn grant_permit(
        ctx: Context<GrantPermit>,
        drone_pda: Pubkey,
        expires_at: i64,
    ) -> Result<()> {
        instructions::grant_permit::handler(ctx, drone_pda, expires_at)
    }

    /// Record a drone airspace crossing.  Drone must sign; crossing fee is collected.
    pub fn record_crossing(
        ctx: Context<RecordCrossing>,
        lat: i64,
        lon: i64,
        alt_m: u32,
        crossed_at: i64,
    ) -> Result<()> {
        instructions::record_crossing::handler(ctx, lat, lon, alt_m, crossed_at)
    }
}
