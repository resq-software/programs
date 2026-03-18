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

use crate::{error::DeliveryError, state::delivery_record::DeliveryRecord};

/// Accounts required to record a new delivery on-chain.
///
/// The `drone` signer represents the drone's authority keypair (ideally backed
/// by a secure enclave).  In production the PDA is derived from the drone
/// serial and a mission-scoped ephemeral key — see `docs/DRONE_KEY_MANAGEMENT.md`.
#[derive(Accounts)]
#[instruction(ipfs_cid: [u8; 64], lat: i64, lon: i64, alt_m: u32, delivered_at: i64)]
pub struct RecordDelivery<'info> {
    /// The drone authority — must sign the transaction.
    #[account(mut)]
    pub drone: Signer<'info>,

    /// The AirspaceAccount of the delivery target property (read-only reference).
    /// CHECK: validated by the resq-airspace program; we only store the address.
    pub airspace: UncheckedAccount<'info>,

    /// The new DeliveryRecord PDA, created by this instruction.
    #[account(
        init,
        payer = drone,
        space = DeliveryRecord::LEN,
        seeds = [
            b"delivery",
            drone.key().as_ref(),
            &delivered_at.to_le_bytes(),
        ],
        bump,
    )]
    pub delivery_record: Account<'info, DeliveryRecord>,

    pub system_program: Program<'info, System>,
}

/// Record an immutable proof-of-delivery on-chain.
///
/// # Arguments
/// * `ipfs_cid`     – 64-byte null-padded base58 IPFS content identifier
/// * `lat`          – latitude × 1e7 (range −900_000_000 to +900_000_000)
/// * `lon`          – longitude × 1e7 (range −1_800_000_000 to +1_800_000_000)
/// * `alt_m`        – altitude in metres
/// * `delivered_at` – Unix timestamp (seconds) of delivery confirmation
pub fn handler(
    ctx: Context<RecordDelivery>,
    ipfs_cid: [u8; 64],
    lat: i64,
    lon: i64,
    alt_m: u32,
    delivered_at: i64,
) -> Result<()> {
    require!(ipfs_cid != [0u8; 64], DeliveryError::EmptyCid);
    require!(delivered_at > 0, DeliveryError::InvalidTimestamp);
    require!(
        lat >= -900_000_000 && lat <= 900_000_000,
        DeliveryError::LatitudeOutOfRange
    );
    require!(
        lon >= -1_800_000_000 && lon <= 1_800_000_000,
        DeliveryError::LongitudeOutOfRange
    );

    let rec = &mut ctx.accounts.delivery_record;
    rec.drone_pda = ctx.accounts.drone.key();
    rec.airspace_pda = ctx.accounts.airspace.key();
    rec.ipfs_cid = ipfs_cid;
    rec.lat = lat;
    rec.lon = lon;
    rec.alt_m = alt_m;
    rec.delivered_at = delivered_at;
    rec.bump = ctx.bumps.delivery_record;

    emit!(DeliveryRecorded {
        drone_pda: rec.drone_pda,
        airspace_pda: rec.airspace_pda,
        delivered_at,
    });

    Ok(())
}

/// Emitted when a new DeliveryRecord is successfully written.
#[event]
pub struct DeliveryRecorded {
    pub drone_pda: Pubkey,
    pub airspace_pda: Pubkey,
    pub delivered_at: i64,
}
