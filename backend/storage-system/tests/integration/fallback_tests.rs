use storage_system::{
    backends::memory::MemoryStorage,
    config::{BackendConfig, StorageConfig},
    storage::{ConversationStorage, StorageManager},
    test_utils::{ConversationBuilder, FailableStorage},
};
use uuid::Uuid;

#[tokio::test]
async fn test_storage_manager_falls_back_on_unavailable_backend() {
    // Create two memory storages wrapped in FailableStorage
    let primary = MemoryStorage::new(100);
    let secondary = MemoryStorage::new(100);

    let failable_primary = FailableStorage::new(primary);
    let _failable_secondary = FailableStorage::new(secondary);

    // Make primary unavailable
    failable_primary.set_fail_on_availability(true).await;

    // Note: This test shows the concept, but StorageManager currently creates
    // backends from config. You'd need to either:
    // 1. Modify StorageManager to accept pre-built backends
    // 2. Test with actual backend configs (file paths that don't exist, etc.)

    // For now, let's test with actual configs
    let config = StorageConfig {
        backends: vec![
            BackendConfig::File {
                base_path: "/nonexistent/path".into(),
                pretty: false,
            },
            BackendConfig::Memory { max_size: 100 },
        ],
        cache: None,
        require_postgres_fallback: false,
    };

    let manager = StorageManager::new(config).await.unwrap();

    // Should have fallen back to memory storage
    assert_eq!(manager.backend_name(), "memory");
    assert!(manager.is_available().await);
}

#[tokio::test]
async fn test_storage_manager_uses_first_available_backend() {
    let config = StorageConfig {
        backends: vec![
            BackendConfig::Memory { max_size: 100 },
            BackendConfig::File {
                base_path: "/tmp/test_storage".into(),
                pretty: false,
            },
        ],
        cache: None,
        require_postgres_fallback: false,
    };

    let manager = StorageManager::new(config).await.unwrap();

    // Should use memory (first available)
    assert_eq!(manager.backend_name(), "memory");
}

#[tokio::test]
async fn test_storage_manager_fails_when_no_backends_available() {
    let config = StorageConfig {
        backends: vec![
            BackendConfig::File {
                base_path: "/nonexistent/path1".into(),
                pretty: false,
            },
            BackendConfig::File {
                base_path: "/nonexistent/path2".into(),
                pretty: false,
            },
        ],
        cache: None,
        require_postgres_fallback: false,
    };

    let result = StorageManager::new(config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_failable_storage_with_actual_operations() {
    let inner = MemoryStorage::new(100);
    let storage = FailableStorage::new(inner);

    let user_id = Uuid::new_v4();
    let conversation = ConversationBuilder::new().user_id(user_id).build();

    // Save conversation
    storage.save_conversation(&conversation).await.unwrap();

    // Verify it's there
    let retrieved = storage.get_conversation(conversation.id).await.unwrap();
    assert!(retrieved.is_some());

    // Enable failures
    storage.set_should_fail(true).await;

    // Try to list - should fail
    let result = storage.list_conversations(user_id).await;
    assert!(result.is_err());

    // Disable failures
    storage.reset().await;

    // Should work now
    let conversations = storage.list_conversations(user_id).await.unwrap();
    assert_eq!(conversations.len(), 1);
}
