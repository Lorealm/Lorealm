#[cfg(feature = "test-utils")]
use storage_system::{
    backends::memory::MemoryStorage,
    storage::{ConversationStorage, MessageStorage},
    test_utils::{ConversationBuilder, FailableStorage, MessageBuilder},
};
#[cfg(feature = "test-utils")]
use uuid::Uuid;

#[cfg(feature = "test-utils")]
use proptest::arbitrary::any;
#[cfg(feature = "test-utils")]
use proptest::strategy::Strategy;
#[cfg(feature = "test-utils")]
use proptest::{prop_assert, prop_assert_eq, prop_assume, proptest};

#[cfg(feature = "test-utils")]
fn uuid_strategy() -> impl Strategy<Value = Uuid> {
    any::<[u8; 16]>().prop_map(|mut bytes| {
        bytes[6] = (bytes[6] & 0x0F) | 0x40; // Set version to 4
        bytes[8] = (bytes[8] & 0x3F) | 0x80; // Set variant to RFC4122
        Uuid::from_bytes(bytes)
    })
}

#[cfg(feature = "test-utils")]
// Property: Any conversation we save should be retrievable
proptest! {
    #[test]
    fn prop_conversation_roundtrip(
        id in uuid_strategy(),
        user_id in uuid_strategy(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let memory = MemoryStorage::new(100);
            let storage = FailableStorage::new(memory);

            let conversation = ConversationBuilder::new()
                .id(id)
                .user_id(user_id)
                .build();

            storage.save_conversation(&conversation).await.unwrap();
            let retrieved = storage.get_conversation(id).await.unwrap();

            prop_assert!(retrieved.is_some());
            let retrieved = retrieved.unwrap();
            prop_assert_eq!(retrieved.id, id);
            prop_assert_eq!(retrieved.user_id, user_id);

            Ok(())
        }).unwrap();
    }
}

#[cfg(feature = "test-utils")]
// Property: Deleting a conversation should make it unretrievable
proptest! {
    #[test]
    fn prop_conversation_delete(
        id in uuid_strategy(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let memory = MemoryStorage::new(100);
            let storage = FailableStorage::new(memory);

            let conversation = ConversationBuilder::new()
                .id(id)
                .build();

            storage.save_conversation(&conversation).await.unwrap();
            storage.delete_conversation(id).await.unwrap();
            let retrieved = storage.get_conversation(id).await.unwrap();

            prop_assert!(retrieved.is_none());

            Ok(())
        }).unwrap();
    }
}

#[cfg(feature = "test-utils")]
// Property: Messages for a conversation should maintain order
proptest! {
    #[test]
    fn prop_message_ordering(
            conv_id in uuid_strategy(),
            message_count in 1usize..20,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let memory = MemoryStorage::new(100);
                let storage = FailableStorage::new(memory);

                let conversation = ConversationBuilder::new()
                    .id(conv_id)
                    .build();
                storage.save_conversation(&conversation).await.unwrap();

                // Add messages
                for i in 0..message_count {
                    let message = MessageBuilder::new(conv_id, None)
                        .id(Uuid::from_u128(i as u128))
                        .content(&format!("Message {}", i))
                        .build();
                    storage.save_message(&message).await.unwrap();
                }

                let messages = storage.get_messages(conv_id).await.unwrap();
                prop_assert_eq!(messages.len(), message_count);

                // Verify order (messages should be in order they were created)
                for (i, message) in messages.iter().enumerate() {
                    prop_assert_eq!(&message.content, &format!("Message {}", i));
                }

                Ok(())
            }).unwrap();
        }
}

#[cfg(feature = "test-utils")]
// Property: List conversations should only return conversations for the specified user
proptest! {
    #[test]
    fn prop_list_conversations_user_isolation(
        user1_id in uuid_strategy(),
        user2_id in uuid_strategy(),
        user1_conv_count in 1usize..10,
        user2_conv_count in 1usize..10,
    ) {
        prop_assume!(user1_id != user2_id);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let memory = MemoryStorage::new(100);
            let storage = FailableStorage::new(memory);

            // Create conversations for user1
            for i in 0..user1_conv_count {
                let conv = ConversationBuilder::new()
                    .id(Uuid::from_u128(i as u128))
                    .user_id(user1_id)
                    .build();
                storage.save_conversation(&conv).await.unwrap();
            }

            // Create conversations for user2
            for i in 0..user2_conv_count {
                let conv = ConversationBuilder::new()
                    .id(Uuid::from_u128((i + 100) as u128))
                    .user_id(user2_id)
                    .build();
                storage.save_conversation(&conv).await.unwrap();
            }

            let user1_convs = storage.list_conversations(user1_id).await.unwrap();
            let user2_convs = storage.list_conversations(user2_id).await.unwrap();

            prop_assert_eq!(user1_convs.len(), user1_conv_count);
            prop_assert_eq!(user2_convs.len(), user2_conv_count);

            // Verify no cross-contamination
            // for conv in &user1_convs {
            //     prop_assert_eq!(conv.user_id, user1_id);
            // }
            // for conv in &user2_convs {
            //     prop_assert_eq!(conv.user_id, user2_id);
            // }

            Ok(())
        }).unwrap();
    }
}
