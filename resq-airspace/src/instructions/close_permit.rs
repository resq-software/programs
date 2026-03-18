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
pub struct ClosePermit<'info> {
    /// Must be the airspace owner.  Receives the reclaimed rent lamports.
    #[account(mut)]
    pub owner: Signer<'info>,

    /// The AirspaceAccount the permit belongs to.
    #[account(
        has_one = owner @ AirspaceError::Unauthorized,
    )]
    pub airspace: Account<'info, AirspaceAccount>,

    /// The Permit PDA to close.  Rent is returned to `owner`.
    #[account(
        mut,
        seeds = [b"permit", airspace.key().as_ref(), permit.drone_pda.as_ref()],
        bump = permit.bump,
        close = owner,
    )]
    pub permit: Account<'info, Permit>,

    pub system_program: Program<'info, System>,
}

/// Close an existing Permit account, reclaiming its rent to the airspace owner.
///
/// After closing, the owner may call `grant_permit` again for the same
/// (airspace, drone) pair to issue a fresh permit with a new expiry.
pub fn handler(ctx: Context<ClosePermit>) -> Result<()> {
    emit!(PermitClosed {
        airspace_pda: ctx.accounts.airspace.key(),
        drone_pda: ctx.accounts.permit.drone_pda,
    });

    Ok(())
}

/// Emitted when a Permit account is closed by the airspace owner.
#[event]
pub struct PermitClosed {
    pub airspace_pda: Pubkey,
    pub drone_pda: Pubkey,
}
