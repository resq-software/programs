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
    system_program,
    AccountDeserialize,
    InstructionData,
    ToAccountMetas,
};
use solana_program_test::*;
use solana_sdk::{
    instruction::Instruction,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use solana_sdk::pubkey::Pubkey;

use resq_airspace::state::airspace_account::{AccessPolicy, AirspaceAccount};
use resq_airspace::state::permit::Permit;

#[allow(unsafe_code)]
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[anchor_lang::solana_program::account_info::AccountInfo],
    data: &[u8],
) -> anchor_lang::solana_program::entrypoint::ProgramResult {
    resq_airspace::entry(program_id, unsafe { std::mem::transmute(accounts) }, data)
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn str_to_bytes32(s: &str) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let bytes = s.as_bytes();
    let len = bytes.len().min(32);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf
}

fn airspace_pda(property_id_bytes: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"airspace", property_id_bytes],
        &resq_airspace::id(),
    )
}

fn permit_pda(airspace: &Pubkey, drone: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"permit", airspace.as_ref(), drone.as_ref()],
        &resq_airspace::id(),
    )
}

const UNIT_POLY: [[i64; 2]; 8] = [[0, 0]; 8];

async fn initialize_property_ix(
    owner: &Pubkey,
    airspace_pda: &Pubkey,
    treasury: &Pubkey,
    pid: [u8; 32],
    min_alt_m: u32,
    max_alt_m: u32,
    vertex_count: u8,
    policy: AccessPolicy,
    fee_lamports: u64,
) -> Instruction {
    let data = resq_airspace::instruction::InitializeProperty {
        property_id: pid,
        min_alt_m,
        max_alt_m,
        poly: UNIT_POLY,
        vertex_count,
        policy,
        fee_lamports,
        treasury: *treasury,
    }
    .data();

    let accounts = resq_airspace::accounts::InitializeProperty {
        owner: *owner,
        airspace: *airspace_pda,
        system_program: system_program::ID,
    }
    .to_account_metas(None);

    Instruction {
        program_id: resq_airspace::id(),
        accounts,
        data,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_initialize_property_happy_path() {
    let program = ProgramTest::new(
        "resq_airspace",
        resq_airspace::id(),
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
    let mut tx = Transaction::new_with_payer(
        &[
            solana_sdk::system_instruction::transfer(&payer.pubkey(), &owner.pubkey(), 1000000000),
            ix,
        ],
        Some(&payer.pubkey()),
    );
    tx.sign(&[&payer, &owner], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let account = banks_client.get_account(pda).await.unwrap().unwrap();
    let acc: AirspaceAccount = AirspaceAccount::try_deserialize(&mut account.data.as_slice()).unwrap();

    assert_eq!(acc.owner, owner.pubkey());
    assert_eq!(acc.policy, AccessPolicy::Open);
    assert_eq!(acc.min_alt_m, 10);
    assert_eq!(acc.max_alt_m, 120);
    assert_eq!(acc.fee_lamports, 0);
}

#[tokio::test]
async fn test_initialize_rejects_empty_property_id() {
    let program = ProgramTest::new(
        "resq_airspace",
        resq_airspace::id(),
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

    let mut tx = Transaction::new_with_payer(
        &[
            solana_sdk::system_instruction::transfer(&payer.pubkey(), &owner.pubkey(), 1000000000),
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
        resq_airspace::id(),
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
        drone_pda: drone.pubkey(),
        expires_at: 0,
    }.data();

    let grant_accounts = resq_airspace::accounts::GrantPermit {
        owner: owner.pubkey(),
        airspace: airspace_pubkey,
        permit: p_pda,
        system_program: system_program::ID,
    }.to_account_metas(None);

    let grant_ix = Instruction {
        program_id: resq_airspace::id(),
        accounts: grant_accounts,
        data: grant_data,
    };

    let mut tx = Transaction::new_with_payer(
        &[
            solana_sdk::system_instruction::transfer(&payer.pubkey(), &owner.pubkey(), 1000000000),
            init_ix,
            grant_ix,
        ],
        Some(&payer.pubkey()),
    );
    tx.sign(&[&payer, &owner], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let account = banks_client.get_account(p_pda).await.unwrap().unwrap();
    let permit: Permit = Permit::try_deserialize(&mut account.data.as_slice()).unwrap();

    assert_eq!(permit.drone_pda, drone.pubkey());
    assert_eq!(permit.airspace, airspace_pubkey);
    assert_eq!(permit.expires_at, 0); // permanent
}

#[tokio::test]
async fn test_record_crossing_open_policy() {
    let program = ProgramTest::new(
        "resq_airspace",
        resq_airspace::id(),
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
    let mut tx1 = Transaction::new_with_payer(
        &[
            solana_sdk::system_instruction::transfer(&payer.pubkey(), &owner.pubkey(), 1000000000),
            init_ix,
        ],
        Some(&payer.pubkey()),
    );
    tx1.sign(&[&payer, &owner], recent_blockhash);
    banks_client.process_transaction(tx1).await.unwrap();

    let (p_pda, _) = permit_pda(&airspace_pubkey, &drone.pubkey());

    // Grant a permit so Anchor doesn't throw AccountNotInitialized
    let grant_data = resq_airspace::instruction::GrantPermit {
        drone_pda: drone.pubkey(),
        expires_at: 0,
    }.data();
    let grant_accounts = resq_airspace::accounts::GrantPermit {
        owner: owner.pubkey(),
        airspace: airspace_pubkey,
        permit: p_pda,
        system_program: system_program::ID,
    }.to_account_metas(None);
    let grant_ix = Instruction {
        program_id: resq_airspace::id(),
        accounts: grant_accounts,
        data: grant_data,
    };
    let mut tx_grant = Transaction::new_with_payer(
        &[grant_ix],
        Some(&payer.pubkey()),
    );
    let recent_blockhash2 = banks_client.get_latest_blockhash().await.unwrap();
    tx_grant.sign(&[&payer, &owner], recent_blockhash2);
    banks_client.process_transaction(tx_grant).await.unwrap();

    // Record crossing
    let crossed_at = 1680000000_i64;

    let cross_data = resq_airspace::instruction::RecordCrossing {
        lat: 407128000,
        lon: -740060000,
        alt_m: 50,
        crossed_at,
    }.data();

    let cross_accounts = resq_airspace::accounts::RecordCrossing {
        drone: drone.pubkey(),
        airspace: airspace_pubkey,
        permit: p_pda, // Doesn't need to exist for Open policy actually, but account required in ctx
        treasury: treasury.pubkey(),
        system_program: system_program::ID,
    }.to_account_metas(None);

    let cross_ix = Instruction {
        program_id: resq_airspace::id(),
        accounts: cross_accounts,
        data: cross_data,
    };

    let mut tx2 = Transaction::new_with_payer(
        &[
            solana_sdk::system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 1000000000),
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
        resq_airspace::id(),
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

    let mut tx1 = Transaction::new_with_payer(
        &[
            solana_sdk::system_instruction::transfer(&payer.pubkey(), &owner.pubkey(), 1000000000),
            init_ix,
        ],
        Some(&payer.pubkey()),
    );
    tx1.sign(&[&payer, &owner], recent_blockhash);
    banks_client.process_transaction(tx1).await.unwrap();

    let (p_pda, _) = permit_pda(&airspace_pubkey, &drone.pubkey());

    // Grant a permit so that Anchor deserialize succeeds, and we reach AccessPolicy::Deny logic
    let grant_data = resq_airspace::instruction::GrantPermit {
        drone_pda: drone.pubkey(),
        expires_at: 0,
    }.data();
    let grant_accounts = resq_airspace::accounts::GrantPermit {
        owner: owner.pubkey(),
        airspace: airspace_pubkey,
        permit: p_pda,
        system_program: system_program::ID,
    }.to_account_metas(None);
    let mut tx_grant = Transaction::new_with_payer(
        &[
            Instruction {
                program_id: resq_airspace::id(),
                accounts: grant_accounts,
                data: grant_data,
            }
        ],
        Some(&payer.pubkey()),
    );
    let recent_blockhash_grant = banks_client.get_latest_blockhash().await.unwrap();
    tx_grant.sign(&[&payer, &owner], recent_blockhash_grant);
    banks_client.process_transaction(tx_grant).await.unwrap();

    let crossed_at = 1680000000_i64;

    let cross_data = resq_airspace::instruction::RecordCrossing {
        lat: 0,
        lon: 0,
        alt_m: 0,
        crossed_at,
    }.data();

    let cross_accounts = resq_airspace::accounts::RecordCrossing {
        drone: drone.pubkey(),
        airspace: airspace_pubkey,
        permit: p_pda,
        treasury: treasury.pubkey(),
        system_program: system_program::ID,
    }.to_account_metas(None);

    let cross_ix = Instruction {
        program_id: resq_airspace::id(),
        accounts: cross_accounts,
        data: cross_data,
    };

    let mut tx2 = Transaction::new_with_payer(
        &[
            solana_sdk::system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 1000000000),
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
        resq_airspace::id(),
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
        owner: owner.pubkey(),
        airspace: airspace_pubkey,
    }.to_account_metas(None);

    let update_ix = Instruction {
        program_id: resq_airspace::id(),
        accounts: update_accounts,
        data: update_data,
    };

    let mut tx = Transaction::new_with_payer(
        &[
            solana_sdk::system_instruction::transfer(&payer.pubkey(), &owner.pubkey(), 1000000000),
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
