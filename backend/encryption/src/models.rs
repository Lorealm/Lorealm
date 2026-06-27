use base64::{Engine, prelude::BASE64_STANDARD};
use bincode::Encode;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Struct to hold encrypted data with metadata for decryption and rotation.
/// This is stored in the DB/file as serialized JSON or binary (e.g., via bincode).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Zeroize, ZeroizeOnDrop, Encode)]
pub struct EncryptedData {
    /// Key version (starts at 1; increment on rotation to track which MK was used).
    pub version: u32,

    /// Encryption context/domain for key derivation
    pub context: EncryptionContext,

    /// Per-item salt for DEK derivation (e.g., 32 bytes from rand).
    pub salt: Vec<u8>,

    /// Nonce/IV for AES-GCM (12 bytes recommended for security/randomness).
    pub nonce: [u8; 12],

    /// The encrypted payload (variable length).
    pub ciphertext: Vec<u8>,

    /// Authentication tag (16 bytes for GCM).
    pub tag: [u8; 16],
}

/// Defines the purpose/domain for encrypted data.
/// Each context uses a different HKDF info string for key derivation,
/// ensuring cryptographic separation between different data types.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Zeroize, ZeroizeOnDrop, Encode,
)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionContext {
    /// API keys, service tokens, OAuth tokens
    ApiKey = 0,

    /// Files and documents
    FileData = 1,

    /// Session tokens and temporary secrets
    SessionData = 2,

    /// Encryption keys (for key wrapping)
    KeyMaterial = 3,
}

impl EncryptionContext {
    /// Get the HKDF context string for key derivation.
    /// Format: "encryption-service:{domain}:dek:v{version}"
    pub fn hkdf_info(&self, version: u32) -> Vec<u8> {
        let domain = match self {
            Self::ApiKey => "api-key",
            Self::FileData => "file-data",
            Self::SessionData => "session-data",
            Self::KeyMaterial => "key-material",
        };

        format!("encryption-service:{domain}:dek:v{version}").into_bytes()
    }

    /// Get a human-readable description for logging/UI
    pub fn description(&self) -> &'static str {
        match self {
            Self::ApiKey => "API Key",
            Self::FileData => "File Data",
            Self::SessionData => "Session Data",
            Self::KeyMaterial => "Key Material",
        }
    }

    /// Compliance tags for audit logging
    pub fn compliance_tags(&self) -> Vec<&'static str> {
        match self {
            Self::ApiKey => vec!["SECRET"],
            _ => vec![],
        }
    }

    /// Recommended minimum key rotation period in days
    pub fn rotation_period_days(&self) -> u32 {
        match self {
            Self::SessionData => 7,   // Weekly
            Self::ApiKey => 90,       // Quarterly
            Self::KeyMaterial => 180, // Semi-annually
            _ => 365,                 // Default annual
        }
    }
}

impl EncryptedData {
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        bincode::encode_to_vec(self, bincode::config::standard())
    }

    pub fn to_base64(&self) -> Result<String, crate::EncryptionError> {
        let bytes = self.to_bytes().map_err(crate::EncryptionError::Bincode)?;
        Ok(BASE64_STANDARD.encode(bytes))
    }
}
