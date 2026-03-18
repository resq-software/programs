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

use crate::{
    error::AirspaceError,
    state::airspace_account::{AccessPolicy, AirspaceAccount},
};

#[derive(Accounts)]
#[instruction(property_id: [u8; 32])]
pub struct InitializeProperty<'info> {
    /// The property owner who will control this airspace.
    #[account(mut)]
    pub owner: Signer<'info>,

    /// The new AirspaceAccount PDA, funded by the owner.
    #[account(
        init,
        payer = owner,
        space = AirspaceAccount::LEN,
        seeds = [b"airspace", property_id.as_ref()],
        bump,
    )]
    pub airspace: Account<'info, AirspaceAccount>,

    pub system_program: Program<'info, System>,
}

/// Create and initialise a new `AirspaceAccount` for a property.
///
/// # Arguments
/// * `property_id`   – 32-byte null-padded external property identifier
/// * `min_alt_m`     – lower altitude bound (metres AGL)
/// * `max_alt_m`     – upper altitude bound (metres AGL, must be > min)
/// * `poly`          – up to 8 polygon vertices as `[lat_1e7, lon_1e7]` pairs
/// * `vertex_count`  – number of valid vertices in `poly` (1–8)
/// * `policy`        – `AccessPolicy` enum value
/// * `fee_lamports`  – per-crossing fee (0 = free)
/// * `treasury`      – SOL account that receives crossing fees
pub fn handler(
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
    require!(property_id != [0u8; 32], AirspaceError::EmptyPropertyId);
    require!(min_alt_m < max_alt_m, AirspaceError::InvalidAltitudeBounds);
    require!(
        vertex_count >= 1 && vertex_count <= 8,
        AirspaceError::InvalidVertexCount
    );
    require!(treasury != Pubkey::default(), AirspaceError::InvalidTreasury);

    let airspace_pda = ctx.accounts.airspace.key();
    let owner_key = ctx.accounts.owner.key();

    let airspace = &mut ctx.accounts.airspace;
    airspace.owner = owner_key;
    airspace.property_id = property_id;
    airspace.min_alt_m = min_alt_m;
    airspace.max_alt_m = max_alt_m;
    airspace.poly = poly;
    airspace.vertex_count = vertex_count;
    airspace.policy = policy;
    airspace.fee_lamports = fee_lamports;
    airspace.treasury = treasury;
    airspace.bump = ctx.bumps.airspace;

    emit!(PropertyInitialized {
        airspace_pda,
        owner: owner_key,
        property_id,
    });

    Ok(())
}

/// Emitted when a new AirspaceAccount is created.
#[event]
pub struct PropertyInitialized {
    pub airspace_pda: Pubkey,
    pub owner: Pubkey,
    pub property_id: [u8; 32],
}