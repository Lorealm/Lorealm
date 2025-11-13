use storage_system::{
    StorageError,
    backends::memory::MemoryStorage,
    storage::{ConversationStorage, MessageStorage, StorageCore, UserStorage},
    test_utils::{ConversationBuilder, FailableStorage, MessageBuilder, UserBuilder},
};
use uuid::Uuid;

#[tokio::test]
async fn test_failable_storage_normal_operations() {
    let inner = MemoryStorage::new(100);
    let storage = FailableStorage::new(inner);

    // Normal operations should work
    assert!(storage.is_available().await);

    let user_id = Uuid::new_v4();
    let conversation = ConversationBuilder::new().user_id(user_id).build();

    let result = storage.save_conversation(&conversation).await;
    assert!(result.is_ok());

    let retrieved = storage.get_conversation(conversation.id).await;
    assert!(retrieved.is_ok());
    assert!(retrieved.unwrap().is_some());
}

#[tokio::test]
async fn test_failable_storage_fails_when_configured() {
    let inner = MemoryStorage::new(100);
    let storage = FailableStorage::new(inner);

    let conversation = ConversationBuilder::new().build();

    // Should work initially
    storage.save_conversation(&conversation).await.unwrap();

    // Enable failures
    storage.set_should_fail(true).await;

    // Should now fail
    let result = storage.get_conversation(conversation.id).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), StorageError::Database(_)));
}

#[tokio::test]
async fn test_failable_storage_availability_failure() {
    let inner = MemoryStorage::new(100);
    let storage = FailableStorage::new(inner);

    assert!(storage.is_available().await);

    storage.set_fail_on_availability(true).await;
    assert!(!storage.is_available().await);

    storage.reset().await;
    assert!(storage.is_available().await);
}

#[tokio::test]
async fn test_failable_storage_init_failure() {
    let inner = MemoryStorage::new(100);
    let mut storage = FailableStorage::new(inner);

    storage.set_fail_on_init(true).await;

    let result = storage.initialize().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_failable_storage_selective_operations() {
    let inner = MemoryStorage::new(100);
    let storage = FailableStorage::new(inner);

    let user_id = Uuid::new_v4();
    let conversation = ConversationBuilder::new().user_id(user_id).build();

    // Save succeeds
    storage.save_conversation(&conversation).await.unwrap();

    // Enable failures
    storage.set_should_fail(true).await;

    // Get fails
    let result = storage.get_conversation(conversation.id).await;
    assert!(result.is_err());

    // Disable failures
    storage.set_should_fail(false).await;

    // Get now succeeds
    let result = storage.get_conversation(conversation.id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_failable_storage_message_operations() {
    let inner = MemoryStorage::new(100);
    let storage = FailableStorage::new(inner);

    let conversation = ConversationBuilder::new().build();
    storage.save_conversation(&conversation).await.unwrap();

    let message = MessageBuilder::new(conversation.id, None).build();

    // Normal save
    storage.save_message(&message).await.unwrap();

    // Enable failures
    storage.set_should_fail(true).await;

    // Should fail
    let result = storage.get_messages(conversation.id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_failable_storage_user_operations() {
    let inner = MemoryStorage::new(100);
    let storage = FailableStorage::new(inner);

    let user = UserBuilder::new().build();

    // Normal save
    storage.save_user(&user).await.unwrap();

    // Enable failures
    storage.set_should_fail(true).await;

    // Should fail
    let result = storage.get_user_by_id(user.id).await;
    assert!(result.is_err());

    let result = storage.get_user_by_name("test").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_failable_storage_reset() {
    let inner = MemoryStorage::new(100);
    let storage = FailableStorage::new(inner);

    // Enable all failures
    storage.set_should_fail(true).await;
    storage.set_fail_on_init(true).await;
    storage.set_fail_on_availability(true).await;

    assert!(!storage.is_available().await);

    // Reset
    storage.reset().await;

    // Should work again
    assert!(storage.is_available().await);

    let conversation = ConversationBuilder::new().build();
    let result = storage.save_conversation(&conversation).await;
    assert!(result.is_ok());
}
