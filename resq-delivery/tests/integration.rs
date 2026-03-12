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
    system_program,
    transaction::Transaction,
};

use resq_delivery::state::delivery_record::DeliveryRecord;

fn process_instruction(
    program_id: &solana_sdk::pubkey::Pubkey,
    accounts: &[solana_sdk::account_info::AccountInfo],
    data: &[u8],
) -> solana_sdk::entrypoint::ProgramResult {
    resq_delivery::entry(program_id, unsafe { std::mem::transmute(accounts) }, data)
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn cid_to_bytes(cid: &str) -> [u8; 64] {
    let mut buf = [0u8; 64];
    let bytes = cid.as_bytes();
    let len = bytes.len().min(64);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf
}

fn delivery_pda(drone: &Pubkey, delivered_at: i64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"delivery", drone.as_ref(), &delivered_at.to_le_bytes()],
        &resq_delivery::id(),
    )
}

fn create_record_ix(
    drone: &Pubkey,
    airspace: &Pubkey,
    record_pda: &Pubkey,
    ipfs_cid: [u8; 64],
    lat: i64,
    lon: i64,
    alt_m: u32,
    delivered_at: i64,
) -> Instruction {
    let data = resq_delivery::instruction::RecordDelivery {
        ipfs_cid,
        lat,
        lon,
        alt_m,
        delivered_at,
    }
    .data();

    let accounts = resq_delivery::accounts::RecordDelivery {
        drone: *drone,
        airspace: *airspace,
        delivery_record: *record_pda,
        system_program: system_program::id(),
    }
    .to_account_metas(None);

    Instruction {
        program_id: resq_delivery::id(),
        accounts,
        data,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_record_delivery_happy_path() {
    let program = ProgramTest::new(
        "resq_delivery",
        resq_delivery::id(),
        processor!(process_instruction),
    );
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let drone = Keypair::new();
    let airspace_pubkey = Pubkey::new_unique();
    let delivered_at = 1680000000_i64;

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

    let fund_ix = solana_sdk::system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 10_000_000);
    let mut tx = Transaction::new_with_payer(&[fund_ix, ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &drone], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // Verify account state
    let account = banks_client.get_account(record_pda).await.unwrap().unwrap();
    let record: DeliveryRecord = DeliveryRecord::try_deserialize(&mut account.data.as_slice()).unwrap();

    assert_eq!(record.drone_pda, drone.pubkey());
    assert_eq!(record.airspace_pda, airspace_pubkey);
    assert_eq!(record.lat, 407128000);
    assert_eq!(record.lon, -740060000);
    assert_eq!(record.alt_m, 50);
    assert_eq!(record.delivered_at, delivered_at);

    let stored_cid_str = std::str::from_utf8(&record.ipfs_cid).unwrap().trim_matches(char::from(0));
    assert!(stored_cid_str.contains("QmResQTestCID"));
}

#[tokio::test]
async fn test_rejects_all_zero_cid() {
    let program = ProgramTest::new(
        "resq_delivery",
        resq_delivery::id(),
        processor!(process_instruction),
    );
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let drone = Keypair::new();
    let airspace_pubkey = Pubkey::new_unique();
    let delivered_at = 1680000001_i64;
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

    let fund_ix = solana_sdk::system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 10_000_000);
    let mut tx = Transaction::new_with_payer(&[fund_ix, ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &drone], recent_blockhash);

    let err = banks_client.process_transaction(tx).await.unwrap_err();
    assert!(err.unwrap().to_string().contains("EmptyCid") || format!("{:?}", err).contains("Custom(6000)"));
}

#[tokio::test]
async fn test_rejects_zero_timestamp() {
    let program = ProgramTest::new(
        "resq_delivery",
        resq_delivery::id(),
        processor!(process_instruction),
    );
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let drone = Keypair::new();
    let airspace_pubkey = Pubkey::new_unique();
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

    let fund_ix = solana_sdk::system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 10_000_000);
    let mut tx = Transaction::new_with_payer(&[fund_ix, ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &drone], recent_blockhash);

    let err = banks_client.process_transaction(tx).await.unwrap_err();
    assert!(err.unwrap().to_string().contains("InvalidTimestamp") || format!("{:?}", err).contains("Custom(6001)"));
}

#[tokio::test]
async fn test_rejects_latitude_out_of_range() {
    let program = ProgramTest::new(
        "resq_delivery",
        resq_delivery::id(),
        processor!(process_instruction),
    );
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let drone = Keypair::new();
    let airspace_pubkey = Pubkey::new_unique();
    let delivered_at = 1680000002_i64;
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

    let fund_ix = solana_sdk::system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 10_000_000);
    let mut tx = Transaction::new_with_payer(&[fund_ix, ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &drone], recent_blockhash);

    let err = banks_client.process_transaction(tx).await.unwrap_err();
    assert!(err.unwrap().to_string().contains("LatitudeOutOfRange") || format!("{:?}", err).contains("Custom(6003)"));
}

#[tokio::test]
async fn test_rejects_longitude_out_of_range() {
    let program = ProgramTest::new(
        "resq_delivery",
        resq_delivery::id(),
        processor!(process_instruction),
    );
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let drone = Keypair::new();
    let airspace_pubkey = Pubkey::new_unique();
    let delivered_at = 1680000003_i64;
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

    let fund_ix = solana_sdk::system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 10_000_000);
    let mut tx = Transaction::new_with_payer(&[fund_ix, ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &drone], recent_blockhash);

    let err = banks_client.process_transaction(tx).await.unwrap_err();
    assert!(err.unwrap().to_string().contains("LongitudeOutOfRange") || format!("{:?}", err).contains("Custom(6004)"));
}

#[tokio::test]
async fn test_duplicate_delivery_fails() {
    let program = ProgramTest::new(
        "resq_delivery",
        resq_delivery::id(),
        processor!(process_instruction),
    );
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let drone = Keypair::new();
    let airspace_pubkey = Pubkey::new_unique();
    let delivered_at = 1680000004_i64;
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

    let fund_ix = solana_sdk::system_instruction::transfer(&payer.pubkey(), &drone.pubkey(), 10_000_000);
    let mut tx1 = Transaction::new_with_payer(&[fund_ix, ix1], Some(&payer.pubkey()));
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
    let mut tx2 = Transaction::new_with_payer(&[ix2], Some(&payer.pubkey()));
    // Force a new blockhash to avoid duplicate transaction signature error,
    // we want the instruction itself to fail.
    let recent_blockhash2 = banks_client.get_latest_blockhash().await.unwrap();
    tx2.sign(&[&payer, &drone], recent_blockhash2);

    let err = banks_client.process_transaction(tx2).await.unwrap_err();
    // Anchor initialization failure
    assert!(format!("{:?}", err).contains("already in use") || format!("{:?}", err).contains("InstructionError"));
}
