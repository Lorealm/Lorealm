use async_trait::async_trait;
use thiserror::Error;

use crate::models::{EncryptedData, EncryptionContext};

#[derive(Error, Debug)]
pub enum EncryptionError {
    // Hashing errors
    #[error("Password hashing failed")]
    HashingFailed,

    #[error("Invalid password hash format")]
    InvalidHashFormat,

    #[error("Password verification failed")]
    VerificationFailed,

    // Key management errors
    #[error("Invalid key length: expected {expected} bytes, got {actual} bytes")]
    InvalidKeyLength { expected: usize, actual: usize },

    #[error("Key derivation failed")]
    KeyDerivationFailed,

    #[error("Key not found for version {0}")]
    KeyNotFound(u32),

    #[error("Failed to load key from environment variable: {0}")]
    KeyLoadError(String),

    #[error("Invalid hex encoding in key")]
    InvalidHexEncoding,

    // Encryption/Decryption errors
    #[error("Encryption operation failed")]
    EncryptionFailed,

    #[error("Decryption operation failed")]
    DecryptionFailed,

    #[error("Authentication tag verification failed")]
    AuthenticationFailed,

    #[error("Invalid UTF-8 in decrypted data")]
    InvalidUtf8,

    // Versioning errors
    #[error("Unsupported encryption version: {0}")]
    UnsupportedVersion(u32),

    #[error(
        "Version mismatch: encrypted with v{encrypted}, attempting to decrypt with v{current}"
    )]
    VersionMismatch { encrypted: u32, current: u32 },

    // Data validation errors
    #[error("Invalid ciphertext format")]
    InvalidCiphertext,

    #[error("Corrupted encrypted data")]
    CorruptedData,

    #[error("Ciphertext too large")]
    CiphertextTooLarge,
    #[error("Failed to convert to bytes")]
    Bincode(bincode::error::EncodeError),
}

#[async_trait]
pub trait Hashable {
    /// Hash the value (e.g., for passwords). Returns the hashed string.
    async fn hash(&self) -> Result<String, EncryptionError>;
}

#[async_trait]
pub trait Encryptable {
    /// Encrypt the value, returning an EncryptedData struct (as discussed previously).
    async fn encrypt(
        &self,
        key_manager: &dyn KeyManager,
        context: EncryptionContext,
    ) -> Result<EncryptedData, EncryptionError>;

    /// Decrypt from EncryptedData back to Self.
    async fn decrypt(
        encrypted: &EncryptedData,
        key_manager: &dyn KeyManager,
    ) -> Result<Self, EncryptionError>
    where
        Self: Sized;
}

/// Optional: A manager trait for key handling (local or AWS KMS).
#[async_trait]
pub trait KeyManager: Send + Sync {
    async fn get_master_key(&self) -> Result<Vec<u8>, EncryptionError>;
    fn current_version(&self) -> u32;
}
