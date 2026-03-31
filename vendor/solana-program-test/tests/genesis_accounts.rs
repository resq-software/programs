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

use {solana_account::Account, solana_program_test::ProgramTest, solana_pubkey::Pubkey};

#[tokio::test]
async fn genesis_accounts() {
    let my_genesis_accounts = [
        (
            Pubkey::new_unique(),
            Account::new(1, 0, &solana_system_interface::program::id()),
        ),
        (
            Pubkey::new_unique(),
            Account::new(1, 0, &solana_sdk_ids::config::id()),
        ),
        (
            Pubkey::new_unique(),
            Account::new(1, 0, &solana_sdk_ids::feature::id()),
        ),
        (
            Pubkey::new_unique(),
            Account::new(1, 0, &solana_stake_interface::program::id()),
        ),
    ];

    let mut program_test = ProgramTest::default();

    for (pubkey, account) in my_genesis_accounts.iter() {
        program_test.add_genesis_account(*pubkey, account.clone());
    }

    let context = program_test.start_with_context().await;

    // Verify the accounts are present.
    for (pubkey, account) in my_genesis_accounts.iter() {
        let fetched_account = context
            .banks_client
            .get_account(*pubkey)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched_account, *account);
    }
}
