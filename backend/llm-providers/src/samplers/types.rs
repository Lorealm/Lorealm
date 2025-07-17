use serde::{Deserialize, Serialize};
use serde_json::Number;

use super::config::SamplerConfig;

#[derive(Serialize, Deserialize)]
pub enum SamplerValue {
    String(String),
    Int(i32),
    Float(f32),
    Bool(bool),
    Array(Vec<SamplerValue>)
}

impl From<String> for SamplerValue {
    fn from(value: String) -> Self {
        SamplerValue::String(value)
    }
}

impl From<i32> for SamplerValue {
    fn from(value: i32) -> Self {
        SamplerValue::Int(value)
    }
}

impl From<f32> for SamplerValue {
    fn from(value: f32) -> Self {
        SamplerValue::Float(value)
    }
}

impl From<bool> for SamplerValue {
    fn from(value: bool) -> Self {
        SamplerValue::Bool(value)
    }
}

impl From<Vec<SamplerValue>> for SamplerValue {
    fn from(value: Vec<SamplerValue>) -> Self {
        SamplerValue::Array(value)
    }
}

impl From<Vec<String>> for SamplerValue {
    fn from(value: Vec<String>) -> Self {
        SamplerValue::Array(value.into_iter().map(SamplerValue::from).collect())
    }
}

impl Into<serde_json::Value> for SamplerValue {
    fn into(self) -> serde_json::Value {
        match self {
            SamplerValue::String(value) => serde_json::Value::String(value),
            SamplerValue::Int(value) => serde_json::Value::Number(value.into()),
            SamplerValue::Float(value) => serde_json::Value::Number(Number::from_f64(value as f64).unwrap()),
            SamplerValue::Bool(value) => serde_json::Value::Bool(value),
            SamplerValue::Array(value) => serde_json::Value::Array(value.into_iter().map(Into::into).collect()),
        }
    }
}

pub trait Sampler: Send {
    fn name(&self) -> String;
    fn value(&self) -> SamplerValue;
}

pub trait SamplerCollection {
    fn samplers(config: &SamplerConfig) -> Vec<Box<dyn Sampler>>;
}