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

use crate::{error::AirspaceError, state::airspace_account::AirspaceAccount};

#[derive(Accounts)]
pub struct TransferOwnership<'info> {
    /// The current airspace owner — must sign the transfer.
    pub owner: Signer<'info>,

    /// The AirspaceAccount whose ownership is being transferred.
    #[account(
        mut,
        has_one = owner @ AirspaceError::Unauthorized,
    )]
    pub airspace: Account<'info, AirspaceAccount>,
}

/// Transfer ownership of an airspace to a new authority.
///
/// Only the current owner may call this.  The new owner must not be the
/// zero address.  After this instruction completes, the old owner has no
/// further authority over the airspace.
///
/// # Arguments
/// * `new_owner` – pubkey that will become the new airspace authority
pub fn handler(ctx: Context<TransferOwnership>, new_owner: Pubkey) -> Result<()> {
    require!(new_owner != Pubkey::default(), AirspaceError::InvalidOwner);
    ctx.accounts.airspace.owner = new_owner;

    emit!(OwnershipTransferred {
        airspace_pda: ctx.accounts.airspace.key(),
        previous_owner: ctx.accounts.owner.key(),
        new_owner,
    });

    Ok(())
}

/// Emitted when airspace ownership changes hands.
#[event]
pub struct OwnershipTransferred {
    pub airspace_pda: Pubkey,
    pub previous_owner: Pubkey,
    pub new_owner: Pubkey,
}
