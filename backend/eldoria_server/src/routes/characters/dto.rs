use serde::Serialize;

use crate::routes::characters::types::ImportResult;

#[derive(Serialize)]
pub struct ImportCharResponse {
    pub results: Vec<ImportResult>
}