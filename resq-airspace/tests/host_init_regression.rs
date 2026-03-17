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

use anchor_lang::{system_program, AccountDeserialize, InstructionData, ToAccountMetas};
use resq_airspace::state::airspace_account::{AccessPolicy, AirspaceAccount};
use solana_program_test::*;
use solana_sdk::{
    instruction::Instruction,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use solana_sdk::pubkey::Pubkey;

#[allow(unsafe_code)]
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[anchor_lang::solana_program::account_info::AccountInfo],
    data: &[u8],
) -> anchor_lang::solana_program::entrypoint::ProgramResult {
    resq_airspace::entry(program_id, unsafe { std::mem::transmute(accounts) }, data)
}

fn property_id_bytes(value: &str) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let bytes = value.as_bytes();
    let len = bytes.len().min(32);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf
}

fn airspace_pda(property_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"airspace", property_id], &resq_airspace::id())
}

#[tokio::test]
async fn host_processor_can_initialize_property_account() {
    let program = ProgramTest::new(
        "resq_airspace",
        resq_airspace::id(),
        processor!(process_instruction),
    );
    let (banks_client, payer, recent_blockhash) = program.start().await;

    let owner = Keypair::new();
    let property_id = property_id_bytes("host-init-regression");
    let (airspace, _) = airspace_pda(&property_id);

    let ix = Instruction {
        program_id: resq_airspace::id(),
        accounts: resq_airspace::accounts::InitializeProperty {
            owner: owner.pubkey(),
            airspace,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: resq_airspace::instruction::InitializeProperty {
            property_id,
            min_alt_m: 10,
            max_alt_m: 120,
            poly: [[0, 0]; 8],
            vertex_count: 1,
            policy: AccessPolicy::Open,
            fee_lamports: 0,
            treasury: owner.pubkey(),
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(
        &[
            solana_sdk::system_instruction::transfer(&payer.pubkey(), &owner.pubkey(), 1_000_000_000),
            ix,
        ],
        Some(&payer.pubkey()),
    );
    tx.sign(&[&payer, &owner], recent_blockhash);

    banks_client.process_transaction(tx).await.unwrap();

    let account = banks_client.get_account(airspace).await.unwrap().unwrap();
    let airspace: AirspaceAccount =
        AirspaceAccount::try_deserialize(&mut account.data.as_slice()).unwrap();
    assert_eq!(airspace.owner, owner.pubkey());
    assert_eq!(airspace.property_id, property_id);
}
