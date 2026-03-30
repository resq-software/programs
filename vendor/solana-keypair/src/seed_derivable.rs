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

//! Implementation of the SeedDerivable trait for Keypair

use {
    crate::{keypair_from_seed, keypair_from_seed_phrase_and_passphrase, Keypair},
    ed25519_dalek_bip32::Error as Bip32Error,
    solana_derivation_path::DerivationPath,
    solana_seed_derivable::SeedDerivable,
    std::error,
};

impl SeedDerivable for Keypair {
    fn from_seed(seed: &[u8]) -> Result<Self, Box<dyn error::Error>> {
        keypair_from_seed(seed)
    }

    fn from_seed_and_derivation_path(
        seed: &[u8],
        derivation_path: Option<DerivationPath>,
    ) -> Result<Self, Box<dyn error::Error>> {
        keypair_from_seed_and_derivation_path(seed, derivation_path)
    }

    fn from_seed_phrase_and_passphrase(
        seed_phrase: &str,
        passphrase: &str,
    ) -> Result<Self, Box<dyn error::Error>> {
        keypair_from_seed_phrase_and_passphrase(seed_phrase, passphrase)
    }
}

pub fn keypair_from_seed_and_derivation_path(
    seed: &[u8],
    derivation_path: Option<DerivationPath>,
) -> Result<Keypair, Box<dyn error::Error>> {
    let derivation_path = derivation_path.unwrap_or_default();
    bip32_derived_keypair(seed, derivation_path).map_err(|err| err.to_string().into())
}

fn bip32_derived_keypair(
    seed: &[u8],
    derivation_path: DerivationPath,
) -> Result<Keypair, Bip32Error> {
    let extended = ed25519_dalek_bip32::ExtendedSigningKey::from_seed(seed)
        .and_then(|extended| extended.derive(&derivation_path))?;
    Ok(Keypair(extended.signing_key))
}
