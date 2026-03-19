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
pub enum DeliveryError {
    /// The IPFS CID bytes are all zero — a valid CID must be provided.
    #[msg("IPFS CID must not be empty")]
    EmptyCid,
    /// The supplied timestamp is zero or negative.
    #[msg("delivered_at timestamp must be a positive Unix epoch value")]
    InvalidTimestamp,
    /// The drone PDA provided does not match the signing authority.
    #[msg("Drone PDA does not match the transaction signer")]
    DroneMismatch,
    /// Latitude is outside the valid range (−90° to +90° × 1e7).
    #[msg("Latitude out of range: must be between -900_000_000 and 900_000_000")]
    LatitudeOutOfRange,
    /// Longitude is outside the valid range (−180° to +180° × 1e7).
    #[msg("Longitude out of range: must be between -1_800_000_000 and 1_800_000_000")]
    LongitudeOutOfRange,
    /// The provided airspace account is not owned by the resq-airspace program.
    #[msg("Airspace account must be owned by the resq-airspace program")]
    InvalidAirspace,
    /// delivered_at is more than 5 minutes before the current block time.
    #[msg("delivered_at must be within 5 minutes of the current block time")]
    TimestampTooOld,
    /// delivered_at is more than 60 seconds ahead of the current block time.
    #[msg("delivered_at must not be more than 60 seconds in the future")]
    TimestampInFuture,
}