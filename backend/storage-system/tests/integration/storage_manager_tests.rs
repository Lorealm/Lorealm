use tempfile::TempDir;

use storage_system::{
    BackendConfig, StorageConfig, StorageManager,
    test_utils::{ConversationBuilder, UserBuilder},
};
use uuid::Uuid;

#[tokio::test]
async fn test_storage_manager_file_backend() {
    let temp = TempDir::new().unwrap();

    let config = StorageConfig {
        backends: vec![BackendConfig::File {
            base_path: temp.path().to_string_lossy().to_string(),
            pretty: true,
        }],
        require_postgres_fallback: false,
        cache: None,
    };

    let storage = StorageManager::new(config).await.unwrap();
    assert_eq!(storage.backend_name(), "file");
    assert!(!storage.is_cache_enabled());

    let convo_id = Uuid::new_v4();

    // Test basic operations
    let conversation = ConversationBuilder::new().id(convo_id).build();

    storage.save_conversation(&conversation).await.unwrap();
    let retrieved = storage.get_conversation(convo_id).await.unwrap();
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_storage_manager_memory_backend() {
    let config = StorageConfig {
        backends: vec![BackendConfig::Memory { max_size: 1000 }],
        require_postgres_fallback: false,
        cache: None,
    };

    let storage = StorageManager::new(config).await.unwrap();
    assert_eq!(storage.backend_name(), "memory");

    let user_id = Uuid::new_v4();

    let user = UserBuilder::new().id(user_id).build();

    storage.save_user(&user).await.unwrap();
    let retrieved = storage.get_user_by_id(user_id).await.unwrap();
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_storage_manager_deref() {
    let config = StorageConfig {
        backends: vec![BackendConfig::Memory { max_size: 100 }],
        require_postgres_fallback: false,
        cache: None,
    };

    let storage = StorageManager::new(config).await.unwrap();

    // Test that Deref works - we can call storage methods directly
    let conversation = ConversationBuilder::new().build();
    storage.save_conversation(&conversation).await.unwrap();
}
