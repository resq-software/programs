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
    /// Required when `airspace.policy` is `Permit` or `Auction`; omit (pass None) for `Open`.
    /// When provided, Anchor verifies the PDA derivation against the drone signer.
    #[account(
        seeds = [b"permit", airspace.key().as_ref(), drone.key().as_ref()],
        bump,
    )]
    pub permit: Option<Account<'info, Permit>>,

    /// The treasury account that receives the crossing fee (if any).
    /// Only consulted when `airspace.policy` is `Permit` or `Auction` and
    /// `airspace.fee_lamports > 0`.
    /// CHECK: must match `airspace.treasury`; validated inside the permit arm.
    #[account(mut)]
    pub treasury: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

/// Record a drone airspace crossing event.
///
/// Fee collection is tied to the access policy:
/// - `Open`    – always allowed, no fee charged.
/// - `Permit`  – valid permit required; crossing fee collected if configured.
/// - `Auction` – same as `Permit` (auction mechanism is a future extension).
/// - `Deny`    – always rejected.
///
/// # Arguments
/// * `lat`        – latitude × 1e7 (range −900_000_000 to +900_000_000)
/// * `lon`        – longitude × 1e7 (range −1_800_000_000 to +1_800_000_000)
/// * `alt_m`      – altitude in metres (enforced against airspace altitude bounds)
/// * `crossed_at` – Unix timestamp (seconds) of the crossing; must be within the
///                  5-minute look-back window and no more than 60 seconds ahead
pub fn handler(
    ctx: Context<RecordCrossing>,
    lat: i64,
    lon: i64,
    alt_m: u32,
    crossed_at: i64,
) -> Result<()> {
    let airspace = &ctx.accounts.airspace;
    let clock = Clock::get()?;

    // Deny check first — no further state is examined for denied airspaces.
    if airspace.policy == AccessPolicy::Deny {
        return err!(AirspaceError::NoValidPermit);
    }

    // Timestamp sanity: positive, not from the distant past, not fabricated.
    require!(crossed_at > 0, AirspaceError::InvalidTimestamp);
    require!(
        crossed_at >= clock.unix_timestamp - 300,
        AirspaceError::TimestampTooOld
    );
    require!(
        crossed_at <= clock.unix_timestamp + 60,
        AirspaceError::TimestampInFuture
    );

    // Coordinate range validation (mirrors record_delivery).
    require!(
        lat >= -900_000_000 && lat <= 900_000_000,
        AirspaceError::LatitudeOutOfRange
    );
    require!(
        lon >= -1_800_000_000 && lon <= 1_800_000_000,
        AirspaceError::LongitudeOutOfRange
    );

    // Altitude bounds declared for this airspace.
    require!(
        alt_m >= airspace.min_alt_m && alt_m <= airspace.max_alt_m,
        AirspaceError::AltitudeOutOfBounds
    );

    // Policy-specific permit check and fee collection.
    // Open policy: unconditionally allowed, no fee charged.
    // Permit/Auction: valid permit required; fee collected if configured.
    // Track the amount actually transferred so the event is accurate.
    let mut fee_paid: u64 = 0;

    match airspace.policy {
        AccessPolicy::Open => {} // no permit, no fee
        AccessPolicy::Permit | AccessPolicy::Auction => {
            let permit = ctx
                .accounts
                .permit
                .as_ref()
                .ok_or(AirspaceError::NoValidPermit)?;
            require!(permit.is_active(clock.unix_timestamp), AirspaceError::PermitExpired);

            // Collect per-crossing fee when configured.
            if airspace.fee_lamports > 0 {
                require_keys_eq!(
                    ctx.accounts.treasury.key(),
                    airspace.treasury,
                    AirspaceError::FeeTransferFailed
                );
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
                fee_paid = airspace.fee_lamports;
            }
        }
        // Deny was already handled above.
        AccessPolicy::Deny => unreachable!(),
    }

    emit!(CrossingRecorded {
        airspace_pda: airspace.key(),
        drone_pda: ctx.accounts.drone.key(),
        lat,
        lon,
        alt_m,
        crossed_at,
        fee_lamports: fee_paid,
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
