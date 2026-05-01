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
pub struct UpdatePolicy<'info> {
    /// Must be the current airspace owner.
    pub owner: Signer<'info>,

    /// The AirspaceAccount to modify.
    #[account(
        mut,
        has_one = owner @ AirspaceError::Unauthorized,
    )]
    pub airspace: Account<'info, AirspaceAccount>,
}

/// Update the access policy and/or per-crossing fee for an airspace.
///
/// Only the registered owner may call this instruction.
pub fn handler(
    ctx: Context<UpdatePolicy>,
    policy: AccessPolicy,
    fee_lamports: u64,
) -> Result<()> {
    let airspace = &mut ctx.accounts.airspace;
    airspace.policy = policy;
    airspace.fee_lamports = fee_lamports;

    emit!(PolicyUpdated {
        airspace_pda: ctx.accounts.airspace.key(),
        policy,
        fee_lamports,
    });

    Ok(())
}

/// Emitted when an airspace policy or fee is changed.
#[event]
pub struct PolicyUpdated {
    pub airspace_pda: Pubkey,
    pub policy: AccessPolicy,
    pub fee_lamports: u64,
}