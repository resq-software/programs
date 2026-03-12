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
}