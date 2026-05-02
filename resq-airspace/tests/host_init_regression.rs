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

use anchor_lang::{
    prelude::{AccountMeta as AnchorAccountMeta, Pubkey as AnchorPubkey},
    system_program as anchor_system_program,
    AccountDeserialize,
    InstructionData,
    ToAccountMetas,
};
use resq_airspace::state::airspace_account::{AccessPolicy, AirspaceAccount};
use solana_account_info::AccountInfo;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_program_test::{processor, ProgramTest};
use solana_pubkey::Pubkey;
use solana_sdk::program_error::ProgramError;
use solana_signer::Signer;
use solana_system_interface::instruction as system_instruction;
use solana_transaction::Transaction;
use solana_program_entrypoint::ProgramResult;

#[allow(unsafe_code)]
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let program_id = anchor_pubkey(*program_id);
    resq_airspace::entry(&program_id, unsafe { std::mem::transmute(accounts) }, data)
        .map_err(|err| ProgramError::from(u64::from(err)))
}

fn property_id_bytes(value: &str) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let bytes = value.as_bytes();
    let len = bytes.len().min(32);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf
}

fn sdk_pubkey(value: AnchorPubkey) -> Pubkey {
    Pubkey::new_from_array(value.to_bytes())
}

fn anchor_pubkey(value: Pubkey) -> AnchorPubkey {
    AnchorPubkey::new_from_array(value.to_bytes())
}

fn sdk_account_metas(value: Vec<AnchorAccountMeta>) -> Vec<AccountMeta> {
    value.into_iter()
        .map(|meta| {
            let pubkey = sdk_pubkey(meta.pubkey);
            if meta.is_writable {
                AccountMeta::new(pubkey, meta.is_signer)
            } else {
                AccountMeta::new_readonly(pubkey, meta.is_signer)
            }
        })
        .collect()
}

fn airspace_pda(property_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"airspace", property_id], &sdk_pubkey(resq_airspace::id()))
}

#[tokio::test]
async fn host_processor_can_initialize_property_account() {
    let program = ProgramTest::new(
        "resq_airspace",
        sdk_pubkey(resq_airspace::id()),
        processor!(process_instruction),
    );
    let (banks_client, payer, recent_blockhash) = program.start().await;

    let owner = Keypair::new();
    let property_id = property_id_bytes("host-init-regression");
    let (airspace, _) = airspace_pda(&property_id);

    let ix = Instruction {
        program_id: sdk_pubkey(resq_airspace::id()),
        accounts: sdk_account_metas(
            resq_airspace::accounts::InitializeProperty {
                owner: anchor_pubkey(owner.pubkey()),
                airspace: anchor_pubkey(airspace),
                system_program: anchor_system_program::ID,
            }
            .to_account_metas(None),
        ),
        data: resq_airspace::instruction::InitializeProperty {
            property_id,
            min_alt_m: 10,
            max_alt_m: 120,
            poly: [[0, 0]; 8],
            vertex_count: 1,
            policy: AccessPolicy::Open,
            fee_lamports: 0,
            treasury: anchor_pubkey(owner.pubkey()),
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(
        &[
            system_instruction::transfer(&payer.pubkey(), &owner.pubkey(), 1_000_000_000),
            ix,
        ],
        Some(&payer.pubkey()),
    );
    tx.sign(&[&payer, &owner], recent_blockhash);

    banks_client.process_transaction(tx).await.unwrap();

    let account = banks_client.get_account(airspace).await.unwrap().unwrap();
    let airspace: AirspaceAccount =
        AirspaceAccount::try_deserialize(&mut account.data.as_slice()).unwrap();
    assert_eq!(sdk_pubkey(airspace.owner), owner.pubkey());
    assert_eq!(airspace.property_id, property_id);
}
