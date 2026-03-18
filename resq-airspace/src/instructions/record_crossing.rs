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
use anchor_lang::system_program;

use crate::{
    error::AirspaceError,
    state::{
        airspace_account::{AccessPolicy, AirspaceAccount},
        permit::Permit,
    },
};

#[derive(Accounts)]
pub struct RecordCrossing<'info> {
    /// The drone authority — must sign the transaction and pays any crossing fee.
    #[account(mut)]
    pub drone: Signer<'info>,

    /// The AirspaceAccount being traversed.
    pub airspace: Account<'info, AirspaceAccount>,

    /// The drone's Permit for this airspace.
    /// Required when `airspace.policy == Permit`.
    #[account(
        seeds = [b"permit", airspace.key().as_ref(), drone.key().as_ref()],
        bump = permit.bump,
    )]
    pub permit: Account<'info, Permit>,

    /// The treasury account that receives the crossing fee (if any).
    /// CHECK: must match `airspace.treasury`; validated below.
    #[account(mut)]
    pub treasury: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

/// Record a drone airspace crossing event.
///
/// If the airspace has a non-zero `fee_lamports` and the drone holds a valid
/// permit, the fee is transferred from the drone's account to the treasury.
///
/// # Arguments
/// * `lat`        – latitude × 1e7
/// * `lon`        – longitude × 1e7
/// * `alt_m`      – altitude in metres
/// * `crossed_at` – Unix timestamp (seconds) of the crossing
pub fn handler(
    ctx: Context<RecordCrossing>,
    lat: i64,
    lon: i64,
    alt_m: u32,
    crossed_at: i64,
) -> Result<()> {
    let airspace = &ctx.accounts.airspace;
    let permit = &ctx.accounts.permit;
    let clock = Clock::get()?;

    // Policy check
    match airspace.policy {
        AccessPolicy::Deny => return err!(AirspaceError::NoValidPermit),
        AccessPolicy::Permit | AccessPolicy::Auction => {
            require!(permit.airspace == airspace.key(), AirspaceError::NoValidPermit);
            require!(permit.is_active(clock.unix_timestamp), AirspaceError::PermitExpired);
        }
        AccessPolicy::Open => {} // always allowed
    }

    // Treasury must match the registered account
    require_keys_eq!(
        ctx.accounts.treasury.key(),
        airspace.treasury,
        AirspaceError::FeeTransferFailed
    );

    // Collect per-crossing fee if applicable
    if airspace.fee_lamports > 0 {
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.key(),
                system_program::Transfer {
                    from: ctx.accounts.drone.to_account_info(),
                    to: ctx.accounts.treasury.to_account_info(),
                },
            ),
            airspace.fee_lamports,
        )?;
    }

    emit!(CrossingRecorded {
        airspace_pda: airspace.key(),
        drone_pda: ctx.accounts.drone.key(),
        lat,
        lon,
        alt_m,
        crossed_at,
        fee_lamports: airspace.fee_lamports,
    });

    Ok(())
}

/// Emitted for every successful drone airspace crossing.
#[event]
pub struct CrossingRecorded {
    pub airspace_pda: Pubkey,
    pub drone_pda: Pubkey,
    pub lat: i64,
    pub lon: i64,
    pub alt_m: u32,
    pub crossed_at: i64,
    pub fee_lamports: u64,
}
