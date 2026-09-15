mod agent;
mod project;
mod service;
mod types;
mod validate;
mod weaver;
pub mod wtn;

pub use project::reject_defaulted_entities;
pub use service::RealmService;
pub use types::*;

/// How many of an agent's most recent observations feed one turn's prompt,
/// and the page size for the observation feed view.
const OBSERVATION_WINDOW: i64 = 200;
