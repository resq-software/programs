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
use solana_instructions_sysvar::{load_current_index_checked, load_instruction_at_checked};
use solana_program::ed25519_program;

declare_id!("GaT1ngA1rSpAcE111111111111111111111111111111");

/// Maximum allowed drift (in seconds) between the on-chain clock and the
/// attestation's client-supplied timestamp. Bounds the replay window for a
/// captured-and-resubmitted telemetry attestation.
const MAX_ATTESTATION_CLOCK_DRIFT_SECS: i64 = 30;

/// Byte offset of the signed message within the Ed25519 precompile instruction data.
const PRECOMPILE_MSG_OFFSET: usize = 112;
/// Size (in bytes) of the packed `TelemetryPayload` signed by the drone's TPM:
/// permit_id[32] + waypoint_index(u16) + lat(f64) + lon(f64) + alt(f32) + timestamp(i64).
const PRECOMPILE_MSG_SIZE: usize = 32 + 2 + 8 + 8 + 4 + 8;

/// Returns `true` when the client-supplied attestation `timestamp` is within
/// [`MAX_ATTESTATION_CLOCK_DRIFT_SECS`] of the on-chain clock `now`.
///
/// Uses [`i64::abs_diff`] (which yields a `u64`) so a hostile caller cannot
/// trigger a signed-integer overflow — e.g. `timestamp == i64::MIN` against a
/// positive on-chain clock — that would otherwise panic the program under the
/// workspace's overflow-checked release profile. This check runs before the
/// Ed25519 precompile is consulted, so it must never panic on attacker-supplied
/// input.
fn is_within_clock_drift(now: i64, timestamp: i64) -> bool {
    now.abs_diff(timestamp) <= MAX_ATTESTATION_CLOCK_DRIFT_SECS as u64
}

/// Computes the permit's waypoint state transition after an attestation at the
/// `current` index has been recorded. Returns `(next_index, still_active)`.
///
/// Uses [`u16::checked_add`] so a long-lived permit that reaches
/// [`u16::MAX`] attestations cannot overflow the counter and abort the
/// instruction under the workspace's overflow-checked release profile. When the
/// `u16` waypoint sequence space is exhausted the permit is retired
/// (`still_active == false`) rather than wrapping or panicking, so no further
/// attestations can be submitted against it while the final attestation at index
/// `u16::MAX` is still recorded cleanly.
fn advance_waypoint(current: u16) -> (u16, bool) {
    match current.checked_add(1) {
        Some(next) => (next, true),
        None => (current, false),
    }
}

/// Maximum absolute latitude (degrees) accepted for an attestation.
const MAX_ABS_LATITUDE_DEG: f64 = 90.0;
/// Maximum absolute longitude (degrees) accepted for an attestation.
const MAX_ABS_LONGITUDE_DEG: f64 = 180.0;

/// Returns `true` when `latitude`/`longitude` are physically plausible WGS-84
/// coordinates: both must be finite (rejecting `NaN` and `±inf`) and within the
/// valid geographic ranges (`|lat| <= 90`, `|lon| <= 180`).
///
/// A freshly *signed* telemetry payload can still carry malformed coordinates —
/// a registered drone with a sensor/serialization bug, or a compromised drone
/// key, can sign impossible positions (latitude 500, infinite altitude). Without
/// this guard those values are recorded verbatim on-chain and permanently
/// advance the permit's waypoint sequence. `f64::is_finite` is available on the
/// Solana BPF/SBF target, so the check runs on-chain. Kept as a standalone pure
/// function so the range logic is unit-testable without the Anchor runtime.
fn are_coordinates_valid(latitude: f64, longitude: f64) -> bool {
    latitude.is_finite()
        && longitude.is_finite()
        && latitude.abs() <= MAX_ABS_LATITUDE_DEG
        && longitude.abs() <= MAX_ABS_LONGITUDE_DEG
}

#[program]
pub mod resq_gating {
    use super::*;

    pub fn initialize_permit(
        ctx: Context<InitializePermit>,
        permit_id: [u8; 32],
        drone_pubkey: Pubkey,
        route_root: [u8; 32],
    ) -> Result<()> {
        let permit = &mut ctx.accounts.permit;
        permit.operator = ctx.accounts.operator.key();
        permit.permit_id = permit_id;
        permit.drone = drone_pubkey;
        permit.route_root = route_root;
        permit.current_waypoint_index = 0;
        permit.is_active = true;
        permit.bump = ctx.bumps.permit;
        Ok(())
    }

    pub fn submit_attestation(
        ctx: Context<SubmitAttestation>,
        latitude: f64,
        longitude: f64,
        altitude: f32,
        timestamp: i64,
        signature: [u8; 64],
    ) -> Result<()> {
        let permit = &mut ctx.accounts.permit;
        require!(permit.is_active, ResQError::InactivePermit);

        // Reject malformed telemetry before it can be recorded on-chain. A valid
        // Ed25519 signature only proves the drone *signed* these coordinates — it
        // does not prove they are physically plausible. A sensor/serialization
        // bug or a compromised (but still registered) drone key can sign
        // non-finite (NaN/±inf) or out-of-range positions (e.g. latitude 500);
        // without this guard they would be persisted verbatim and permanently
        // advance the permit's waypoint sequence. Checked here, at the boundary,
        // before any attestation is written or the waypoint advanced.
        require!(
            are_coordinates_valid(latitude, longitude),
            ResQError::InvalidCoordinates
        );

        // Reject stale or replayed attestations: the client-supplied timestamp
        // must be within a tight window of the on-chain clock.
        let clock = Clock::get()?;
        require!(
            is_within_clock_drift(clock.unix_timestamp, timestamp),
            ResQError::StaleAttestation
        );

        let sysvar_info = &ctx.accounts.instructions_sysvar;

        // 1. Determine the index of the currently executing instruction
        let current_index = load_current_index_checked(sysvar_info)? as usize;
        require!(current_index > 0, ResQError::MissingSignaturePrecompile);

        // 2. Load the preceding instruction using a relative offset to prevent index manipulation
        let precompile_index = current_index - 1;
        let precompile_ix = load_instruction_at_checked(precompile_index, sysvar_info)?;

        // 3. Assert that the precompile targeted the native Ed25519 program
        require_keys_eq!(
            precompile_ix.program_id,
            ed25519_program::ID,
            ResQError::InvalidPrecompileProgram
        );

        // 4. Extract and validate precompile parameters to mitigate offset exploits
        let data = &precompile_ix.data;
        require!(data.len() >= 16, ResQError::MalformedPrecompileHeader);

        let num_signatures = data[0];
        require_eq!(num_signatures, 1, ResQError::InvalidSignatureCount);

        // Parse offsets using little-endian representation
        let sig_offset = u16::from_le_bytes([data[2], data[3]]) as usize;
        let sig_ix = u16::from_le_bytes([data[4], data[5]]);
        let pubkey_offset = u16::from_le_bytes([data[6], data[7]]) as usize;
        let pubkey_ix = u16::from_le_bytes([data[8], data[9]]);
        let msg_offset = u16::from_le_bytes([data[10], data[11]]) as usize;
        let msg_size = u16::from_le_bytes([data[12], data[13]]) as usize;
        let msg_ix = u16::from_le_bytes([data[14], data[15]]);

        // Enforce that verification data is contained within the precompile instruction body
        require_eq!(sig_ix, 0xFFFF, ResQError::CrossInstructionOffsetsProhibited);
        require_eq!(
            pubkey_ix,
            0xFFFF,
            ResQError::CrossInstructionOffsetsProhibited
        );
        require_eq!(msg_ix, 0xFFFF, ResQError::CrossInstructionOffsetsProhibited);

        // Enforce strict offset boundaries to prevent overlapping input data
        require_eq!(pubkey_offset, 16, ResQError::InvalidPublicKeyOffset);
        require_eq!(sig_offset, 48, ResQError::InvalidSignatureOffset);
        require_eq!(
            msg_offset,
            PRECOMPILE_MSG_OFFSET,
            ResQError::InvalidMessageOffset
        );
        require_eq!(
            msg_size,
            PRECOMPILE_MSG_SIZE,
            ResQError::InvalidMessageLength
        );

        // Bounds-check the full extent of every slice *before* indexing into `data`.
        // Validating the header fields alone is not sufficient: a crafted precompile
        // instruction can advertise valid offsets/sizes while truncating the actual
        // data buffer, which would otherwise panic the program (denial of service).
        require!(
            data.len() >= pubkey_offset + 32,
            ResQError::MalformedPrecompileHeader
        );
        require!(
            data.len() >= sig_offset + 64,
            ResQError::MalformedPrecompileHeader
        );
        require!(
            data.len() >= msg_offset + msg_size,
            ResQError::MalformedPrecompileHeader
        );

        // Slice verification data according to validated offsets
        let verified_pubkey = &data[pubkey_offset..pubkey_offset + 32];
        let verified_sig = &data[sig_offset..sig_offset + 64];
        let verified_msg = &data[msg_offset..msg_offset + msg_size];

        // 5. Match the verified signature against the drone's registered on-chain key
        let registered_drone_bytes = permit.drone.to_bytes();
        require!(
            verified_pubkey == registered_drone_bytes,
            ResQError::DroneIdentityMismatch
        );

        // Validate signature match
        require!(verified_sig == signature, ResQError::SignatureMismatch);

        // 6. Reconstruct the expected message payload to verify telemetry data integrity.
        // Must match the packed `TelemetryPayload` layout signed on the drone
        // (services/edge-aeai/include/tpm_signing.hpp) byte-for-byte:
        // permit_id[32], waypoint_index (u16 LE), lat (f64 LE), lon (f64 LE),
        // alt (f32 LE), timestamp (i64 LE) — 62 bytes total.
        let mut expected_msg = Vec::with_capacity(PRECOMPILE_MSG_SIZE);
        expected_msg.extend_from_slice(&permit.permit_id);
        expected_msg.extend_from_slice(&permit.current_waypoint_index.to_le_bytes());
        expected_msg.extend_from_slice(&latitude.to_le_bytes());
        expected_msg.extend_from_slice(&longitude.to_le_bytes());
        expected_msg.extend_from_slice(&altitude.to_le_bytes());
        expected_msg.extend_from_slice(&timestamp.to_le_bytes());

        require!(
            verified_msg == expected_msg,
            ResQError::TelemetryPayloadSpoofed
        );

        // 7. Write the attestation data to the permanent ledger record
        let attestation = &mut ctx.accounts.attestation;
        attestation.permit = permit.key();
        attestation.waypoint_index = permit.current_waypoint_index;
        attestation.latitude = latitude;
        attestation.longitude = longitude;
        attestation.altitude = altitude;
        attestation.timestamp = timestamp;
        attestation.signature = signature;

        // Advance the waypoint sequence index to authorize the next route
        // transition. A checked increment prevents a long-lived permit from
        // overflowing the u16 counter (which would abort under the workspace's
        // overflow-checked release profile after all signature/payload checks
        // have already passed); once the sequence space is exhausted the permit
        // is retired instead of wrapping.
        let (next_index, still_active) = advance_waypoint(permit.current_waypoint_index);
        permit.current_waypoint_index = next_index;
        permit.is_active = still_active;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(permit_id: [u8; 32])]
pub struct InitializePermit<'info> {
    #[account(
        init,
        payer = operator,
        space = 8 + 32 + 32 + 32 + 32 + 2 + 1 + 1,
        seeds = [b"airspace_permit", operator.key().as_ref(), &permit_id],
        bump
    )]
    pub permit: Account<'info, AirspacePermit>,
    #[account(mut)]
    pub operator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SubmitAttestation<'info> {
    #[account(
        mut,
        seeds = [b"airspace_permit", permit.operator.as_ref(), &permit.permit_id],
        bump = permit.bump
    )]
    pub permit: Account<'info, AirspacePermit>,

    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 2 + 8 + 8 + 4 + 8 + 64,
        seeds = [
            b"location_attestation",
            permit.key().as_ref(),
            &permit.current_waypoint_index.to_le_bytes()
        ],
        bump
    )]
    pub attestation: Account<'info, LocationAttestation>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Instructions sysvar checked via direct program validation
    #[account(address = solana_instructions_sysvar::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct AirspacePermit {
    pub operator: Pubkey,
    /// Unique identifier for this permit; also used as a PDA seed and as the
    /// first field of the signed telemetry payload binding attestations to
    /// this specific permit.
    pub permit_id: [u8; 32],
    pub drone: Pubkey,
    pub route_root: [u8; 32],
    pub current_waypoint_index: u16,
    pub is_active: bool,
    pub bump: u8,
}

#[account]
pub struct LocationAttestation {
    pub permit: Pubkey,
    pub waypoint_index: u16,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f32,
    pub timestamp: i64,
    pub signature: [u8; 64],
}

#[error_code]
pub enum ResQError {
    #[msg("The specified airspace permit is inactive.")]
    InactivePermit,
    #[msg("The attestation timestamp is outside the allowed freshness window.")]
    StaleAttestation,
    #[msg("Missing signature verification precompile instruction.")]
    MissingSignaturePrecompile,
    #[msg("The preceding instruction does not target the native Ed25519 precompile.")]
    InvalidPrecompileProgram,
    #[msg("The precompile instruction header is malformed.")]
    MalformedPrecompileHeader,
    #[msg("The precompile must verify exactly one signature.")]
    InvalidSignatureCount,
    #[msg("Cross-instruction offsets are prohibited.")]
    CrossInstructionOffsetsProhibited,
    #[msg("The public key offset is incorrect.")]
    InvalidPublicKeyOffset,
    #[msg("The signature offset is incorrect.")]
    InvalidSignatureOffset,
    #[msg("The message offset is incorrect.")]
    InvalidMessageOffset,
    #[msg("The message length does not match the expected telemetry payload size.")]
    InvalidMessageLength,
    #[msg("The verified public key does not match the drone's registered on-chain identity.")]
    DroneIdentityMismatch,
    #[msg("The signature does not match the precompile record.")]
    SignatureMismatch,
    #[msg("The verified message payload does not match the submitted telemetry coordinates.")]
    TelemetryPayloadSpoofed,
    #[msg("The attestation coordinates are non-finite (NaN/inf) or outside valid WGS-84 ranges.")]
    InvalidCoordinates,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_timestamp_within_drift_window() {
        let now = 1_700_000_000_i64;
        assert!(is_within_clock_drift(now, now));
        assert!(is_within_clock_drift(
            now,
            now + MAX_ATTESTATION_CLOCK_DRIFT_SECS
        ));
        assert!(is_within_clock_drift(
            now,
            now - MAX_ATTESTATION_CLOCK_DRIFT_SECS
        ));
    }

    #[test]
    fn rejects_timestamp_outside_drift_window() {
        let now = 1_700_000_000_i64;
        assert!(!is_within_clock_drift(
            now,
            now + MAX_ATTESTATION_CLOCK_DRIFT_SECS + 1
        ));
        assert!(!is_within_clock_drift(
            now,
            now - MAX_ATTESTATION_CLOCK_DRIFT_SECS - 1
        ));
    }

    #[test]
    fn does_not_overflow_on_hostile_timestamps() {
        // A naive `(now - timestamp).abs()` panics under overflow-checked
        // arithmetic on these inputs; `is_within_clock_drift` must instead
        // reject them without panicking. This path runs before any signature
        // is verified, so it is reachable by an unauthenticated caller.
        let now = 1_700_000_000_i64;
        assert!(!is_within_clock_drift(now, i64::MIN));
        assert!(!is_within_clock_drift(now, i64::MAX));
        // Extreme clock values must also be handled symmetrically.
        assert!(!is_within_clock_drift(i64::MAX, i64::MIN));
        assert!(!is_within_clock_drift(i64::MIN, i64::MAX));
        assert!(!is_within_clock_drift(0, i64::MIN));
    }

    #[test]
    fn advance_waypoint_increments_within_range() {
        assert_eq!(advance_waypoint(0), (1, true));
        assert_eq!(advance_waypoint(41), (42, true));
        assert_eq!(advance_waypoint(u16::MAX - 1), (u16::MAX, true));
    }

    #[test]
    fn advance_waypoint_retires_permit_at_max_without_overflow() {
        // At u16::MAX a naive `+= 1` overflows and aborts the instruction under
        // the workspace's overflow-checked release profile. `advance_waypoint`
        // must instead retire the permit (still_active == false) and hold the
        // index steady so the counter never wraps or panics.
        assert_eq!(advance_waypoint(u16::MAX), (u16::MAX, false));
    }

    #[test]
    fn accepts_coordinates_within_valid_ranges() {
        assert!(are_coordinates_valid(0.0, 0.0));
        assert!(are_coordinates_valid(37.7749, -122.4194));
        // Boundary values are inclusive.
        assert!(are_coordinates_valid(90.0, 180.0));
        assert!(are_coordinates_valid(-90.0, -180.0));
    }

    #[test]
    fn rejects_non_finite_coordinates() {
        // A freshly signed but malformed payload with NaN/inf must never be
        // recorded on-chain, even though the Ed25519 signature would verify.
        assert!(!are_coordinates_valid(f64::NAN, 0.0));
        assert!(!are_coordinates_valid(0.0, f64::NAN));
        assert!(!are_coordinates_valid(f64::INFINITY, 0.0));
        assert!(!are_coordinates_valid(0.0, f64::NEG_INFINITY));
        assert!(!are_coordinates_valid(f64::INFINITY, f64::INFINITY));
    }

    #[test]
    fn rejects_out_of_range_coordinates() {
        // Physically impossible positions (e.g. latitude 500) must be rejected
        // so they cannot permanently advance the permit's waypoint sequence.
        assert!(!are_coordinates_valid(500.0, 0.0));
        assert!(!are_coordinates_valid(90.0001, 0.0));
        assert!(!are_coordinates_valid(-90.0001, 0.0));
        assert!(!are_coordinates_valid(0.0, 180.0001));
        assert!(!are_coordinates_valid(0.0, -180.0001));
        assert!(!are_coordinates_valid(500.0, 500.0));
    }
}
