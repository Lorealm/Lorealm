use storage_system::backends::file::FileStorage;
use storage_system::{Storage, StorageError};
use storage_system::test_utils::fixtures::*;
use tempfile::TempDir;

async fn setup_file_storage() -> (FileStorage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = FileStorage::new(temp_dir.path(), true).unwrap();
    storage.initialize().await.unwrap();
    (storage, temp_dir)
}

#[tokio::test]
async fn test_file_storage_safe_path() {
    let (storage, _temp) = setup_file_storage().await;
    
    let conversation = ConversationBuilder::new()
        .id("safe_id_123")
        .build();
    
    storage.save_conversation(&conversation).await.unwrap();
    let retrieved = storage.get_conversation("safe_id_123").await.unwrap();
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_file_storage_path_traversal_protection() {
    let (storage, _temp) = setup_file_storage().await;
    
    // Try path traversal attack
    let conversation = ConversationBuilder::new()
        .id("../../../etc/passwd")
        .build();
    
    let result = storage.save_conversation(&conversation).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_file_storage_atomic_write() {
    let (storage, _temp) = setup_file_storage().await;
    
    let conversation = ConversationBuilder::new()
        .id("atomic_test")
        .title("Original")
        .build();
    
    storage.save_conversation(&conversation).await.unwrap();
    
    // Update multiple times rapidly
    for i in 0..10 {
        let mut updated = conversation.clone();
        updated.title = Some(format!("Update {}", i));
        storage.save_conversation(&updated).await.unwrap();
    }
    
    // Should have the last update
    let retrieved = storage.get_conversation("atomic_test").await.unwrap();
    assert_eq!(retrieved.unwrap().title, Some("Update 9".to_string()));
}

#[tokio::test]
async fn test_file_storage_sharding() {
    let (storage, temp) = setup_file_storage().await;
    
    // Create conversations with IDs that will shard differently
    for i in 0..20 {
        let conversation = ConversationBuilder::new()
            .id(&format!("conv_{:04}", i))
            .build();
        storage.save_conversation(&conversation).await.unwrap();
    }
    
    // Verify sharded directory structure exists
    let conversations_dir = temp.path().join("conversations");
    assert!(conversations_dir.exists());
    
    // Check that shard directories were created
    let entries: Vec<_> = std::fs::read_dir(conversations_dir)
        .unwrap()
        .collect();
    assert!(entries.len() > 1); // Should have multiple shard directories
}

#[tokio::test]
async fn test_file_storage_invalid_characters() {
    let (storage, _temp) = setup_file_storage().await;
    
    let invalid_ids = vec![
        "id/with/slashes",
        "id\\with\\backslashes",
        "id\0with\0nulls",
        "..",
        ".",
        "",
    ];
    
    for id in invalid_ids {
        let conversation = ConversationBuilder::new()
            .id(id)
            .build();
        
        let result = storage.save_conversation(&conversation).await;
        assert!(result.is_err(), "Should reject invalid ID: {}", id);
    }
}