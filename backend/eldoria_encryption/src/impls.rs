use crate::constants;
use crate::models::{EncryptedData, EncryptionContext};
use crate::traits::{Encryptable, EncryptionError, Hashable, KeyManager};
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::OsRng;
use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit, Nonce};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use async_trait::async_trait;
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// Implementation of Hashable for String (typically for passwords).
/// Uses Argon2id for secure password hashing.
#[async_trait]
impl Hashable for String {
    async fn hash(&self) -> Result<String, EncryptionError> {
        let salt = SaltString::generate(OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(self.as_bytes(), &salt)
            .map_err(|_| EncryptionError::HashingFailed)?;

        Ok(password_hash.to_string())
    }
}

/// Implementation of Hashable for &str (for convenience).
#[async_trait]
impl Hashable for &str {
    async fn hash(&self) -> Result<String, EncryptionError> {
        self.to_string().hash().await
    }
}

/// Helper function to verify a password against a hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, EncryptionError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| EncryptionError::InvalidHashFormat)?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn hash_session_token(token: &str, secret: &[u8]) -> Result<String, EncryptionError> {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(secret).map_err(|_| EncryptionError::HashingFailed)?;
    mac.update(token.as_bytes());
    Ok(format!("{:x}", mac.finalize().into_bytes()))
}

pub fn verify_session_token(user_token: &str, server_token: &str) -> Result<bool, EncryptionError> {
    // Use constant time comparison
    Ok(user_token.as_bytes().ct_eq(server_token.as_bytes()).unwrap_u8() == 1)
}

/// Core encryption implementation that works with any byte slice.
/// This eliminates code duplication between String and Vec<u8> implementations.
async fn encrypt_bytes(
    data: &[u8],
    key_manager: &dyn KeyManager,
    context: EncryptionContext,
) -> Result<EncryptedData, EncryptionError> {
    let master_key = key_manager.get_master_key().await?;
    let version = key_manager.current_version();

    // Generate a random salt for key derivation using cryptographically secure RNG
    let mut salt = vec![0u8; constants::SALT_SIZE];
    OsRng.fill_bytes(&mut salt);

    // Derive a data encryption key (DEK) using HKDF with zeroization
    let hk = Hkdf::<Sha256>::new(Some(&salt), &master_key);
    let mut dek = Zeroizing::new([0u8; constants::DEK_SIZE]);
    hk.expand(&context.hkdf_info(version), &mut *dek)
        .map_err(|_| EncryptionError::KeyDerivationFailed)?;

    // Create cipher
    // Temporarily muting deprecation errors due to an issue within `aes-gcm`
    // See https://github.com/RustCrypto/traits/issues/2036
    #[allow(deprecated)]
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&*dek));

    // Generate random nonce using cryptographically secure RNG
    let mut nonce_bytes = [0u8; constants::NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    #[allow(deprecated)]
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt the data
    let ciphertext_with_tag = cipher
        .encrypt(nonce, data)
        .map_err(|_| EncryptionError::EncryptionFailed)?;

    // Split ciphertext and tag (GCM appends 16-byte tag)
    if ciphertext_with_tag.len() < constants::TAG_SIZE {
        return Err(EncryptionError::InvalidCiphertext);
    }

    let (ciphertext, tag_slice) =
        ciphertext_with_tag.split_at(ciphertext_with_tag.len() - constants::TAG_SIZE);
    let mut tag = [0u8; constants::TAG_SIZE];
    tag.copy_from_slice(tag_slice);

    Ok(EncryptedData {
        version,
        context,
        salt,
        nonce: nonce_bytes,
        ciphertext: ciphertext.to_vec(),
        tag,
    })
}

/// Core decryption implementation that returns raw bytes.
/// This eliminates code duplication between String and Vec<u8> implementations.
async fn decrypt_bytes(
    encrypted: &EncryptedData,
    key_manager: &dyn KeyManager,
) -> Result<Vec<u8>, EncryptionError> {
    // Validate version
    if encrypted.version != constants::SUPPORTED_VERSION {
        return Err(EncryptionError::UnsupportedVersion(encrypted.version));
    }

    // Validate data integrity
    if encrypted.salt.len() != constants::SALT_SIZE {
        return Err(EncryptionError::CorruptedData);
    }

    if encrypted.nonce.len() != constants::NONCE_SIZE {
        return Err(EncryptionError::CorruptedData);
    }

    if encrypted.ciphertext.len() > constants::MAX_CIPHERTEXT_SIZE {
        return Err(EncryptionError::CiphertextTooLarge);
    }

    let master_key = key_manager.get_master_key().await?;

    // Derive the same DEK using stored salt with zeroization
    let hk = Hkdf::<Sha256>::new(Some(&encrypted.salt), &master_key);
    let mut dek = Zeroizing::new([0u8; constants::DEK_SIZE]);
    hk.expand(&encrypted.context.hkdf_info(encrypted.version), &mut *dek)
        .map_err(|_| EncryptionError::KeyDerivationFailed)?;

    // Create cipher
    // Temporarily muting deprecation errors due to an issue within `aes-gcm`
    // See https://github.com/RustCrypto/traits/issues/2036
    #[allow(deprecated)]
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&*dek));
    #[allow(deprecated)]
    let nonce = Nonce::from_slice(&encrypted.nonce);

    // Reconstruct ciphertext with tag
    let mut ciphertext_with_tag = encrypted.ciphertext.clone();
    ciphertext_with_tag.extend_from_slice(&encrypted.tag);

    // Decrypt - authentication failure will be caught here
    cipher
        .decrypt(nonce, ciphertext_with_tag.as_ref())
        .map_err(|_| EncryptionError::AuthenticationFailed)
}

/// Implementation of Encryptable for String.
/// Delegates to the shared encrypt_bytes/decrypt_bytes functions.
#[async_trait]
impl Encryptable for String {
    async fn encrypt(
        &self,
        key_manager: &dyn KeyManager,
        context: EncryptionContext,
    ) -> Result<EncryptedData, EncryptionError> {
        encrypt_bytes(self.as_bytes(), key_manager, context).await
    }

    async fn decrypt(
        encrypted: &EncryptedData,
        key_manager: &dyn KeyManager,
    ) -> Result<Self, EncryptionError> {
        let plaintext = decrypt_bytes(encrypted, key_manager).await?;
        String::from_utf8(plaintext).map_err(|_| EncryptionError::InvalidUtf8)
    }
}

/// Implementation for Vec<u8> to encrypt arbitrary binary data.
/// Delegates to the shared encrypt_bytes/decrypt_bytes functions.
#[async_trait]
impl Encryptable for Vec<u8> {
    async fn encrypt(
        &self,
        key_manager: &dyn KeyManager,
        context: EncryptionContext,
    ) -> Result<EncryptedData, EncryptionError> {
        encrypt_bytes(self.as_ref(), key_manager, context).await
    }

    async fn decrypt(
        encrypted: &EncryptedData,
        key_manager: &dyn KeyManager,
    ) -> Result<Self, EncryptionError> {
        decrypt_bytes(encrypted, key_manager).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::LocalKeyManager;

    #[tokio::test]
    async fn test_hash_password() {
        let password = "my_secure_password".to_string();
        let hash1 = password.hash().await.unwrap();
        let hash2 = password.hash().await.unwrap();

        // Hashes should be different due to random salt
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(verify_password(&password, &hash1).unwrap());
        assert!(verify_password(&password, &hash2).unwrap());
    }

    #[tokio::test]
    async fn test_verify_wrong_password() {
        let password = "correct_password".to_string();
        let hash = password.hash().await.unwrap();

        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_string() {
        let km = LocalKeyManager::new_random();
        let original = "Hello, World! ðŸ”’".to_string();

        let encrypted = original
            .encrypt(&km, EncryptionContext::SessionData)
            .await
            .unwrap();
        let decrypted = String::decrypt(&encrypted, &km).await.unwrap();

        assert_eq!(original, decrypted);
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_bytes() {
        let km = LocalKeyManager::new_random();
        let original = vec![1u8, 2, 3, 4, 5, 255, 128];

        let encrypted = original
            .encrypt(&km, EncryptionContext::SessionData)
            .await
            .unwrap();
        let decrypted = Vec::<u8>::decrypt(&encrypted, &km).await.unwrap();

        assert_eq!(original, decrypted);
    }

    #[tokio::test]
    async fn test_encryption_produces_different_ciphertext() {
        let km = LocalKeyManager::new_random();
        let data = "same data".to_string();

        let encrypted1 = data
            .encrypt(&km, EncryptionContext::SessionData)
            .await
            .unwrap();
        let encrypted2 = data
            .encrypt(&km, EncryptionContext::SessionData)
            .await
            .unwrap();

        // Different salt and nonce should produce different ciphertext
        assert_ne!(encrypted1.salt, encrypted2.salt);
        assert_ne!(encrypted1.nonce, encrypted2.nonce);
        assert_ne!(encrypted1.ciphertext, encrypted2.ciphertext);
    }

    #[tokio::test]
    async fn test_decrypt_with_wrong_key_fails() {
        let km1 = LocalKeyManager::new_random();
        let km2 = LocalKeyManager::new_random();
        let data = "secret data".to_string();

        let encrypted = data
            .encrypt(&km1, EncryptionContext::SessionData)
            .await
            .unwrap();
        let result = String::decrypt(&encrypted, &km2).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EncryptionError::AuthenticationFailed
        ));
    }

    #[tokio::test]
    async fn test_tampered_ciphertext_fails() {
        let km = LocalKeyManager::new_random();
        let data = "original data".to_string();

        let mut encrypted = data
            .encrypt(&km, EncryptionContext::SessionData)
            .await
            .unwrap();

        // Tamper with the ciphertext
        if !encrypted.ciphertext.is_empty() {
            encrypted.ciphertext[0] ^= 1;
        }

        let result = String::decrypt(&encrypted, &km).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EncryptionError::AuthenticationFailed
        ));
    }

    #[tokio::test]
    async fn test_unsupported_version_fails() {
        let km = LocalKeyManager::new_random();
        let data = "test data".to_string();

        let mut encrypted = data
            .encrypt(&km, EncryptionContext::SessionData)
            .await
            .unwrap();
        encrypted.version = 99; // Unsupported version

        let result = String::decrypt(&encrypted, &km).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EncryptionError::UnsupportedVersion(99)
        ));
    }

    #[tokio::test]
    async fn test_corrupted_salt_fails() {
        let km = LocalKeyManager::new_random();
        let data = "test data".to_string();

        let mut encrypted = data
            .encrypt(&km, EncryptionContext::SessionData)
            .await
            .unwrap();
        encrypted.salt = vec![0u8; 16]; // Wrong size

        let result = String::decrypt(&encrypted, &km).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EncryptionError::CorruptedData
        ));
    }

    #[tokio::test]
    async fn test_invalid_hash_format() {
        let result = verify_password("password", "not-a-valid-hash");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EncryptionError::InvalidHashFormat
        ));
    }
}
