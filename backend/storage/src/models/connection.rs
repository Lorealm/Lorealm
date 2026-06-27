use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::models::Snowflake;

/// An encrypted secret (currently always a provider API key). The encrypted
/// payload columns mirror `EncryptedData` from the encryption crate field
/// for field; conversion between the two lives in the service layer so the
/// storage crate stays free of the encryption dependency.
///
/// NOTE: no `Serialize` derive, deliberately — this struct must never travel
/// through an API response. `SecretMetadata` is the serializable face.
#[derive(Debug, FromRow, Clone)]
pub struct Secret {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub provider: String,
    pub label: String,
    /// Last few characters of the plaintext key, for display only.
    pub key_hint: String,
    /// Master key version the payload was encrypted under.
    pub enc_version: i32,
    /// Encryption context name, e.g. "api_key".
    pub context: String,
    pub salt: Vec<u8>,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub tag: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow, Clone)]
pub struct SecretMetadata {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub provider: String,
    pub label: String,
    pub key_hint: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A connection profile: a named (provider, model, parameters) bundle
/// pointing at the secret used to authenticate. Parameter columns are JSONB
/// and deserialize into the provider crate's typed structs in the service
/// layer (strict on write, lenient on read).
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ConnectionProfile {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub secret_id: Option<Snowflake>,
    pub name: String,
    pub provider: String,
    pub model: Option<String>,
    pub token_budget: i32,
    pub common_params: serde_json::Value,
    pub provider_params: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
