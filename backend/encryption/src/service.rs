// Add to lib.rs or new convenience.rs module

use crate::{Encryptable, EncryptedData, EncryptionContext, EncryptionError, KeyManager};

/// High-level encryption functions with context built-in
pub struct EncryptionService<'a> {
    key_manager: &'a dyn KeyManager,
}

impl<'a> EncryptionService<'a> {
    pub fn new(key_manager: &'a dyn KeyManager) -> Self {
        Self { key_manager }
    }

    /// Encrypt an API key
    pub async fn encrypt_api_key(&self, api_key: &str, binding: &[u8]) -> Result<EncryptedData, EncryptionError> {
        api_key
            .to_string()
            .encrypt(self.key_manager, EncryptionContext::ApiKey, binding)
            .await
    }

    /// Encrypt file data
    pub async fn encrypt_file(&self, data: Vec<u8>, binding: &[u8]) -> Result<EncryptedData, EncryptionError> {
        data.encrypt(self.key_manager, EncryptionContext::FileData, binding)
            .await
    }

    pub async fn encrypt_session_token(&self, token: &str, binding: &[u8]) -> Result<EncryptedData, EncryptionError> {
        token
            .to_string()
            .encrypt(self.key_manager, EncryptionContext::SessionData, binding)
            .await
    }

    /// Generic encryption with custom context
    pub async fn encrypt_with_context(
        &self,
        data: &str,
        context: EncryptionContext,
        binding: &[u8]
    ) -> Result<EncryptedData, EncryptionError> {
        data.to_string().encrypt(self.key_manager, context, binding).await
    }

    /// Decrypt any encrypted data (context is embedded)
    pub async fn decrypt_string(
        &self,
        encrypted: &EncryptedData,
        binding: &[u8]
    ) -> Result<String, EncryptionError> {
        String::decrypt(encrypted, self.key_manager, binding).await
    }

    /// Decrypt binary data
    pub async fn decrypt_bytes(
        &self,
        encrypted: &EncryptedData,
        binding: &[u8]
    ) -> Result<Vec<u8>, EncryptionError> {
        Vec::<u8>::decrypt(encrypted, self.key_manager, binding).await
    }
}
