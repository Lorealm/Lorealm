use crate::constants;
use crate::models::{EncryptedData, EncryptionContext};
use crate::traits::{Encryptable, EncryptionError, Hashable, KeyManager};
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{OsRng, Payload};
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce, aead::Aead};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
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
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(secret)
        .map_err(|_| EncryptionError::HashingFailed)?;
    mac.update(token.as_bytes());
    Ok(format!("{:x}", mac.finalize().into_bytes()))
}

pub fn verify_session_token(user_token: &str, server_token: &str) -> Result<bool, EncryptionError> {
    // Use constant time comparison
    Ok(user_token
        .as_bytes()
        .ct_eq(server_token.as_bytes())
        .unwrap_u8()
        == 1)
}

/// Encrypt `data`, cryptographically bound to `binding` (e.g. the row's
/// snowflake id as big-endian bytes). Decryption fails unless the same
/// binding is supplied, so a ciphertext copied into another row is useless.
pub(crate) async fn encrypt_bytes(
    data: &[u8],
    key_manager: &dyn KeyManager,
    context: EncryptionContext,
    binding: &[u8],
) -> Result<EncryptedData, EncryptionError> {
    let master_key = key_manager.get_master_key().await?;
    let version = key_manager.current_version();

    // Random salt -> fresh DEK per encryption (HKDF), sidestepping GCM's
    // nonce-collision bound under a single long-lived key.
    let mut salt = vec![0u8; constants::SALT_SIZE];
    OsRng.fill_bytes(&mut salt);

    let hk = Hkdf::<Sha256>::new(Some(&salt), &*master_key);
    let mut dek = Zeroizing::new([0u8; constants::DEK_SIZE]);
    hk.expand(&context.hkdf_info(version), &mut *dek)
        .map_err(|_| EncryptionError::KeyDerivationFailed)?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&*dek));

    let mut nonce_bytes = [0u8; constants::NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // The binding rides as AAD: authenticated, not stored in the row,
    // must be re-supplied verbatim on decrypt.
    let ciphertext_with_tag = cipher
        .encrypt(
            nonce,
            Payload {
                msg: data,
                aad: binding,
            },
        )
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

/// Decrypt. `binding` must match what was passed to `encrypt_bytes` for this
/// row. Uses the version and context *stored on the row* — never the current
/// ones — so rows encrypted before a key rotation stay readable.
///
/// Returns the plaintext zeroized-on-drop, since it's a secret.
pub(crate) async fn decrypt_bytes(
    encrypted: &EncryptedData,
    key_manager: &dyn KeyManager,
    binding: &[u8],
) -> Result<Zeroizing<Vec<u8>>, EncryptionError> {
    // Structural validation before any key material is touched.
    if encrypted.salt.len() != constants::SALT_SIZE {
        return Err(EncryptionError::CorruptedData);
    }

    // NOTE: requires a versioned getter on KeyManager:
    //     async fn get_master_key_version(&self, version: u32)
    //         -> Result<Zeroizing<[u8; 32]>, EncryptionError>;
    // get_master_key() can become a thin wrapper over it.
    let master_key = key_manager
        .get_master_key_version(encrypted.version)
        .await?;

    let hk = Hkdf::<Sha256>::new(Some(&encrypted.salt), &*master_key);
    let mut dek = Zeroizing::new([0u8; constants::DEK_SIZE]);
    hk.expand(&encrypted.context.hkdf_info(encrypted.version), &mut *dek)
        .map_err(|_| EncryptionError::KeyDerivationFailed)?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&*dek));
    let nonce = Nonce::from_slice(&encrypted.nonce);

    // Reassemble ciphertext || tag for the AEAD API.
    let mut combined = Vec::with_capacity(encrypted.ciphertext.len() + constants::TAG_SIZE);
    combined.extend_from_slice(&encrypted.ciphertext);
    combined.extend_from_slice(&encrypted.tag);

    let plaintext = cipher
        .decrypt(
            nonce,
            Payload {
                msg: &combined,
                aad: binding,
            },
        )
        // AEAD failure means wrong key, wrong binding (AAD), or tampering --
        // all authentication failures by construction.
        .map_err(|_| EncryptionError::AuthenticationFailed)?;

    Ok(Zeroizing::new(plaintext))
}

// Tie the constants to the algorithm's hard requirements at compile time.
const _: () = {
    assert!(constants::NONCE_SIZE == 12, "AES-GCM nonce must be 96 bits");
    assert!(constants::DEK_SIZE == 32, "AES-256 key must be 256 bits");
    assert!(constants::TAG_SIZE == 16, "GCM tag is 128 bits");
    assert!(
        constants::SALT_SIZE >= 16,
        "HKDF salt should be >= 128 bits"
    );
};

/// Implementation of Encryptable for String.
#[async_trait]
impl Encryptable for String {
    async fn encrypt(
        &self,
        key_manager: &dyn KeyManager,
        context: EncryptionContext,
        binding: &[u8],
    ) -> Result<EncryptedData, EncryptionError> {
        encrypt_bytes(self.as_bytes(), key_manager, context, binding).await
    }

    async fn decrypt(
        encrypted: &EncryptedData,
        key_manager: &dyn KeyManager,
        binding: &[u8],
    ) -> Result<Self, EncryptionError> {
        let mut plaintext = decrypt_bytes(encrypted, key_manager, binding).await?;
        // Move the buffer out without copying; the empty Vec left behind is
        // what the Zeroizing wrapper scrubs on drop.
        let bytes = std::mem::take(&mut *plaintext);
        match String::from_utf8(bytes) {
            Ok(s) => Ok(s),
            Err(e) => {
                // from_utf8's error owns the buffer; scrub it before dropping.
                drop(Zeroizing::new(e.into_bytes()));
                Err(EncryptionError::InvalidUtf8)
            }
        }
    }
}

/// Implementation for Vec<u8> to encrypt arbitrary binary data.
#[async_trait]
impl Encryptable for Vec<u8> {
    async fn encrypt(
        &self,
        key_manager: &dyn KeyManager,
        context: EncryptionContext,
        binding: &[u8],
    ) -> Result<EncryptedData, EncryptionError> {
        encrypt_bytes(self, key_manager, context, binding).await
    }

    async fn decrypt(
        encrypted: &EncryptedData,
        key_manager: &dyn KeyManager,
        binding: &[u8],
    ) -> Result<Self, EncryptionError> {
        let mut plaintext = decrypt_bytes(encrypted, key_manager, binding).await?;
        Ok(std::mem::take(&mut *plaintext))
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
        let binding: &[u8] = "123".as_bytes();

        let encrypted = original
            .encrypt(&km, EncryptionContext::SessionData, binding)
            .await
            .unwrap();
        let decrypted = String::decrypt(&encrypted, &km, binding).await.unwrap();

        assert_eq!(original, decrypted);
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_bytes() {
        let km = LocalKeyManager::new_random();
        let original = vec![1u8, 2, 3, 4, 5, 255, 128];
        let binding: &[u8] = "123".as_bytes();

        let encrypted = original
            .encrypt(&km, EncryptionContext::SessionData, binding)
            .await
            .unwrap();
        let decrypted = Vec::<u8>::decrypt(&encrypted, &km, binding).await.unwrap();

        assert_eq!(original, decrypted);
    }

    #[tokio::test]
    async fn test_encryption_produces_different_ciphertext() {
        let km = LocalKeyManager::new_random();
        let data = "same data".to_string();
        let binding: &[u8] = "123".as_bytes();

        let encrypted1 = data
            .encrypt(&km, EncryptionContext::SessionData, binding)
            .await
            .unwrap();
        let encrypted2 = data
            .encrypt(&km, EncryptionContext::SessionData, binding)
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
        let binding: &[u8] = "123".as_bytes();

        let encrypted = data
            .encrypt(&km1, EncryptionContext::SessionData, binding)
            .await
            .unwrap();
        let result = String::decrypt(&encrypted, &km2, binding).await;

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
        let binding: &[u8] = "123".as_bytes();

        let mut encrypted = data
            .encrypt(&km, EncryptionContext::SessionData, binding)
            .await
            .unwrap();

        // Tamper with the ciphertext
        if !encrypted.ciphertext.is_empty() {
            encrypted.ciphertext[0] ^= 1;
        }

        let result = String::decrypt(&encrypted, &km, binding).await;
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
        let binding: &[u8] = "123".as_bytes();

        let mut encrypted = data
            .encrypt(&km, EncryptionContext::SessionData, binding)
            .await
            .unwrap();
        encrypted.version = 99; // Unsupported version

        let result = String::decrypt(&encrypted, &km, binding).await;
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
        let binding: &[u8] = "123".as_bytes();

        let mut encrypted = data
            .encrypt(&km, EncryptionContext::SessionData, binding)
            .await
            .unwrap();
        encrypted.salt = vec![0u8; 16]; // Wrong size

        let result = String::decrypt(&encrypted, &km, binding).await;
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
