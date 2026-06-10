use std::{
    fmt::Display,
    num::ParseIntError,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

/// Snowflake ID — a 64-bit unique identifier encoding timestamp + worker + sequence.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Hash, Ord, sqlx::Type)]
#[sqlx(transparent)]
pub struct Snowflake(pub i64);

impl std::fmt::Display for Snowflake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for Snowflake {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Snowflake {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // Accept both string and number from incoming JSON
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Snowflake;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a snowflake as string or integer")
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Snowflake, E> {
                Ok(Snowflake(v))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Snowflake, E> {
                Ok(Snowflake(v as i64))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Snowflake, E> {
                v.parse::<i64>().map(Snowflake).map_err(E::custom)
            }
        }
        d.deserialize_any(Visitor)
    }
}

impl FromStr for Snowflake {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Snowflake(s.parse::<i64>()?))
    }
}

/// Content address of a stored blob. NOT a snowflake — identity *is* the bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlobId(String); // lowercase hex of the sha256 digest

impl BlobId {
    /// Hash content. Identical bytes always yield the same id.
    pub fn from_content(bytes: &[u8]) -> Self {
        Self(hex::encode(Sha256::digest(bytes)))
    }

    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// The string you persist in `character_versions.avatar_uri`.
    pub fn to_uri(&self) -> String {
        format!("sha256:{}", self.0)
    }

    pub fn from_uri(uri: &str) -> Option<Self> {
        uri.strip_prefix("sha256:").map(|hex| Self(hex.to_string()))
    }

    /// Relative location under the blobs root: `<ab>/<full-hex>`.
    pub fn relative_path(&self) -> PathBuf {
        Path::new(&self.0[..2]).join(&self.0)
    }
}

impl Display for BlobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Specifies how functions requiring versioning should be saved.
/// 
/// This is used for providing transaction-based behavior so every minute change
/// don't result in a new version.
pub enum VersionedSaveMode {
    /// Append a new version. Caller has set a fresh `current_version_id`.
    NewVersion,
    /// Update the existing version identified by `current_version_id`.
    UpdateCurrent,
}
