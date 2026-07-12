// ---- tests for the binding / rotation / versioning work -----------------

use lorealm_encryption::{
    Encryptable, EncryptionContext, EncryptionError, KeyManager, LocalKeyManager,
};

#[tokio::test]
async fn test_decrypt_with_wrong_binding_fails() {
    let km = LocalKeyManager::new_random();
    let data = "bound data".to_string();

    let encrypted = data
        .encrypt(&km, EncryptionContext::SessionData, b"row-1")
        .await
        .unwrap();

    // Same key, same context, same row tuple — but presented as a
    // different row. This is the copied-ciphertext attack the AAD exists
    // to stop.
    let result = String::decrypt(&encrypted, &km, b"row-2").await;
    assert!(matches!(
        result.unwrap_err(),
        EncryptionError::AuthenticationFailed
    ));
}

#[tokio::test]
async fn test_ciphertext_not_portable_across_contexts() {
    let km = LocalKeyManager::new_random();
    let binding: &[u8] = b"row-1";

    let mut encrypted = "context-bound"
        .to_string()
        .encrypt(&km, EncryptionContext::SessionData, binding)
        .await
        .unwrap();

    // Swap the stored context: the derived DEK differs, so this must fail.
    // NOTE: substitute any other variant your EncryptionContext has.
    encrypted.context = EncryptionContext::ApiKey;

    let result = String::decrypt(&encrypted, &km, binding).await;
    assert!(matches!(
        result.unwrap_err(),
        EncryptionError::AuthenticationFailed
    ));
}

#[tokio::test]
async fn test_rotation_preserves_old_rows() {
    let mut km = LocalKeyManager::new(vec![0x11; 32]).unwrap();
    let binding: &[u8] = b"row-1";

    let old = "pre-rotation"
        .to_string()
        .encrypt(&km, EncryptionContext::SessionData, binding)
        .await
        .unwrap();
    assert_eq!(old.version, 1);

    km.rotate_key(vec![0x22; 32]).await.unwrap();
    assert_eq!(km.current_version(), 2);

    // The old row must still decrypt — via its stored version, not the
    // current one. This is the regression test for replace-style rotation.
    let decrypted = String::decrypt(&old, &km, binding).await.unwrap();
    assert_eq!(decrypted, "pre-rotation");

    // New encryptions pick up the new version.
    let fresh = "post-rotation"
        .to_string()
        .encrypt(&km, EncryptionContext::SessionData, binding)
        .await
        .unwrap();
    assert_eq!(fresh.version, 2);
}

#[tokio::test]
async fn test_re_encrypt_migrates_to_current_version() {
    let mut km = LocalKeyManager::new(vec![0x11; 32]).unwrap();
    let binding: &[u8] = b"row-1";

    let old = "migrate me"
        .to_string()
        .encrypt(&km, EncryptionContext::SessionData, binding)
        .await
        .unwrap();

    km.rotate_key(vec![0x22; 32]).await.unwrap();
    let migrated = km.re_encrypt(&old, binding).await.unwrap();

    assert_eq!(migrated.version, 2);
    assert_eq!(
        String::decrypt(&migrated, &km, binding).await.unwrap(),
        "migrate me"
    );

    // Migration must preserve the row binding, not launder it away.
    let result = String::decrypt(&migrated, &km, b"row-2").await;
    assert!(matches!(
        result.unwrap_err(),
        EncryptionError::AuthenticationFailed
    ));
}

#[tokio::test]
async fn test_key_history_restores_all_versions() {
    // Simulates a process restart after one rotation: the new process is
    // constructed from the full key history and must read rows from
    // every generation.
    let binding: &[u8] = b"row-1";

    let mut km = LocalKeyManager::new(vec![0x11; 32]).unwrap();
    let v1_row = "v1 data"
        .to_string()
        .encrypt(&km, EncryptionContext::SessionData, binding)
        .await
        .unwrap();
    km.rotate_key(vec![0x22; 32]).await.unwrap();
    let v2_row = "v2 data"
        .to_string()
        .encrypt(&km, EncryptionContext::SessionData, binding)
        .await
        .unwrap();

    let restarted =
        LocalKeyManager::new_with_history(vec![vec![0x11; 32], vec![0x22; 32]]).unwrap();
    assert_eq!(restarted.current_version(), 2);
    assert_eq!(
        String::decrypt(&v1_row, &restarted, binding).await.unwrap(),
        "v1 data"
    );
    assert_eq!(
        String::decrypt(&v2_row, &restarted, binding).await.unwrap(),
        "v2 data"
    );
}

#[tokio::test]
async fn test_rotate_rejects_wrong_length_key() {
    let mut km = LocalKeyManager::new(vec![0x11; 32]).unwrap();

    let result = km.rotate_key(vec![0u8; 16]).await;
    assert!(matches!(
        result.unwrap_err(),
        EncryptionError::InvalidKeyLength {
            expected: 32,
            actual: 16
        }
    ));
    // A failed rotation must not bump the version.
    assert_eq!(km.current_version(), 1);
}

#[tokio::test]
async fn test_decrypt_non_utf8_as_string_fails() {
    let km = LocalKeyManager::new_random();
    let binding: &[u8] = b"row-1";

    // Valid data, but not valid UTF-8.
    let binary = vec![0u8, 159, 146, 150];
    let encrypted = binary
        .encrypt(&km, EncryptionContext::SessionData, binding)
        .await
        .unwrap();

    // Decrypts fine as bytes...
    assert_eq!(
        Vec::<u8>::decrypt(&encrypted, &km, binding).await.unwrap(),
        binary
    );
    // ...but as String it's a clean InvalidUtf8, not a panic or a
    // misleading auth failure.
    let result = String::decrypt(&encrypted, &km, binding).await;
    assert!(matches!(result.unwrap_err(), EncryptionError::InvalidUtf8));
}
