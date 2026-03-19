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
use solana_account::Account;
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

use anchor_lang::Discriminator;
use resq_airspace::state::airspace_account::AirspaceAccount;
use resq_delivery::state::delivery_record::DeliveryRecord;

#[allow(unsafe_code)]
fn process_instruction(
    program_id: &SolanaPubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let program_id = anchor_pubkey(*program_id);
    resq_delivery::entry(&program_id, unsafe { std::mem::transmute(accounts) }, data)
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

fn cid_to_bytes(cid: &str) -> [u8; 64] {
    let mut buf = [0u8; 64];
    let bytes = cid.as_bytes();
    let len = bytes.len().min(64);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf
}

fn delivery_pda(drone: &SolanaPubkey, delivered_at: i64) -> (SolanaPubkey, u8) {
    SolanaPubkey::find_program_address(
        &[b"delivery", drone.as_ref(), &delivered_at.to_le_bytes()],
        &sdk_pubkey(resq_delivery::id()),
    )
}

/// Add a valid-looking AirspaceAccount owned by the resq-airspace program to
/// the test validator.  The delivery program uses `Account<'info, AirspaceAccount>`
/// which checks both owner == resq-airspace program ID *and* the 8-byte
/// Anchor discriminator, so the account data must be at least `AirspaceAccount::LEN`
/// bytes with the correct discriminator prefix.
fn seed_airspace_account(program: &mut ProgramTest) -> SolanaPubkey {
    let airspace_pubkey = SolanaPubkey::new_unique();
    let mut data = vec![0u8; AirspaceAccount::LEN];
    data[..8].copy_from_slice(&AirspaceAccount::DISCRIMINATOR);
    program.add_account(
        airspace_pubkey,
        Account {
            lamports: 1_000_000,
            data,
            owner: sdk_pubkey(resq_airspace::id()),
            executable: false,
            rent_epoch: 0,
        },
    );
    airspace_pubkey
}

fn create_record_ix(
    drone: &SolanaPubkey,
    airspace: &SolanaPubkey,
    record_pda: &SolanaPubkey,
    ipfs_cid: [u8; 64],
    lat: i64,
    lon: i64,
    alt_m: u32,
    delivered_at: i64,
) -> SolanaInstruction {
    let data = resq_delivery::instruction::RecordDelivery {
        ipfs_cid,
        lat,
        lon,
        alt_m,
        delivered_at,
    }
    .data();

    let accounts = resq_delivery::accounts::RecordDelivery {
        drone: anchor_pubkey(*drone),
        airspace: anchor_pubkey(*airspace),
        delivery_record: anchor_pubkey(*record_pda),
        system_program: anchor_lang::system_program::ID,
    }
    .to_account_metas(None);

    SolanaInstruction {
        program_id: sdk_pubkey(resq_delivery::id()),
        accounts: sdk_account_metas(accounts),
        data,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_record_delivery_happy_path() {
    let mut program = ProgramTest::new(
        "resq_delivery",
        sdk_pubkey(resq_delivery::id()),
        processor!(process_instruction),
    );
    let airspace_pubkey = seed_airspace_account(&mut program);
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let drone = Keypair::new();
    let clock: Clock = banks_client.get_sysvar().await.unwrap();
    let delivered_at = clock.unix_timestamp - 30;

    let (record_pda, _) = delivery_pda(&drone.pubkey(), delivered_at);
    let cid_str = "QmResQTestCID1234567890abcdefghijklmnopqrstuvwxyz";

    let ix = create_record_ix(
        &drone.pubkey(),
        &airspace_pubkey,
        &record_pda,
        cid_to_bytes(cid_str),
        407128000,
        -740060000,
        50,
        delivered_at,
    );

    let fund_ix = system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 10_000_000);
    let mut tx = SolanaTransaction::new_with_payer(&[fund_ix, ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &drone], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify account state
    let account = banks_client.get_account(record_pda).await.unwrap().unwrap();
    let record: DeliveryRecord = DeliveryRecord::try_deserialize(&mut account.data.as_slice()).unwrap();

    assert_eq!(sdk_pubkey(record.drone_pda), drone.pubkey());
    assert_eq!(sdk_pubkey(record.airspace_pda), airspace_pubkey);
    assert_eq!(record.lat, 407128000);
    assert_eq!(record.lon, -740060000);
    assert_eq!(record.alt_m, 50);
    assert_eq!(record.delivered_at, delivered_at);

    let stored_cid_str = std::str::from_utf8(&record.ipfs_cid).unwrap().trim_matches(char::from(0));
    assert!(stored_cid_str.contains("QmResQTestCID"));
}

#[tokio::test]
async fn test_rejects_all_zero_cid() {
    let mut program = ProgramTest::new(
        "resq_delivery",
        sdk_pubkey(resq_delivery::id()),
        processor!(process_instruction),
    );
    let airspace_pubkey = seed_airspace_account(&mut program);
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let drone = Keypair::new();
    let clock: Clock = banks_client.get_sysvar().await.unwrap();
    let delivered_at = clock.unix_timestamp - 30;
    let (record_pda, _) = delivery_pda(&drone.pubkey(), delivered_at);

    let ix = create_record_ix(
        &drone.pubkey(),
        &airspace_pubkey,
        &record_pda,
        [0u8; 64],
        0,
        0,
        0,
        delivered_at,
    );

    let fund_ix = system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 10_000_000);
    let mut tx = SolanaTransaction::new_with_payer(&[fund_ix, ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &drone], recent_blockhash);

    let err = banks_client.process_transaction(tx).await.unwrap_err();
    assert!(err.unwrap().to_string().contains("EmptyCid") || format!("{:?}", err).contains("Custom(6000)"));
}

#[tokio::test]
async fn test_rejects_zero_timestamp() {
    let mut program = ProgramTest::new(
        "resq_delivery",
        sdk_pubkey(resq_delivery::id()),
        processor!(process_instruction),
    );
    let airspace_pubkey = seed_airspace_account(&mut program);
    let (banks_client, payer, recent_blockhash) = program.start().await;

    let drone = Keypair::new();
    let delivered_at = 0_i64;
    let (record_pda, _) = delivery_pda(&drone.pubkey(), delivered_at);

    let ix = create_record_ix(
        &drone.pubkey(),
        &airspace_pubkey,
        &record_pda,
        cid_to_bytes("QmValidCID"),
        0,
        0,
        0,
        delivered_at,
    );

    let fund_ix = system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 10_000_000);
    let mut tx = SolanaTransaction::new_with_payer(&[fund_ix, ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &drone], recent_blockhash);

    let err = banks_client.process_transaction(tx).await.unwrap_err();
    assert!(err.unwrap().to_string().contains("InvalidTimestamp") || format!("{:?}", err).contains("Custom(6001)"));
}

#[tokio::test]
async fn test_rejects_latitude_out_of_range() {
    let mut program = ProgramTest::new(
        "resq_delivery",
        sdk_pubkey(resq_delivery::id()),
        processor!(process_instruction),
    );
    let airspace_pubkey = seed_airspace_account(&mut program);
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let drone = Keypair::new();
    let clock: Clock = banks_client.get_sysvar().await.unwrap();
    let delivered_at = clock.unix_timestamp - 30;
    let (record_pda, _) = delivery_pda(&drone.pubkey(), delivered_at);

    let ix = create_record_ix(
        &drone.pubkey(),
        &airspace_pubkey,
        &record_pda,
        cid_to_bytes("QmValidCID"),
        900_000_001,
        0,
        0,
        delivered_at,
    );

    let fund_ix = system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 10_000_000);
    let mut tx = SolanaTransaction::new_with_payer(&[fund_ix, ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &drone], recent_blockhash);

    let err = banks_client.process_transaction(tx).await.unwrap_err();
    assert!(err.unwrap().to_string().contains("LatitudeOutOfRange") || format!("{:?}", err).contains("Custom(6003)"));
}

#[tokio::test]
async fn test_rejects_longitude_out_of_range() {
    let mut program = ProgramTest::new(
        "resq_delivery",
        sdk_pubkey(resq_delivery::id()),
        processor!(process_instruction),
    );
    let airspace_pubkey = seed_airspace_account(&mut program);
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let drone = Keypair::new();
    let clock: Clock = banks_client.get_sysvar().await.unwrap();
    let delivered_at = clock.unix_timestamp - 30;
    let (record_pda, _) = delivery_pda(&drone.pubkey(), delivered_at);

    let ix = create_record_ix(
        &drone.pubkey(),
        &airspace_pubkey,
        &record_pda,
        cid_to_bytes("QmValidCID"),
        0,
        -1_800_000_001,
        0,
        delivered_at,
    );

    let fund_ix = system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 10_000_000);
    let mut tx = SolanaTransaction::new_with_payer(&[fund_ix, ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &drone], recent_blockhash);

    let err = banks_client.process_transaction(tx).await.unwrap_err();
    assert!(err.unwrap().to_string().contains("LongitudeOutOfRange") || format!("{:?}", err).contains("Custom(6004)"));
}

#[tokio::test]
async fn test_duplicate_delivery_fails() {
    let mut program = ProgramTest::new(
        "resq_delivery",
        sdk_pubkey(resq_delivery::id()),
        processor!(process_instruction),
    );
    let airspace_pubkey = seed_airspace_account(&mut program);
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let drone = Keypair::new();
    let clock: Clock = banks_client.get_sysvar().await.unwrap();
    let delivered_at = clock.unix_timestamp - 30;
    let (record_pda, _) = delivery_pda(&drone.pubkey(), delivered_at);

    let ix1 = create_record_ix(
        &drone.pubkey(),
        &airspace_pubkey,
        &record_pda,
        cid_to_bytes("QmFirst"),
        0,
        0,
        0,
        delivered_at,
    );

    let fund_ix = system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 10_000_000);
    let mut tx1 = SolanaTransaction::new_with_payer(&[fund_ix, ix1], Some(&payer.pubkey()));
    tx1.sign(&[&payer, &drone], recent_blockhash);
    banks_client.process_transaction(tx1).await.unwrap();

    let ix2 = create_record_ix(
        &drone.pubkey(),
        &airspace_pubkey,
        &record_pda,
        cid_to_bytes("QmSecond"),
        0,
        0,
        0,
        delivered_at,
    );

    // Will fail because account is already initialized
    let mut tx2 = SolanaTransaction::new_with_payer(&[ix2], Some(&payer.pubkey()));
    // Force a new blockhash to avoid duplicate transaction signature error,
    // we want the instruction itself to fail.
    let recent_blockhash2 = banks_client.get_latest_blockhash().await.unwrap();
    tx2.sign(&[&payer, &drone], recent_blockhash2);

    let err = banks_client.process_transaction(tx2).await.unwrap_err();
    // Anchor initialization failure
    assert!(format!("{:?}", err).contains("already in use") || format!("{:?}", err).contains("InstructionError"));
}

#[tokio::test]
async fn test_rejects_airspace_not_owned_by_airspace_program() {
    let mut program = ProgramTest::new(
        "resq_delivery",
        sdk_pubkey(resq_delivery::id()),
        processor!(process_instruction),
    );
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let drone = Keypair::new();
    // This pubkey has no account, so it defaults to system-program-owned —
    // the owner constraint must reject it.
    let bogus_airspace = SolanaPubkey::new_unique();
    let clock: Clock = banks_client.get_sysvar().await.unwrap();
    let delivered_at = clock.unix_timestamp - 30;
    let (record_pda, _) = delivery_pda(&drone.pubkey(), delivered_at);

    let ix = create_record_ix(
        &drone.pubkey(),
        &bogus_airspace,
        &record_pda,
        cid_to_bytes("QmValidCID"),
        407128000,
        -740060000,
        50,
        delivered_at,
    );

    let fund_ix = system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 10_000_000);
    let mut tx = SolanaTransaction::new_with_payer(&[fund_ix, ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &drone], recent_blockhash);

    let err = banks_client.process_transaction(tx).await.unwrap_err();
    assert!(
        err.unwrap().to_string().contains("InvalidAirspace")
            || format!("{:?}", err).contains("Custom(6005)")
            || format!("{:?}", err).contains("IllegalOwner")
            || format!("{:?}", err).contains("ConstraintOwner")
            || format!("{:?}", err).contains("AccountNotInitialized")
            || format!("{:?}", err).contains("Custom(3012)")
    );
}
