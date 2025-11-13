use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
}


#[derive(Debug, Serialize, Deserialize)]
/// Core settings for all providers. Should never be used directly
pub struct CoreSettings {
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop: Option<Vec<String>>,
}

/// Interface for mutating provider settings
pub trait SettingsMutator{
    /// Mutate an arbitrary field
    /// Will fail if the field does not exist, or the value cannot be parsed into a concrete type
    fn mutate_field(&mut self, field: &str, value: &str) -> Result<(), String>;

    fn try_parse_value(&self, value: &str, target_type: FieldType) -> Option<Value> {
        match target_type {
            FieldType::String => Some(Value::String(value.to_owned())),
            FieldType::Integer => value.parse::<i64>().ok().map(Value::from),
            FieldType::Float => value.parse::<f64>().ok().map(Value::from),
            FieldType::Boolean => value.parse::<bool>().ok().map(Value::from),
        }
    }
} 

impl CoreSettings {
    pub fn new(model: String) -> Self {
        Self {
            model,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop: None,
        }
    }

    pub fn set_max_tokens(&mut self, max_tokens: u32) {
        self.max_tokens = Some(max_tokens);
    }

    pub fn set_temperature(&mut self, temperature: f32) {
        self.temperature = Some(temperature);
    }

    pub fn set_top_p(&mut self, top_p: f32) {
        self.top_p = Some(top_p);
    }

    pub fn set_stop(&mut self, stop: Vec<String>) {
        self.stop = Some(stop);
    }
}

impl SettingsMutator for CoreSettings {
    fn mutate_field(&mut self, field: &str, value: &str) -> Result<(), String> {
        match field {
            "model" => {
                self.model = value.to_string();
                Ok(())
            }
            "max_tokens" => {
                let parsed = self.try_parse_value(value, FieldType::Integer)
                    .and_then(|v| v.as_i64())
                    .ok_or("Invalid integer")?;
                self.max_tokens = Some(parsed as u32);
                Ok(())
            }
            "temperature" => {
                let parsed = self.try_parse_value(value, FieldType::Float)
                    .and_then(|v| v.as_f64())
                    .ok_or("Invalid float")?;
                self.temperature = Some(parsed as f32);
                Ok(())
            }
            "top_p" => {
                let parsed = self.try_parse_value(value, FieldType::Float)
                    .and_then(|v| v.as_f64())
                    .ok_or("Invalid float")?;
                self.top_p = Some(parsed as f32);
                Ok(())
            }
            _ => Err(format!("Unknown field: {}", field)),
        }
    }
}
