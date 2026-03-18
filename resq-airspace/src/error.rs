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

#[error_code]
pub enum AirspaceError {
    /// Only the airspace owner may call this instruction.
    #[msg("Caller is not the airspace owner")]
    Unauthorized,
    /// Property ID bytes are all zero — a valid identifier must be provided.
    #[msg("Property ID must not be empty")]
    EmptyPropertyId,
    /// Altitude bounds are invalid (min must be < max).
    #[msg("min_alt_m must be less than max_alt_m")]
    InvalidAltitudeBounds,
    /// Polygon vertex count is outside the 1–8 range.
    #[msg("vertex_count must be between 1 and 8")]
    InvalidVertexCount,
    /// The drone does not hold a valid Permit for this airspace.
    #[msg("Drone does not hold a valid permit for this airspace")]
    NoValidPermit,
    /// The permit has expired.
    #[msg("Permit has expired")]
    PermitExpired,
    /// The fee transfer failed.
    #[msg("Crossing fee transfer failed")]
    FeeTransferFailed,
    /// expires_at is in the past.
    #[msg("Permit expiry must be in the future")]
    ExpiryInPast,
    /// Drone altitude is outside the airspace's declared bounds.
    #[msg("Drone altitude is outside the permitted altitude range for this airspace")]
    AltitudeOutOfBounds,
    /// crossed_at timestamp is zero or negative.
    #[msg("crossed_at must be a positive Unix epoch value")]
    InvalidTimestamp,
    /// crossed_at timestamp is too far in the future.
    #[msg("crossed_at must not be more than 60 seconds in the future")]
    TimestampInFuture,
    /// crossed_at timestamp is too far in the past (> 5 minutes before block time).
    #[msg("crossed_at must be within 5 minutes of the current block time")]
    TimestampTooOld,
    /// Latitude is outside the valid range (−90° to +90° × 1e7).
    #[msg("Latitude out of range: must be between -900_000_000 and 900_000_000")]
    LatitudeOutOfRange,
    /// Longitude is outside the valid range (−180° to +180° × 1e7).
    #[msg("Longitude out of range: must be between -1_800_000_000 and 1_800_000_000")]
    LongitudeOutOfRange,
    /// Treasury pubkey is the zero address; fees would be permanently lost.
    #[msg("Treasury must not be the zero address")]
    InvalidTreasury,
}