use chrono::{DateTime, Utc};
use lorealm_storage::models::{Role, Snowflake};
use serde::{Deserialize, Serialize};

use crate::services::permission::Permissions;

/// Permission bits travel as a decimal string
pub(crate) mod bits {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    use crate::services::permission::Permissions;

    pub fn serialize<S: Serializer>(p: &Permissions, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(&p.bits())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Permissions, D::Error> {
        let raw = String::deserialize(d)?;
        raw.parse::<i64>()
            .map(Permissions::from_db)
            .map_err(D::Error::custom)
    }

    pub mod option {
        use serde::{Deserialize, Deserializer};

        use crate::services::permission::Permissions;

        pub fn deserialize<'de, D: Deserializer<'de>>(
            d: D,
        ) -> Result<Option<Permissions>, D::Error> {
            #[derive(Deserialize)]
            struct Wrap(#[serde(with = "super")] Permissions);
            Ok(Option::<Wrap>::deserialize(d)?.map(|w| w.0))
        }
    }
}

#[derive(Serialize)]
pub struct RoleResponse {
    pub id: Snowflake,
    pub name: String,
    #[serde(with = "bits")]
    pub permissions: Permissions,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}

impl From<Role> for RoleResponse {
    fn from(role: Role) -> Self {
        Self {
            id: role.id,
            name: role.name,
            permissions: Permissions::from_db(role.permissions),
            position: role.position,
            created_at: role.created_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    #[serde(with = "bits")]
    pub permissions: Permissions,
    pub position: i32,
}

#[derive(Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "bits::option::deserialize")]
    pub permissions: Option<Permissions>,
    pub position: Option<i32>,
}

#[derive(Deserialize)]
pub struct AddMemberRequest {
    pub username: String,
}
