use eldoria_storage::models::{ActivationState, Snowflake, StateDelta};
use serde::Serialize;
use std::collections::HashMap;

pub type StatePool = HashMap<String, HashMap<String, serde_json::Value>>;

#[derive(Serialize)]
pub struct ReconstructedState {
    pub persistent: StatePool,
    pub activation: HashMap<Snowflake, ActivationState>,
}

#[derive(Debug)]
pub struct CapturedState {
    pub activation: HashMap<Snowflake, ActivationState>,
    pub persistent: StateDelta,
}
