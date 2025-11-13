use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Custom type for handling position deserialization
#[derive(Debug, Clone)]
pub struct ContextPosition(Option<(i32, Option<i32>)>);

impl ContextPosition {
    pub fn get(&self) -> Option<(i32, Option<i32>)> {
        self.0
    }
}

// Implement FromStr for parsing the string input
impl FromStr for ContextPosition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Remove brackets and split by comma
        let cleaned = s.trim_matches(|c| c == '[' || c == ']').trim();
        if cleaned.is_empty() || cleaned.to_lowercase() == "none" {
            return Ok(ContextPosition(None));
        }

        let parts: Vec<&str> = cleaned.split(',').map(|s| s.trim()).collect();
        if parts.len() != 2 {
            return Err("Expected two values in position tuple".to_string());
        }

        let x = parts[0]
            .parse::<i32>()
            .map_err(|e| format!("Invalid x coordinate: {}", e))?;

        let y = match parts[1].to_lowercase().as_str() {
            "none" => None,
            s => Some(s.parse::<i32>()
                .map_err(|e| format!("Invalid y coordinate: {}", e))?),
        };

        Ok(ContextPosition(Some((x, y))))
    }
}

// Implement Deserialize for Position
impl<'de> Deserialize<'de> for ContextPosition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ContextPosition::from_str(&s).map_err(serde::de::Error::custom)
    }
}

// Implement Serialize for Position
impl Serialize for ContextPosition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.0 {
            None => serializer.serialize_str("none"),
            Some((x, None)) => serializer.serialize_str(&format!("[{}, none]", x)),
            Some((x, Some(y))) => serializer.serialize_str(&format!("[{}, {}]", x, y)),
        }
    }
}