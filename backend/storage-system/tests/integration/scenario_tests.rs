use storage_system::{
    backends::memory::MemoryStorage,
    storage::{ConversationStorage, MessageStorage, UserStorage},
    test_utils::{ConversationBuilder, FailableStorage, MessageBuilder, UserBuilder},
};
use uuid::Uuid;

#[tokio::test]
async fn test_intermittent_failure_scenario() {
    let inner = MemoryStorage::new(100);
    let storage = FailableStorage::new(inner);

    let user_id = Uuid::new_v4();
    let conversation = ConversationBuilder::new().user_id(user_id).build();

    // Initial save succeeds
    storage.save_conversation(&conversation).await.unwrap();

    // Simulate intermittent failure
    storage.set_should_fail(true).await;
    let result = storage.get_conversation(conversation.id).await;
    assert!(result.is_err());

    // Recovery
    storage.set_should_fail(false).await;
    let retrieved = storage.get_conversation(conversation.id).await.unwrap();
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_multi_entity_operations_with_failures() {
    let inner = MemoryStorage::new(100);
    let storage = FailableStorage::new(inner);

    // Create entities
    let user = UserBuilder::new().build();
    let conversation = ConversationBuilder::new().user_id(user.id).build();
    let message = MessageBuilder::new(conversation.id, None).build();

    // Save user
    storage.save_user(&user).await.unwrap();

    // Save conversation
    storage.save_conversation(&conversation).await.unwrap();

    // Enable failures
    storage.set_should_fail(true).await;

    // Message save should fail
    let result = storage.save_message(&message).await;
    assert!(result.is_err());

    // Disable failures
    storage.reset().await;

    // Now it should work
    storage.save_message(&message).await.unwrap();

    // Verify everything is there
    let messages = storage.get_messages(conversation.id).await.unwrap();
    assert_eq!(messages.len(), 1);
}

#[tokio::test]
async fn test_cascading_operations_with_failure() {
    let inner = MemoryStorage::new(100);
    let storage = FailableStorage::new(inner);

    let user_id = Uuid::new_v4();

    // Create multiple conversations
    for i in 0..5 {
        let conversation = ConversationBuilder::new()
            .user_id(user_id)
            .context_type(format!("test_{}", i))
            .build();
        storage.save_conversation(&conversation).await.unwrap();
    }

    // List should show all 5
    let conversations = storage.list_conversations(user_id).await.unwrap();
    assert_eq!(conversations.len(), 5);

    // Enable failures
    storage.set_should_fail(true).await;

    // Can't list anymore
    let result = storage.list_conversations(user_id).await;
    assert!(result.is_err());

    // Can't create new ones
    let new_conversation = ConversationBuilder::new().user_id(user_id).build();
    let result = storage.save_conversation(&new_conversation).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_failure_after_partial_success() {
    let inner = MemoryStorage::new(100);
    let storage = FailableStorage::new(inner);

    let conversation = ConversationBuilder::new().build();
    storage.save_conversation(&conversation).await.unwrap();

    // Add some messages successfully
    for i in 0..3 {
        let message = MessageBuilder::new(conversation.id, None)
            .content(format!("Message {}", i))
            .build();
        storage.save_message(&message).await.unwrap();
    }

    // Enable failures
    storage.set_should_fail(true).await;

    // Try to add more messages - should fail
    let message = MessageBuilder::new(conversation.id, None)
        .content("Failed message")
        .build();
    let result = storage.save_message(&message).await;
    assert!(result.is_err());

    // Disable failures
    storage.reset().await;

    // Should still have only 3 messages
    let messages = storage.get_messages(conversation.id).await.unwrap();
    assert_eq!(messages.len(), 3);
}
