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

use {
    solana_keypair::Keypair,
    solana_program_test::ProgramTestContext,
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    solana_signer::Signer,
    solana_stake_interface::{
        instruction as stake_instruction,
        state::{Authorized, Lockup},
    },
    solana_system_interface::{instruction as system_instruction, program as system_program},
    solana_transaction::Transaction,
    solana_vote_program::{
        vote_instruction,
        vote_state::{self, VoteInit, VoteStateV4},
    },
};

pub async fn setup_stake(
    context: &mut ProgramTestContext,
    user: &Keypair,
    vote_address: &Pubkey,
    stake_lamports: u64,
) -> Pubkey {
    let stake_keypair = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &stake_instruction::create_account_and_delegate_stake(
            &context.payer.pubkey(),
            &stake_keypair.pubkey(),
            vote_address,
            &Authorized::auto(&user.pubkey()),
            &Lockup::default(),
            stake_lamports,
        ),
        Some(&context.payer.pubkey()),
        &vec![&context.payer, &stake_keypair, user],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
    stake_keypair.pubkey()
}

pub async fn setup_vote(context: &mut ProgramTestContext) -> Pubkey {
    let mut instructions = vec![];
    let validator_keypair = Keypair::new();
    instructions.push(system_instruction::create_account(
        &context.payer.pubkey(),
        &validator_keypair.pubkey(),
        Rent::default().minimum_balance(0),
        0,
        &system_program::id(),
    ));
    let vote_lamports = Rent::default().minimum_balance(VoteStateV4::size_of());
    let vote_keypair = Keypair::new();
    let user_keypair = Keypair::new();
    instructions.append(&mut vote_instruction::create_account_with_config(
        &context.payer.pubkey(),
        &vote_keypair.pubkey(),
        &VoteInit {
            node_pubkey: validator_keypair.pubkey(),
            authorized_voter: user_keypair.pubkey(),
            ..VoteInit::default()
        },
        vote_lamports,
        vote_instruction::CreateVoteAccountConfig {
            space: vote_state::VoteStateV4::size_of() as u64,
            ..vote_instruction::CreateVoteAccountConfig::default()
        },
    ));

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &vec![&context.payer, &validator_keypair, &vote_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    vote_keypair.pubkey()
}
