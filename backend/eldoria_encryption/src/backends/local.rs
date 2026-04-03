use crate::{
    Encryptable, EncryptedData, constants,
    traits::{EncryptionError, KeyManager},
};
use aes_gcm::aead::{OsRng, rand_core::RngCore};
use async_trait::async_trait;
use std::sync::Arc;
use zeroize::Zeroizing;

/// Local key manager that stores the master key in memory.
/// In production, you might load this from environment variables or a secure file.
#[derive(Debug)]
pub struct LocalKeyManager {
    master_key: Arc<Zeroizing<Vec<u8>>>,
    version: u32,
}

impl LocalKeyManager {
    /// Create a new LocalKeyManager with a provided master key.
    /// The key should be 32 bytes (256 bits) for AES-256.
    pub fn new(master_key: Vec<u8>) -> Result<Self, EncryptionError> {
        if master_key.len() != constants::MASTER_KEY_SIZE {
            return Err(EncryptionError::InvalidKeyLength {
                expected: constants::MASTER_KEY_SIZE,
                actual: master_key.len(),
            });
        }
        Ok(Self {
            master_key: Arc::new(Zeroizing::new(master_key)),
            version: 1,
        })
    }

    /// Create a new LocalKeyManager with a randomly generated master key.
    /// Useful for testing or initial setup.
    ///
    /// # Security
    /// Uses OsRng for cryptographically secure random key generation.
    pub fn new_random() -> Self {
        let mut master_key = vec![0u8; constants::MASTER_KEY_SIZE];
        OsRng.fill_bytes(&mut master_key);

        Self {
            master_key: Arc::new(Zeroizing::new(master_key)),
            version: 1,
        }
    }

    /// Load master key from environment variable.
    /// Expected format: hex-encoded 32-byte key.
    ///
    /// # Example
    /// ```bash
    /// export MASTER_KEY="0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    /// ```
    pub fn from_env(var_name: &str) -> Result<Self, EncryptionError> {
        let key_hex = std::env::var(var_name).map_err(|e| {
            EncryptionError::KeyLoadError(format!(
                "Environment variable '{var_name}' not found: {e}"
            ))
        })?;

        let master_key = hex::decode(&key_hex).map_err(|_| EncryptionError::InvalidHexEncoding)?;

        Self::new(master_key)
    }

    /// Get the current version of the master key.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Rotate the master key to a new key and increment the version.
    ///
    /// This method updates the master key stored in the key manager and increments
    /// the version number. After rotation, new encryptions will use the new key,
    /// but old encrypted data can still be decrypted using `re_encrypt`.
    ///
    /// # Arguments
    /// * `new_key` - The new master key (should be 32 bytes for AES-256)
    ///
    /// # Returns
    /// * `Ok(())` on successful key rotation
    /// * `Err(EncryptionError)` if the key rotation fails
    ///
    /// # Example
    /// ```ignore
    /// let mut manager = LocalKeyManager::new_random();
    /// let new_key = vec![1u8; 32];
    /// manager.rotate_key(new_key).await?;
    /// ```
    pub async fn rotate_key(&mut self, new_key: Vec<u8>) -> Result<(), EncryptionError> {
        self.version += 1;
        self.master_key = Arc::new(Zeroizing::new(new_key));
        Ok(())
    }

    /// Re-encrypt data with the current master key.
    ///
    /// This method decrypts data that was encrypted with a previous version of the
    /// master key and re-encrypts it with the current master key. Useful after key
    /// rotation to migrate old encrypted data to use the new key.
    ///
    /// # Arguments
    /// * `old_encrypted` - The encrypted data to re-encrypt
    ///
    /// # Returns
    /// * `Ok(EncryptedData)` containing the data encrypted with the current master key
    /// * `Err(EncryptionError)` if decryption or re-encryption fails
    ///
    /// # Example
    /// ```ignore
    /// let old_data = /* previously encrypted data */;
    /// let new_data = manager.re_encrypt(&old_data).await?;
    /// ```
    pub async fn re_encrypt(
        &self,
        old_encrypted: &EncryptedData,
    ) -> Result<EncryptedData, EncryptionError> {
        let plaintext = String::decrypt(old_encrypted, self).await?;
        plaintext.encrypt(self, old_encrypted.context.clone()).await
    }
}

#[async_trait]
impl KeyManager for LocalKeyManager {
    async fn get_master_key(&self) -> Result<Vec<u8>, EncryptionError> {
        Ok((**self.master_key).clone())
    }

    fn current_version(&self) -> u32 {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_random() {
        let manager = LocalKeyManager::new_random();
        assert_eq!(manager.version(), 1);
    }

    #[test]
    fn test_new_with_valid_key() {
        let key = vec![0u8; 32];
        let manager = LocalKeyManager::new(key).unwrap();
        assert_eq!(manager.version(), 1);
    }

    #[test]
    fn test_new_with_invalid_key_too_short() {
        let key = vec![0u8; 16]; // Too short
        let result = LocalKeyManager::new(key);
        assert!(result.is_err());
        match result.unwrap_err() {
            EncryptionError::InvalidKeyLength { expected, actual } => {
                assert_eq!(expected, 32);
                assert_eq!(actual, 16);
            }
            _ => panic!("Expected InvalidKeyLength error"),
        }
    }

    #[test]
    fn test_new_with_invalid_key_too_long() {
        let key = vec![0u8; 64]; // Too long
        let result = LocalKeyManager::new(key);
        assert!(result.is_err());
        match result.unwrap_err() {
            EncryptionError::InvalidKeyLength { expected, actual } => {
                assert_eq!(expected, 32);
                assert_eq!(actual, 64);
            }
            _ => panic!("Expected InvalidKeyLength error"),
        }
    }

    #[tokio::test]
    async fn test_get_master_key() {
        let key = vec![42u8; 32];
        let manager = LocalKeyManager::new(key.clone()).unwrap();
        let retrieved = manager.get_master_key().await.unwrap();
        assert_eq!(*retrieved, key);
    }

    #[test]
    fn test_from_env_valid() {
        // Set up test environment variable
        let test_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        unsafe {
            std::env::set_var("TEST_MASTER_KEY", test_key);
        }

        let result = LocalKeyManager::from_env("TEST_MASTER_KEY");
        assert!(result.is_ok());

        let manager = result.unwrap();
        assert_eq!(manager.version(), 1);

        // Clean up
        unsafe {
            std::env::remove_var("TEST_MASTER_KEY");
        }
    }

    #[test]
    fn test_from_env_missing_var() {
        let result = LocalKeyManager::from_env("NONEXISTENT_KEY");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EncryptionError::KeyLoadError(_)
        ));
    }

    #[test]
    fn test_from_env_invalid_hex() {
        unsafe { std::env::set_var("TEST_INVALID_KEY", "not-valid-hex!") };

        let result = LocalKeyManager::from_env("TEST_INVALID_KEY");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EncryptionError::InvalidHexEncoding
        ));

        unsafe { std::env::remove_var("TEST_INVALID_KEY") };
    }

    #[test]
    fn test_from_env_wrong_length() {
        // Valid hex but wrong length (only 16 bytes)
        unsafe { std::env::set_var("TEST_SHORT_KEY", "0123456789abcdef0123456789abcdef") };

        let result = LocalKeyManager::from_env("TEST_SHORT_KEY");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EncryptionError::InvalidKeyLength { .. }
        ));

        unsafe { std::env::remove_var("TEST_SHORT_KEY") };
    }

    #[tokio::test]
    async fn test_random_keys_are_different() {
        let manager1 = LocalKeyManager::new_random();
        let manager2 = LocalKeyManager::new_random();

        let key1 = manager1.get_master_key().await.unwrap();
        let key2 = manager2.get_master_key().await.unwrap();

        // Extremely unlikely to be equal if RNG is working correctly
        assert_ne!(key1, key2);
    }
}
