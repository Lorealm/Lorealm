pub mod impls;
pub mod models;
pub mod service;
pub mod traits;

pub mod constants {
    pub const SUPPORTED_VERSION: u32 = 1;
    pub const MASTER_KEY_SIZE: usize = 32;
    pub const NONCE_SIZE: usize = 12;
    pub const TAG_SIZE: usize = 16;
    pub const SALT_SIZE: usize = 32;
    pub const DEK_SIZE: usize = 32;
    pub const CURRENT_VERSION: u32 = 1;
    pub const MAX_CIPHERTEXT_SIZE: usize = 10 * 1024 * 1024; // 10MB
}

pub mod backends;

// Re-export commonly used items for convenience
pub use models::{EncryptedData, EncryptionContext};
pub use traits::{Encryptable, EncryptionError, Hashable, KeyManager};

#[cfg(feature = "local")]
pub use backends::LocalKeyManager;

pub use impls::{
    decrypt_bytes, encrypt_bytes, hash_session_token, verify_password, verify_session_token,
};
