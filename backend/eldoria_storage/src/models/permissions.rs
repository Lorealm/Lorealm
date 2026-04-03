use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::prelude::Snowflake;

// =============================================================================
// Permission bitfield
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permissions(pub i64);

impl fmt::Display for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Permissions({:#018x})", self.0)
    }
}

// =============================================================================
// Role
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Role {
    pub id: Snowflake,
    pub name: String,
    pub permissions: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateRole {
    pub id: Snowflake,
    pub name: String,
    pub permissions: i64,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateRole {
    pub name: Option<String>,
    pub permissions: Option<i64>,
}

// =============================================================================
// User ↔ Role mapping
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserRole {
    pub user_id: Snowflake,
    pub role_id: Snowflake,
    pub assigned_at: DateTime<Utc>,
}
