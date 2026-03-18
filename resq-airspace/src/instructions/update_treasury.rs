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
pub struct UpdateTreasury<'info> {
    /// Must be the current airspace owner.
    pub owner: Signer<'info>,

    /// The AirspaceAccount to modify.
    #[account(
        mut,
        has_one = owner @ AirspaceError::Unauthorized,
    )]
    pub airspace: Account<'info, AirspaceAccount>,
}

/// Update the treasury address that receives per-crossing fees.
///
/// Only the registered owner may call this instruction.
///
/// # Arguments
/// * `treasury` – new SOL account that will receive crossing fees
pub fn handler(ctx: Context<UpdateTreasury>, treasury: Pubkey) -> Result<()> {
    require!(treasury != Pubkey::default(), AirspaceError::InvalidTreasury);
    ctx.accounts.airspace.treasury = treasury;

    emit!(TreasuryUpdated {
        airspace_pda: ctx.accounts.airspace.key(),
        treasury,
    });

    Ok(())
}

/// Emitted when the treasury address is changed.
#[event]
pub struct TreasuryUpdated {
    pub airspace_pda: Pubkey,
    pub treasury: Pubkey,
}
