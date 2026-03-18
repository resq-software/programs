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

use anchor_lang::{InstructionData, ToAccountMetas, AccountDeserialize};
use solana_program_test::*;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    sysvar::clock::Clock,
    transaction::Transaction,
};
use solana_account_info::AccountInfo;
use solana_instruction::{AccountMeta, Instruction as SolanaInstruction};
use solana_keypair::Keypair as SolanaKeypair;
use solana_program_test::{processor, ProgramTest};
use solana_pubkey::Pubkey as SolanaPubkey;
use solana_sdk::program_error::ProgramError;
use solana_signer::Signer as SolanaSigner;
use solana_system_interface::instruction as system_instruction;
use solana_transaction::Transaction as SolanaTransaction;
use solana_program_entrypoint::ProgramResult;

use resq_airspace::state::airspace_account::{AccessPolicy, AirspaceAccount};
use resq_airspace::state::permit::Permit;

#[allow(unsafe_code)]
fn process_instruction(
    program_id: &SolanaPubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let program_id = anchor_pubkey(*program_id);
    resq_airspace::entry(&program_id, unsafe { std::mem::transmute(accounts) }, data)
        .map_err(|err| ProgramError::from(u64::from(err)))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn sdk_pubkey(value: anchor_lang::prelude::Pubkey) -> SolanaPubkey {
    SolanaPubkey::new_from_array(value.to_bytes())
}

fn anchor_pubkey(value: SolanaPubkey) -> anchor_lang::prelude::Pubkey {
    anchor_lang::prelude::Pubkey::new_from_array(value.to_bytes())
}

fn sdk_account_metas(value: Vec<anchor_lang::prelude::AccountMeta>) -> Vec<AccountMeta> {
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

fn str_to_bytes32(s: &str) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let bytes = s.as_bytes();
    let len = bytes.len().min(32);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf
}

fn airspace_pda(property_id_bytes: &[u8; 32]) -> (SolanaPubkey, u8) {
    SolanaPubkey::find_program_address(
        &[b"airspace", property_id_bytes],
        &sdk_pubkey(resq_airspace::id()),
    )
}

fn permit_pda(airspace: &SolanaPubkey, drone: &SolanaPubkey) -> (SolanaPubkey, u8) {
    SolanaPubkey::find_program_address(
        &[b"permit", airspace.as_ref(), drone.as_ref()],
        &sdk_pubkey(resq_airspace::id()),
    )
}

const UNIT_POLY: [[i64; 2]; 8] = [[0, 0]; 8];

async fn initialize_property_ix(
    owner: &SolanaPubkey,
    airspace_pda: &SolanaPubkey,
    treasury: &SolanaPubkey,
    pid: [u8; 32],
    min_alt_m: u32,
    max_alt_m: u32,
    vertex_count: u8,
    policy: AccessPolicy,
    fee_lamports: u64,
) -> SolanaInstruction {
    let data = resq_airspace::instruction::InitializeProperty {
        property_id: pid,
        min_alt_m,
        max_alt_m,
        poly: UNIT_POLY,
        vertex_count,
        policy,
        fee_lamports,
        treasury: anchor_pubkey(*treasury),
    }
    .data();

    let accounts = resq_airspace::accounts::InitializeProperty {
        owner: anchor_pubkey(*owner),
        airspace: anchor_pubkey(*airspace_pda),
        system_program: anchor_lang::system_program::ID,
    }
    .to_account_metas(None);

    SolanaInstruction {
        program_id: sdk_pubkey(resq_airspace::id()),
        accounts: sdk_account_metas(accounts),
        data,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_initialize_property_happy_path() {
    let program = ProgramTest::new(
        "resq_airspace",
        sdk_pubkey(resq_airspace::id()),
        processor!(process_instruction),
    );
    let (banks_client, payer, recent_blockhash) = program.start().await;

    let owner = Keypair::new();
    let pid = str_to_bytes32("property-open-001");
    let (pda, _) = airspace_pda(&pid);

    let ix = initialize_property_ix(
        &owner.pubkey(),
        &pda,
        &owner.pubkey(), // treasury
        pid,
        10,
        120,
        1, // vertex count
        AccessPolicy::Open,
        0, // fee
    ).await;

    // Airdrop some SOL to owner for rent
    let mut tx = SolanaTransaction::new_with_payer(
        &[
            system_instruction::transfer(&payer.pubkey(), &owner.pubkey(), 1000000000),
            ix,
        ],
        Some(&payer.pubkey()),
    );
    tx.sign(&[&payer, &owner], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let account = banks_client.get_account(pda).await.unwrap().unwrap();
    let acc: AirspaceAccount = AirspaceAccount::try_deserialize(&mut account.data.as_slice()).unwrap();

    assert_eq!(sdk_pubkey(acc.owner), owner.pubkey());
    assert_eq!(acc.policy, AccessPolicy::Open);
    assert_eq!(acc.min_alt_m, 10);
    assert_eq!(acc.max_alt_m, 120);
    assert_eq!(acc.fee_lamports, 0);
}

#[tokio::test]
async fn test_initialize_rejects_empty_property_id() {
    let program = ProgramTest::new(
        "resq_airspace",
        sdk_pubkey(resq_airspace::id()),
        processor!(process_instruction),
    );
    let (banks_client, payer, recent_blockhash) = program.start().await;

    let owner = Keypair::new();
    let pid = [0u8; 32];
    let (pda, _) = airspace_pda(&pid);

    let ix = initialize_property_ix(
        &owner.pubkey(),
        &pda,
        &owner.pubkey(),
        pid,
        10,
        120,
        1,
        AccessPolicy::Open,
        0,
    ).await;

    let mut tx = SolanaTransaction::new_with_payer(
        &[
            system_instruction::transfer(&payer.pubkey(), &owner.pubkey(), 1000000000),
            ix,
        ],
        Some(&payer.pubkey()),
    );
    tx.sign(&[&payer, &owner], recent_blockhash);

    let err = banks_client.process_transaction(tx).await.unwrap_err();
    assert!(err.unwrap().to_string().contains("EmptyPropertyId") || format!("{:?}", err).contains("Custom(6001)"));
}

#[tokio::test]
async fn test_grant_permit_happy_path() {
    let program = ProgramTest::new(
        "resq_airspace",
        sdk_pubkey(resq_airspace::id()),
        processor!(process_instruction),
    );
    let (banks_client, payer, recent_blockhash) = program.start().await;

    let owner = Keypair::new();
    let pid = str_to_bytes32("property-permit-001");
    let (airspace_pubkey, _) = airspace_pda(&pid);

    let init_ix = initialize_property_ix(
        &owner.pubkey(),
        &airspace_pubkey,
        &owner.pubkey(),
        pid,
        0,
        100,
        1,
        AccessPolicy::Permit,
        0,
    ).await;

    let drone = Keypair::new();
    let (p_pda, _) = permit_pda(&airspace_pubkey, &drone.pubkey());

    let grant_data = resq_airspace::instruction::GrantPermit {
        drone_pda: anchor_pubkey(drone.pubkey()),
        expires_at: 0,
    }.data();

    let grant_accounts = resq_airspace::accounts::GrantPermit {
        owner: anchor_pubkey(owner.pubkey()),
        airspace: anchor_pubkey(airspace_pubkey),
        permit: anchor_pubkey(p_pda),
        system_program: anchor_lang::system_program::ID,
    }.to_account_metas(None);

    let grant_ix = SolanaInstruction {
        program_id: sdk_pubkey(resq_airspace::id()),
        accounts: sdk_account_metas(grant_accounts),
        data: grant_data,
    };

    let mut tx = SolanaTransaction::new_with_payer(
        &[
            system_instruction::transfer(&payer.pubkey(), &owner.pubkey(), 1000000000),
            init_ix,
            grant_ix,
        ],
        Some(&payer.pubkey()),
    );
    tx.sign(&[&payer, &owner], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let account = banks_client.get_account(p_pda).await.unwrap().unwrap();
    let permit: Permit = Permit::try_deserialize(&mut account.data.as_slice()).unwrap();

    assert_eq!(sdk_pubkey(permit.drone_pda), drone.pubkey());
    assert_eq!(sdk_pubkey(permit.airspace), airspace_pubkey);
    assert_eq!(permit.expires_at, 0); // permanent
}

#[tokio::test]
async fn test_record_crossing_open_policy() {
    let program = ProgramTest::new(
        "resq_airspace",
        sdk_pubkey(resq_airspace::id()),
        processor!(process_instruction),
    );
    let (banks_client, payer, recent_blockhash) = program.start().await;

    let owner = Keypair::new();
    let drone = Keypair::new();
    let treasury = Keypair::new();
    let pid = str_to_bytes32("property-open-cross");
    let (airspace_pubkey, _) = airspace_pda(&pid);

    let init_ix = initialize_property_ix(
        &owner.pubkey(),
        &airspace_pubkey,
        &treasury.pubkey(),
        pid,
        0,
        100,
        1,
        AccessPolicy::Open,
        0,
    ).await;

    // Create airspace account
    let mut tx1 = SolanaTransaction::new_with_payer(
        &[
            system_instruction::transfer(&payer.pubkey(), &owner.pubkey(), 1000000000),
            init_ix,
        ],
        Some(&payer.pubkey()),
    );
    tx1.sign(&[&payer, &owner], recent_blockhash);
    banks_client.process_transaction(tx1).await.unwrap();

    let (p_pda, _) = permit_pda(&airspace_pubkey, &drone.pubkey());

    // With Optional permit, Open policy no longer requires a pre-existing permit.
    // We grant one here only to keep the account context valid; the handler ignores it.
    let grant_data = resq_airspace::instruction::GrantPermit {
        drone_pda: anchor_pubkey(drone.pubkey()),
        expires_at: 0,
    }.data();
    let grant_accounts = resq_airspace::accounts::GrantPermit {
        owner: anchor_pubkey(owner.pubkey()),
        airspace: anchor_pubkey(airspace_pubkey),
        permit: anchor_pubkey(p_pda),
        system_program: anchor_lang::system_program::ID,
    }.to_account_metas(None);
    let grant_ix = SolanaInstruction {
        program_id: sdk_pubkey(resq_airspace::id()),
        accounts: sdk_account_metas(grant_accounts),
        data: grant_data,
    };
    let mut tx_grant = SolanaTransaction::new_with_payer(
        &[grant_ix],
        Some(&payer.pubkey()),
    );
    let recent_blockhash2 = banks_client.get_latest_blockhash().await.unwrap();
    tx_grant.sign(&[&payer, &owner], recent_blockhash2);
    banks_client.process_transaction(tx_grant).await.unwrap();

    // Derive crossed_at from the live test clock so it falls within the 5-minute
    // look-back window enforced by the program.
    let clock: Clock = banks_client.get_sysvar().await.unwrap();
    let crossed_at = clock.unix_timestamp - 30;

    let cross_data = resq_airspace::instruction::RecordCrossing {
        lat: 407128000,
        lon: -740060000,
        alt_m: 50,
        crossed_at,
    }.data();

    // Open policy does not require a permit; pass None.
    let cross_accounts = resq_airspace::accounts::RecordCrossing {
        drone: anchor_pubkey(drone.pubkey()),
        airspace: anchor_pubkey(airspace_pubkey),
        permit: None,
        treasury: anchor_pubkey(treasury.pubkey()),
        system_program: anchor_lang::system_program::ID,
    }.to_account_metas(None);

    let cross_ix = SolanaInstruction {
        program_id: sdk_pubkey(resq_airspace::id()),
        accounts: sdk_account_metas(cross_accounts),
        data: cross_data,
    };

    let mut tx2 = SolanaTransaction::new_with_payer(
        &[
            system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 1000000000),
            cross_ix,
        ],
        Some(&payer.pubkey()),
    );
    let recent_blockhash2 = banks_client.get_latest_blockhash().await.unwrap();
    tx2.sign(&[&payer, &drone], recent_blockhash2);
    banks_client.process_transaction(tx2).await.unwrap();
}

#[tokio::test]
async fn test_record_crossing_deny_policy() {
    let program = ProgramTest::new(
        "resq_airspace",
        sdk_pubkey(resq_airspace::id()),
        processor!(process_instruction),
    );
    let (banks_client, payer, recent_blockhash) = program.start().await;

    let owner = Keypair::new();
    let drone = Keypair::new();
    let treasury = Keypair::new();
    let pid = str_to_bytes32("property-deny-cross");
    let (airspace_pubkey, _) = airspace_pda(&pid);

    let init_ix = initialize_property_ix(
        &owner.pubkey(),
        &airspace_pubkey,
        &treasury.pubkey(),
        pid,
        0,
        100,
        1,
        AccessPolicy::Deny,
        0,
    ).await;

    let mut tx1 = SolanaTransaction::new_with_payer(
        &[
            system_instruction::transfer(&payer.pubkey(), &owner.pubkey(), 1000000000),
            init_ix,
        ],
        Some(&payer.pubkey()),
    );
    tx1.sign(&[&payer, &owner], recent_blockhash);
    banks_client.process_transaction(tx1).await.unwrap();

    let (p_pda, _) = permit_pda(&airspace_pubkey, &drone.pubkey());

    // Grant a permit so that Anchor deserialize succeeds, and we reach AccessPolicy::Deny logic
    let grant_data = resq_airspace::instruction::GrantPermit {
        drone_pda: anchor_pubkey(drone.pubkey()),
        expires_at: 0,
    }.data();
    let grant_accounts = resq_airspace::accounts::GrantPermit {
        owner: anchor_pubkey(owner.pubkey()),
        airspace: anchor_pubkey(airspace_pubkey),
        permit: anchor_pubkey(p_pda),
        system_program: anchor_lang::system_program::ID,
    }.to_account_metas(None);
    let mut tx_grant = SolanaTransaction::new_with_payer(
        &[
            SolanaInstruction {
                program_id: sdk_pubkey(resq_airspace::id()),
                accounts: sdk_account_metas(grant_accounts),
                data: grant_data,
            }
        ],
        Some(&payer.pubkey()),
    );
    let recent_blockhash_grant = banks_client.get_latest_blockhash().await.unwrap();
    tx_grant.sign(&[&payer, &owner], recent_blockhash_grant);
    banks_client.process_transaction(tx_grant).await.unwrap();

    // Deny policy rejects before timestamp/altitude checks, but use a
    // realistic timestamp in case policy order ever changes.
    let clock: Clock = banks_client.get_sysvar().await.unwrap();
    let crossed_at = clock.unix_timestamp - 30;

    let cross_data = resq_airspace::instruction::RecordCrossing {
        lat: 0,
        lon: 0,
        alt_m: 50, // within min_alt_m=0, max_alt_m=100
        crossed_at,
    }.data();

    let cross_accounts = resq_airspace::accounts::RecordCrossing {
        drone: anchor_pubkey(drone.pubkey()),
        airspace: anchor_pubkey(airspace_pubkey),
        permit: Some(anchor_pubkey(p_pda)),
        treasury: anchor_pubkey(treasury.pubkey()),
        system_program: anchor_lang::system_program::ID,
    }.to_account_metas(None);

    let cross_ix = SolanaInstruction {
        program_id: sdk_pubkey(resq_airspace::id()),
        accounts: sdk_account_metas(cross_accounts),
        data: cross_data,
    };

    let mut tx2 = SolanaTransaction::new_with_payer(
        &[
            system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 1000000000),
            cross_ix,
        ],
        Some(&payer.pubkey()),
    );

    let recent_blockhash2 = banks_client.get_latest_blockhash().await.unwrap();
    tx2.sign(&[&payer, &drone], recent_blockhash2);

    let err = banks_client.process_transaction(tx2).await.unwrap_err();
    assert!(err.unwrap().to_string().contains("NoValidPermit") || format!("{:?}", err).contains("Custom(6004)"));
}

#[tokio::test]
async fn test_update_policy() {
    let program = ProgramTest::new(
        "resq_airspace",
        sdk_pubkey(resq_airspace::id()),
        processor!(process_instruction),
    );
    let (banks_client, payer, recent_blockhash) = program.start().await;

    let owner = Keypair::new();
    let pid = str_to_bytes32("property-policy-update");
    let (airspace_pubkey, _) = airspace_pda(&pid);

    let init_ix = initialize_property_ix(
        &owner.pubkey(),
        &airspace_pubkey,
        &owner.pubkey(),
        pid,
        0,
        100,
        1,
        AccessPolicy::Open,
        0,
    ).await;

    let update_data = resq_airspace::instruction::UpdatePolicy {
        policy: AccessPolicy::Deny,
        fee_lamports: 0,
    }.data();

    let update_accounts = resq_airspace::accounts::UpdatePolicy {
        owner: anchor_pubkey(owner.pubkey()),
        airspace: anchor_pubkey(airspace_pubkey),
    }.to_account_metas(None);

    let update_ix = SolanaInstruction {
        program_id: sdk_pubkey(resq_airspace::id()),
        accounts: sdk_account_metas(update_accounts),
        data: update_data,
    };

    let mut tx = SolanaTransaction::new_with_payer(
        &[
            system_instruction::transfer(&payer.pubkey(), &owner.pubkey(), 1000000000),
            init_ix,
            update_ix,
        ],
        Some(&payer.pubkey()),
    );
    tx.sign(&[&payer, &owner], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let account = banks_client.get_account(airspace_pubkey).await.unwrap().unwrap();
    let acc: AirspaceAccount = AirspaceAccount::try_deserialize(&mut account.data.as_slice()).unwrap();

    assert_eq!(acc.policy, AccessPolicy::Deny);
}
