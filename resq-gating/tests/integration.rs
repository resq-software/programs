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

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_account_info::AccountInfo;
use solana_instruction::{AccountMeta, Instruction as SolanaInstruction};
use solana_program_entrypoint::ProgramResult;
use solana_program_test::{processor, ProgramTest};
use solana_pubkey::Pubkey as SolanaPubkey;
use solana_sdk::program_error::ProgramError;
use solana_sdk::{
    signature::Keypair, signer::Signer, sysvar::clock::Clock, transaction::Transaction,
};

use resq_gating::{AirspacePermit, LocationAttestation};

#[allow(unsafe_code)]
fn process_instruction(
    program_id: &SolanaPubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let program_id = anchor_pubkey(*program_id);
    resq_gating::entry(&program_id, unsafe { std::mem::transmute(accounts) }, data)
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
    value
        .into_iter()
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

fn permit_pda(permit_id: &[u8; 32]) -> (SolanaPubkey, u8) {
    SolanaPubkey::find_program_address(
        &[b"airspace_permit", permit_id],
        &sdk_pubkey(resq_gating::id()),
    )
}

fn attestation_pda(permit: &SolanaPubkey, waypoint_index: u16) -> (SolanaPubkey, u8) {
    SolanaPubkey::find_program_address(
        &[
            b"location_attestation",
            permit.as_ref(),
            &waypoint_index.to_le_bytes(),
        ],
        &sdk_pubkey(resq_gating::id()),
    )
}

/// Packs the telemetry payload exactly as `services/edge-aeai/include/tpm_signing.hpp`'s
/// `TelemetryPayload` struct does, and as `submit_attestation` reconstructs it on-chain:
/// permit_id[32] || waypoint_index (u16 LE) || lat (f64 LE) || lon (f64 LE) ||
/// alt (f32 LE) || timestamp (i64 LE) = 62 bytes.
fn pack_telemetry_payload(
    permit_id: &[u8; 32],
    waypoint_index: u16,
    latitude: f64,
    longitude: f64,
    altitude: f32,
    timestamp: i64,
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(62);
    msg.extend_from_slice(permit_id);
    msg.extend_from_slice(&waypoint_index.to_le_bytes());
    msg.extend_from_slice(&latitude.to_le_bytes());
    msg.extend_from_slice(&longitude.to_le_bytes());
    msg.extend_from_slice(&altitude.to_le_bytes());
    msg.extend_from_slice(&timestamp.to_le_bytes());
    msg
}

/// Builds a native Ed25519 program precompile instruction using the exact
/// single-signature header layout that `submit_attestation` parses:
/// `[num_sigs=1][pad][sig_offset u16][sig_ix=0xFFFF][pubkey_offset u16]
/// [pubkey_ix=0xFFFF][msg_offset u16][msg_size u16][msg_ix=0xFFFF]`
/// followed by the pubkey (32B), signature (64B), and message bytes.
fn build_ed25519_ix(pubkey: [u8; 32], signature: [u8; 64], message: &[u8]) -> SolanaInstruction {
    const PUBKEY_OFFSET: u16 = 16;
    const SIG_OFFSET: u16 = 48;
    const MSG_OFFSET: u16 = 112;
    const NO_INDEX: u16 = 0xFFFF;

    let mut data = Vec::with_capacity(112 + message.len());
    data.push(1u8); // num_signatures
    data.push(0u8); // padding
    data.extend_from_slice(&SIG_OFFSET.to_le_bytes());
    data.extend_from_slice(&NO_INDEX.to_le_bytes());
    data.extend_from_slice(&PUBKEY_OFFSET.to_le_bytes());
    data.extend_from_slice(&NO_INDEX.to_le_bytes());
    data.extend_from_slice(&MSG_OFFSET.to_le_bytes());
    data.extend_from_slice(&(message.len() as u16).to_le_bytes());
    data.extend_from_slice(&NO_INDEX.to_le_bytes());
    data.extend_from_slice(&pubkey);
    data.extend_from_slice(&signature);
    data.extend_from_slice(message);

    SolanaInstruction {
        program_id: sdk_pubkey(solana_program::ed25519_program::ID),
        accounts: vec![],
        data,
    }
}

/// Builds a *malformed* Ed25519 precompile instruction: the header declares
/// the standard offsets/sizes (as `submit_attestation`'s `require_eq!` checks
/// expect), but the instruction data buffer is truncated to exactly 112 bytes
/// -- i.e. it stops right where the message region would begin, omitting the
/// message payload entirely. Prior to the OOB-slice fix, a downstream program
/// that only checked `data.len() >= 112` would then slice `data[112..174]`
/// out of bounds.
fn build_truncated_ed25519_ix() -> SolanaInstruction {
    const PUBKEY_OFFSET: u16 = 16;
    const SIG_OFFSET: u16 = 48;
    const MSG_OFFSET: u16 = 112;
    const MSG_SIZE: u16 = 62;
    const NO_INDEX: u16 = 0xFFFF;

    let mut data = Vec::with_capacity(112);
    data.push(1u8);
    data.push(0u8);
    data.extend_from_slice(&SIG_OFFSET.to_le_bytes());
    data.extend_from_slice(&NO_INDEX.to_le_bytes());
    data.extend_from_slice(&PUBKEY_OFFSET.to_le_bytes());
    data.extend_from_slice(&NO_INDEX.to_le_bytes());
    data.extend_from_slice(&MSG_OFFSET.to_le_bytes());
    data.extend_from_slice(&MSG_SIZE.to_le_bytes());
    data.extend_from_slice(&NO_INDEX.to_le_bytes());
    data.extend_from_slice(&[0u8; 32]); // fake pubkey region (16..48)
    data.extend_from_slice(&[0u8; 64]); // fake signature region (48..112)
                                        // NOTE: no message bytes appended -- data.len() == 112, but the header
                                        // claims a message region of 112..174.
    assert_eq!(data.len(), 112);

    SolanaInstruction {
        program_id: sdk_pubkey(solana_program::ed25519_program::ID),
        accounts: vec![],
        data,
    }
}

fn init_permit_ix(
    operator: &SolanaPubkey,
    permit: &SolanaPubkey,
    permit_id: [u8; 32],
    drone: SolanaPubkey,
    route_root: [u8; 32],
) -> SolanaInstruction {
    let data = resq_gating::instruction::InitializePermit {
        permit_id,
        drone_pubkey: anchor_pubkey(drone),
        route_root,
    }
    .data();

    let accounts = resq_gating::accounts::InitializePermit {
        permit: anchor_pubkey(*permit),
        operator: anchor_pubkey(*operator),
        system_program: anchor_lang::system_program::ID,
    }
    .to_account_metas(None);

    SolanaInstruction {
        program_id: sdk_pubkey(resq_gating::id()),
        accounts: sdk_account_metas(accounts),
        data,
    }
}

#[allow(clippy::too_many_arguments)]
fn submit_attestation_ix(
    permit: &SolanaPubkey,
    attestation: &SolanaPubkey,
    payer: &SolanaPubkey,
    latitude: f64,
    longitude: f64,
    altitude: f32,
    timestamp: i64,
    signature: [u8; 64],
) -> SolanaInstruction {
    let data = resq_gating::instruction::SubmitAttestation {
        latitude,
        longitude,
        altitude,
        timestamp,
        signature,
    }
    .data();

    let accounts = resq_gating::accounts::SubmitAttestation {
        permit: anchor_pubkey(*permit),
        attestation: anchor_pubkey(*attestation),
        payer: anchor_pubkey(*payer),
        instructions_sysvar: solana_instructions_sysvar::ID,
        system_program: anchor_lang::system_program::ID,
    }
    .to_account_metas(None);

    SolanaInstruction {
        program_id: sdk_pubkey(resq_gating::id()),
        accounts: sdk_account_metas(accounts),
        data,
    }
}

fn new_program_test() -> ProgramTest {
    ProgramTest::new(
        "resq_gating",
        sdk_pubkey(resq_gating::id()),
        processor!(process_instruction),
    )
}

// ─── Tests ───────────────────────────────────────────────────────────────────

/// (a) A correctly-signed attestation with a matching permit_id succeeds
/// end-to-end: initialize_permit -> submit_attestation. This is the
/// regression the two CRITICAL bugs (missing permit_id field / mismatched
/// PDA seeds / wrong message reconstruction) broke -- previously this could
/// never succeed because the SubmitAttestation account's PDA seeds could
/// never resolve to the permit created by initialize_permit.
#[tokio::test]
async fn test_submit_attestation_happy_path() {
    let program = new_program_test();
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let operator = payer.pubkey();
    let permit_id = [7u8; 32];
    let drone = Keypair::new();
    let route_root = [9u8; 32];

    let (permit, _) = permit_pda(&permit_id);
    let init_ix = init_permit_ix(&operator, &permit, permit_id, drone.pubkey(), route_root);
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let waypoint_index: u16 = 0;
    let (attestation, _) = attestation_pda(&permit, waypoint_index);

    let clock: Clock = banks_client.get_sysvar().await.unwrap();
    let timestamp = clock.unix_timestamp;
    let latitude = 37.7749_f64;
    let longitude = -122.4194_f64;
    let altitude = 120.5_f32;

    let message = pack_telemetry_payload(
        &permit_id,
        waypoint_index,
        latitude,
        longitude,
        altitude,
        timestamp,
    );
    let signature: [u8; 64] = drone.sign_message(&message).into();

    let ed25519_ix = build_ed25519_ix(drone.pubkey().to_bytes(), signature, &message);
    let submit_ix = submit_attestation_ix(
        &permit,
        &attestation,
        &payer.pubkey(),
        latitude,
        longitude,
        altitude,
        timestamp,
        signature,
    );

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[ed25519_ix, submit_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let attestation_account = banks_client
        .get_account(attestation)
        .await
        .unwrap()
        .unwrap();
    let record: LocationAttestation =
        LocationAttestation::try_deserialize(&mut attestation_account.data.as_slice()).unwrap();
    assert_eq!(record.permit, anchor_pubkey(permit));
    assert_eq!(record.waypoint_index, 0);
    assert_eq!(record.latitude, latitude);
    assert_eq!(record.longitude, longitude);
    assert_eq!(record.altitude, altitude);
    assert_eq!(record.timestamp, timestamp);
    assert_eq!(record.signature, signature);

    let permit_account = banks_client.get_account(permit).await.unwrap().unwrap();
    let permit_state: AirspacePermit =
        AirspacePermit::try_deserialize(&mut permit_account.data.as_slice()).unwrap();
    assert_eq!(permit_state.permit_id, permit_id);
    assert_eq!(permit_state.current_waypoint_index, 1);
}

/// (b) A signature over a *different* permit_id than the one stored on-chain
/// (simulating a replayed/misattributed signature, or tampered telemetry)
/// must be rejected with `TelemetryPayloadSpoofed`.
#[tokio::test]
async fn test_wrong_permit_id_is_rejected() {
    let program = new_program_test();
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let operator = payer.pubkey();
    let permit_id = [1u8; 32];
    let wrong_permit_id = [2u8; 32];
    let drone = Keypair::new();
    let route_root = [3u8; 32];

    let (permit, _) = permit_pda(&permit_id);
    let init_ix = init_permit_ix(&operator, &permit, permit_id, drone.pubkey(), route_root);
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let waypoint_index: u16 = 0;
    let (attestation, _) = attestation_pda(&permit, waypoint_index);

    let clock: Clock = banks_client.get_sysvar().await.unwrap();
    let timestamp = clock.unix_timestamp;
    let latitude = 1.0_f64;
    let longitude = 2.0_f64;
    let altitude = 3.0_f32;

    // Signed message is bound to the WRONG permit_id.
    let tampered_message = pack_telemetry_payload(
        &wrong_permit_id,
        waypoint_index,
        latitude,
        longitude,
        altitude,
        timestamp,
    );
    let signature: [u8; 64] = drone.sign_message(&tampered_message).into();

    let ed25519_ix = build_ed25519_ix(drone.pubkey().to_bytes(), signature, &tampered_message);
    let submit_ix = submit_attestation_ix(
        &permit,
        &attestation,
        &payer.pubkey(),
        latitude,
        longitude,
        altitude,
        timestamp,
        signature,
    );

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[ed25519_ix, submit_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    let err = banks_client.process_transaction(tx).await.unwrap_err();
    assert!(
        err.to_string().contains("TelemetryPayloadSpoofed")
            || format!("{err:?}").contains("Custom(6013)"),
        "unexpected error: {err:?}"
    );
}

/// (c) A timestamp outside the +/-30s freshness window must be rejected with
/// `StaleAttestation`, even before the Ed25519 precompile is consulted.
#[tokio::test]
async fn test_stale_timestamp_is_rejected() {
    let program = new_program_test();
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let operator = payer.pubkey();
    let permit_id = [4u8; 32];
    let drone = Keypair::new();
    let route_root = [5u8; 32];

    let (permit, _) = permit_pda(&permit_id);
    let init_ix = init_permit_ix(&operator, &permit, permit_id, drone.pubkey(), route_root);
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let waypoint_index: u16 = 0;
    let (attestation, _) = attestation_pda(&permit, waypoint_index);

    let clock: Clock = banks_client.get_sysvar().await.unwrap();
    // One hour in the past -- well outside the 30s window.
    let timestamp = clock.unix_timestamp - 3600;
    let latitude = 1.0_f64;
    let longitude = 2.0_f64;
    let altitude = 3.0_f32;

    let message = pack_telemetry_payload(
        &permit_id,
        waypoint_index,
        latitude,
        longitude,
        altitude,
        timestamp,
    );
    let signature: [u8; 64] = drone.sign_message(&message).into();

    // Note: no preceding Ed25519 precompile instruction is even required to
    // exercise this check -- the freshness check runs before precompile
    // parsing, so a single-instruction transaction is sufficient.
    let submit_ix = submit_attestation_ix(
        &permit,
        &attestation,
        &payer.pubkey(),
        latitude,
        longitude,
        altitude,
        timestamp,
        signature,
    );

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(&[submit_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    let err = banks_client.process_transaction(tx).await.unwrap_err();
    assert!(
        err.to_string().contains("StaleAttestation") || format!("{err:?}").contains("Custom(6001)"),
        "unexpected error: {err:?}"
    );
}

/// (d) A malformed, truncated (112-byte) Ed25519 precompile instruction --
/// crafted so its header advertises the standard message offset/size
/// (112/62) while the actual data buffer ends exactly at byte 112 -- must be
/// rejected cleanly (a typed error / clean transaction failure), never a
/// panic/process abort, when paired with a `submit_attestation` call.
#[tokio::test]
async fn test_truncated_precompile_does_not_panic() {
    let program = new_program_test();
    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let operator = payer.pubkey();
    let permit_id = [6u8; 32];
    let drone = Keypair::new();
    let route_root = [8u8; 32];

    let (permit, _) = permit_pda(&permit_id);
    let init_ix = init_permit_ix(&operator, &permit, permit_id, drone.pubkey(), route_root);
    let mut tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let waypoint_index: u16 = 0;
    let (attestation, _) = attestation_pda(&permit, waypoint_index);

    let clock: Clock = banks_client.get_sysvar().await.unwrap();
    let timestamp = clock.unix_timestamp;

    let malformed_ed25519_ix = build_truncated_ed25519_ix();
    let submit_ix = submit_attestation_ix(
        &permit,
        &attestation,
        &payer.pubkey(),
        1.0,
        2.0,
        3.0,
        timestamp,
        [0u8; 64],
    );

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx =
        Transaction::new_with_payer(&[malformed_ed25519_ix, submit_ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    // The key assertion: this call returns cleanly (Ok with a program-level
    // error, or a sanitize/precompile-level Err) -- it must never panic or
    // abort the process, which is what the pre-fix `data[112..174]` slice on
    // a 112-byte buffer would have done.
    let result = banks_client.process_transaction(tx).await;
    assert!(
        result.is_err(),
        "malformed precompile instruction should not succeed"
    );
}
