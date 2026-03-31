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

use {
    solana_keypair::Keypair,
    solana_loader_v3_interface::state::UpgradeableLoaderState,
    solana_message::{v0::Message, VersionedMessage},
    solana_program_test::ProgramTest,
    solana_pubkey::Pubkey,
    solana_sdk_ids::bpf_loader_upgradeable,
    solana_signer::Signer,
    solana_system_interface::instruction as system_instruction,
    solana_transaction::{versioned::VersionedTransaction, Transaction},
};

#[tokio::test]
async fn test_bpf_loader_upgradeable_present() {
    // Arrange
    let (banks_client, payer, recent_blockhash) = ProgramTest::default().start().await;

    let buffer_keypair = Keypair::new();
    let upgrade_authority_keypair = Keypair::new();

    let rent = banks_client.get_rent().await.unwrap();
    let buffer_rent = rent.minimum_balance(UpgradeableLoaderState::size_of_programdata(1));

    let create_buffer_instructions = solana_loader_v3_interface::instruction::create_buffer(
        &payer.pubkey(),
        &buffer_keypair.pubkey(),
        &upgrade_authority_keypair.pubkey(),
        buffer_rent,
        1,
    )
    .unwrap();

    let mut transaction =
        Transaction::new_with_payer(&create_buffer_instructions[..], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &buffer_keypair], recent_blockhash);

    // Act
    banks_client.process_transaction(transaction).await.unwrap();

    // Assert
    let buffer_account = banks_client
        .get_account(buffer_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(buffer_account.owner, bpf_loader_upgradeable::id());
}

#[tokio::test]
async fn versioned_transaction() {
    let program_test = ProgramTest::default();
    let context = program_test.start_with_context().await;

    let program_id = Pubkey::new_unique();
    let account = Keypair::new();
    let rent = context.banks_client.get_rent().await.unwrap();
    let space = 82;
    let transaction = VersionedTransaction::try_new(
        VersionedMessage::V0(
            Message::try_compile(
                &context.payer.pubkey(),
                &[system_instruction::create_account(
                    &context.payer.pubkey(),
                    &account.pubkey(),
                    rent.minimum_balance(space),
                    space as u64,
                    &program_id,
                )],
                &[],
                context.last_blockhash,
            )
            .unwrap(),
        ),
        &[&context.payer, &account],
    )
    .unwrap();

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}
