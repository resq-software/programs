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
    state::{airspace_account::AirspaceAccount, permit::Permit},
};

#[derive(Accounts)]
#[instruction(drone_pda: Pubkey, expires_at: i64)]
pub struct GrantPermit<'info> {
    /// Must be the airspace owner.
    #[account(mut)]
    pub owner: Signer<'info>,

    /// The AirspaceAccount the permit applies to.
    #[account(
        has_one = owner @ AirspaceError::Unauthorized,
    )]
    pub airspace: Account<'info, AirspaceAccount>,

    /// The new Permit PDA — one permit per (airspace, drone) pair.
    #[account(
        init,
        payer = owner,
        space = Permit::LEN,
        seeds = [b"permit", airspace.key().as_ref(), drone_pda.as_ref()],
        bump,
    )]
    pub permit: Account<'info, Permit>,

    pub system_program: Program<'info, System>,
}

/// Issue an airspace access permit to a drone PDA.
///
/// # Arguments
/// * `drone_pda`  – the drone's Program-Derived Address
/// * `expires_at` – Unix timestamp when the permit expires (0 = never)
pub fn handler(ctx: Context<GrantPermit>, drone_pda: Pubkey, expires_at: i64) -> Result<()> {
    let clock = Clock::get()?;
    require!(
        expires_at == 0 || expires_at > clock.unix_timestamp,
        AirspaceError::ExpiryInPast
    );

    let permit_pda = ctx.accounts.permit.key();
    let airspace_pda = ctx.accounts.airspace.key();

    let permit = &mut ctx.accounts.permit;
    permit.airspace = airspace_pda;
    permit.drone_pda = drone_pda;
    permit.granted_at = clock.unix_timestamp;
    permit.expires_at = expires_at;
    permit.bump = ctx.bumps.permit;

    emit!(PermitGranted {
        permit_pda,
        airspace_pda,
        drone_pda,
        expires_at,
    });

    Ok(())
}

/// Emitted when a new Permit is issued.
#[event]
pub struct PermitGranted {
    pub permit_pda: Pubkey,
    pub airspace_pda: Pubkey,
    pub drone_pda: Pubkey,
    pub expires_at: i64,
}