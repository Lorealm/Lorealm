use crate::{
    EncryptedData, constants, decrypt_bytes, encrypt_bytes,
    traits::{EncryptionError, KeyManager},
};
use aes_gcm::aead::{OsRng, rand_core::RngCore};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use zeroize::Zeroizing;

/// Local key manager that stores master keys in memory, one per version.
/// Old versions are retained so rows encrypted before a rotation remain
/// decryptable; `re_encrypt` migrates them forward.
#[derive(Debug)]
pub struct LocalKeyManager {
    keys: HashMap<u32, Arc<Zeroizing<Vec<u8>>>>,
    current: u32,
}

impl LocalKeyManager {
    /// Create a new LocalKeyManager with a provided master key (version 1).
    /// The key must be 32 bytes (256 bits) for AES-256.
    pub fn new(master_key: Vec<u8>) -> Result<Self, EncryptionError> {
        Self::new_with_history(vec![master_key])
    }

    /// Create a manager from a full key history, oldest first; the last
    /// entry becomes the current version. This is how a restarted process
    /// regains the ability to decrypt rows from every key generation.
    pub fn new_with_history(history: Vec<Vec<u8>>) -> Result<Self, EncryptionError> {
        if history.is_empty() {
            return Err(EncryptionError::KeyLoadError(
                "key history must contain at least one key".to_string(),
            ));
        }
        let mut keys = HashMap::new();
        let mut current = 0;
        for (i, key) in history.into_iter().enumerate() {
            if key.len() != constants::MASTER_KEY_SIZE {
                return Err(EncryptionError::InvalidKeyLength {
                    expected: constants::MASTER_KEY_SIZE,
                    actual: key.len(),
                });
            }
            current = (i + 1) as u32;
            keys.insert(current, Arc::new(Zeroizing::new(key)));
        }
        Ok(Self { keys, current })
    }

    /// Create a new LocalKeyManager with a randomly generated master key.
    /// Useful for testing or initial setup.
    ///
    /// # Security
    /// Uses OsRng for cryptographically secure random key generation.
    pub fn new_random() -> Self {
        let mut master_key = vec![0u8; constants::MASTER_KEY_SIZE];
        OsRng.fill_bytes(&mut master_key);
        Self::new(master_key).expect("generated key has the correct length")
    }

    /// Load a single master key (version 1) from an environment variable.
    /// Expected format: hex-encoded 32-byte key.
    pub fn from_env(var_name: &str) -> Result<Self, EncryptionError> {
        let key_hex = std::env::var(var_name).map_err(|e| {
            EncryptionError::KeyLoadError(format!(
                "Environment variable '{var_name}' not found: {e}"
            ))
        })?;

        let master_key = hex::decode(&key_hex).map_err(|_| EncryptionError::InvalidHexEncoding)?;

        Self::new(master_key)
    }

    /// Load a key history from an environment variable containing
    /// comma-separated hex keys, oldest first (the last is current).
    ///
    /// # Example
    /// ```bash
    /// export MASTER_KEYS="<v1 hex>,<v2 hex>"
    /// ```
    pub fn from_env_with_history(var_name: &str) -> Result<Self, EncryptionError> {
        let raw = std::env::var(var_name).map_err(|e| {
            EncryptionError::KeyLoadError(format!(
                "Environment variable '{var_name}' not found: {e}"
            ))
        })?;

        let history = raw
            .split(',')
            .map(|h| hex::decode(h.trim()).map_err(|_| EncryptionError::InvalidHexEncoding))
            .collect::<Result<Vec<_>, _>>()?;

        Self::new_with_history(history)
    }

    /// Get the current version of the master key.
    pub fn version(&self) -> u32 {
        self.current
    }

    /// Rotate to a new master key, incrementing the version. Previous keys
    /// are retained: new encryptions use the new key, while existing rows
    /// keep decrypting under their stored version until migrated with
    /// `re_encrypt`.
    pub async fn rotate_key(&mut self, new_key: Vec<u8>) -> Result<(), EncryptionError> {
        if new_key.len() != constants::MASTER_KEY_SIZE {
            return Err(EncryptionError::InvalidKeyLength {
                expected: constants::MASTER_KEY_SIZE,
                actual: new_key.len(),
            });
        }
        self.current += 1;
        self.keys
            .insert(self.current, Arc::new(Zeroizing::new(new_key)));
        Ok(())
    }

    /// Re-encrypt a row's data under the current master key. Decrypts using
    /// the version stored on the row, so this works *after* rotation — which
    /// is the point. The binding (row identity) must match the original and
    /// is carried over to the new ciphertext.
    pub async fn re_encrypt(
        &self,
        old_encrypted: &EncryptedData,
        binding: &[u8],
    ) -> Result<EncryptedData, EncryptionError> {
        let plaintext = decrypt_bytes(old_encrypted, self, binding).await?;
        encrypt_bytes(&plaintext, self, old_encrypted.context.clone(), binding).await
    }
}

#[async_trait]
impl KeyManager for LocalKeyManager {
    async fn get_master_key(&self) -> Result<Zeroizing<Vec<u8>>, EncryptionError> {
        self.get_master_key_version(self.current).await
    }

    async fn get_master_key_version(
        &self,
        version: u32,
    ) -> Result<Zeroizing<Vec<u8>>, EncryptionError> {
        self.keys
            .get(&version)
            .map(|k| Zeroizing::new((***k).clone()))
            .ok_or(EncryptionError::UnsupportedVersion(version))
    }

    fn current_version(&self) -> u32 {
        self.current
    }
}
