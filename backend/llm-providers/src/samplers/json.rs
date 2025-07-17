use std::collections::HashMap;

use serde_json::Value;

use super::types::Sampler;

pub type Payload = HashMap<String, Value>;

pub fn extend_payload(left: &mut Payload, right: &Payload) {
    for (key, value) in right {
        left.insert(key.clone(), value.clone());
    }
}

pub fn serialize_samplers(samplers: &[Box<dyn Sampler>]) -> Payload {
    let map: HashMap<String, Value> = samplers
        .iter()
        .map(|s| (s.name(), s.value().into()))
        .collect();

    map
}

pub fn serialize_samplers_to_string(samplers: &[Box<dyn Sampler>]) -> String {
    let value = serialize_samplers(samplers);
    serde_json::to_string_pretty(&value).unwrap()
}

pub trait ProviderPayload {
    fn create(&self) -> Payload;
    fn with_samplers(&mut self, samplers: &[Box<dyn Sampler>]) -> Payload {
        let mut payload = self.create();
        extend_payload(&mut payload, &serialize_samplers(samplers));
        payload
    }

    fn insert_option<T>(&self, payload: &mut Payload, key: &str, value: Option<T>)
    where
        T: Into<Value>,
    {
        if let Some(v) = value {
            payload.insert(key.to_string(), v.into());
        }
    }
}