use crate::{test_utils::{fixtures::{ConversationBuilder, MessageBuilder, UserBuilder}, MockStorage}, Storage, StorageError};

#[tokio::test]
async fn test_mock_storage_conversation_crud() {
    let storage = MockStorage::new();
    
    // Create
    let conversation = ConversationBuilder::new()
        .id("conv_123")
        .user_id("user_456")
        .title("Test Chat")
        .build();
    
    let id = storage.save_conversation(&conversation).await.unwrap();
    assert_eq!(id, "conv_123");
    
    // Read
    let retrieved = storage.get_conversation("conv_123").await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().title, Some("Test Chat".to_string()));
    
    // Update
    let mut updated = conversation.clone();
    updated.title = Some("Updated Title".to_string());
    storage.save_conversation(&updated).await.unwrap();
    
    let retrieved = storage.get_conversation("conv_123").await.unwrap();
    assert_eq!(retrieved.unwrap().title, Some("Updated Title".to_string()));
    
    // Delete
    storage.delete_conversation("conv_123").await.unwrap();
    let retrieved = storage.get_conversation("conv_123").await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_mock_storage_messages() {
    let storage = MockStorage::new();
    
    let conversation = ConversationBuilder::new()
        .id("conv_123")
        .build();
    storage.save_conversation(&conversation).await.unwrap();
    
    // Add messages
    let msg1 = MessageBuilder::new("conv_123")
        .id("msg_1")
        .content("Hello")
        .build();
    
    let msg2 = MessageBuilder::new("conv_123")
        .id("msg_2")
        .content("World")
        .build();
    
    storage.save_message(&msg1).await.unwrap();
    storage.save_message(&msg2).await.unwrap();
    
    // Retrieve messages
    let messages = storage.get_messages("conv_123").await.unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].content, "Hello");
    assert_eq!(messages[1].content, "World");
}

#[tokio::test]
async fn test_mock_storage_list_conversations() {
    let storage = MockStorage::new();
    
    let conv1 = ConversationBuilder::new()
        .id("conv_1")
        .user_id("user_1")
        .build();
    
    let conv2 = ConversationBuilder::new()
        .id("conv_2")
        .user_id("user_1")
        .build();
    
    let conv3 = ConversationBuilder::new()
        .id("conv_3")
        .user_id("user_2")
        .build();
    
    storage.save_conversation(&conv1).await.unwrap();
    storage.save_conversation(&conv2).await.unwrap();
    storage.save_conversation(&conv3).await.unwrap();
    
    let user1_convs = storage.list_conversations("user_1").await.unwrap();
    assert_eq!(user1_convs.len(), 2);
    
    let user2_convs = storage.list_conversations("user_2").await.unwrap();
    assert_eq!(user2_convs.len(), 1);
}

#[tokio::test]
async fn test_mock_storage_failure_mode() {
    let storage = MockStorage::new();
    storage.set_should_fail(true);
    
    let conversation = ConversationBuilder::new().build();
    let result = storage.save_conversation(&conversation).await;
    
    assert!(result.is_err());
    match result {
        Err(StorageError::Database(msg)) => assert_eq!(msg, "Mock failure"),
        _ => panic!("Expected Database error"),
    }
}

#[tokio::test]
async fn test_user_crud() {
    let storage = MockStorage::new();
    
    let user = UserBuilder::new()
        .id("user_123")
        .email("test@example.com")
        .display_name("Test User")
        .build();
    
    storage.save_user(&user).await.unwrap();
    
    let retrieved = storage.get_user("user_123").await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().email, Some("test@example.com".to_string()));
}