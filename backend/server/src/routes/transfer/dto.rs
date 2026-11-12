use lorealm_storage::models::Snowflake;
use serde::Serialize;

use crate::services::cards::import::AttachmentReport;
pub use crate::services::transfer::{Item, Kind, TRANSFER_VERSION, TransferFile};

#[derive(Serialize)]
pub struct ImportResponse {
    pub kind: Kind,
    pub id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<AttachmentReport>,
}
