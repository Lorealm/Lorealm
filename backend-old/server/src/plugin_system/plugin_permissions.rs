use std::collections::HashSet;

use serde::{Deserialize, Serialize, Serializer};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum PluginPermission {
    Log(LogPermission),
    Context(ContextPermission),
    Http(HttpPermission),
    Component(ComponentPermission)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LogPermission {
    Read,
    Write,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContextPermission {
    Read,
    Write,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HttpPermission {
    Get,
    Post,
    // Other HTTP methods
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ComponentPermission {
    Register
}

impl PluginPermission {
    // Parse a permission string into the enum
    pub fn from_str(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        
        match (parts[0], parts[1]) {
            ("log", "read") => Some(PluginPermission::Log(LogPermission::Read)),
            ("log", "write") => Some(PluginPermission::Log(LogPermission::Write)),
            ("context", "read") => Some(PluginPermission::Context(ContextPermission::Read)),
            ("context", "write") => Some(PluginPermission::Context(ContextPermission::Write)),
            ("http", "get") => Some(PluginPermission::Http(HttpPermission::Get)),
            ("http", "post") => Some(PluginPermission::Http(HttpPermission::Post)),
            ("component", "register") => Some(PluginPermission::Component(ComponentPermission::Register)),
            _ => None,
        }
    }
    
    // Convert the enum back to string format
    pub fn to_str(&self) -> String {
        let value = match self {
            PluginPermission::Log(LogPermission::Read) => "log:read",
            PluginPermission::Log(LogPermission::Write) => "log:write",
            PluginPermission::Context(ContextPermission::Read) => "context:read",
            PluginPermission::Context(ContextPermission::Write) => "context:write",
            PluginPermission::Http(HttpPermission::Get) => "http:get",
            PluginPermission::Http(HttpPermission::Post) => "http:post",
            PluginPermission::Component(ComponentPermission::Register) => "component:register",
        };

        value.to_string()
    }
}

impl TryFrom<String> for PluginPermission {
    type Error = PluginPermissionError;
    
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s).ok_or(PluginPermissionError::InvalidFormat)
    }
}

impl From<PluginPermission> for String {
    fn from(permission: PluginPermission) -> String {
        permission.to_str()
    }
}

impl Serialize for PluginPermission {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_str())
    }
}

pub fn permissions_to_json(permissions: &[PluginPermission]) -> Value {
    // Convert vector to a HashSet for quick lookups
    let permissions_set: HashSet<&PluginPermission> = permissions.iter().collect();
    
    // Create the base structure with empty objects for each group
    let mut result = json!({
        "log": {},
        "context": {},
        "http": {}
    });
    
    // Fill in log permissions
    if permissions_set.contains(&PluginPermission::Log(LogPermission::Read)) {
        result["log"]["read"] = json!(false);
    }
    if permissions_set.contains(&PluginPermission::Log(LogPermission::Write)) {
        result["log"]["write"] = json!(false);
    }
    
    // Fill in context permissions
    if permissions_set.contains(&PluginPermission::Context(ContextPermission::Read)) {
        result["context"]["read"] = json!(false);
    }
    if permissions_set.contains(&PluginPermission::Context(ContextPermission::Write)) {
        result["context"]["write"] = json!(false);
    }
    
    // Fill in http permissions
    if permissions_set.contains(&PluginPermission::Http(HttpPermission::Get)) {
        result["http"]["get"] = json!(false);
    }
    if permissions_set.contains(&PluginPermission::Http(HttpPermission::Post)) {
        result["http"]["post"] = json!(false);
    }
    
    result
}

/// Error type for invalid plugin permission format
#[derive(Debug)]
pub enum PluginPermissionError {
    InvalidFormat,
    InsufficientImplicitPermission,
    InsufficientExplicitPermission,
    InvalidPlugin(String)
}

impl std::fmt::Display for PluginPermissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginPermissionError::InvalidFormat => write!(f, "Invalid plugin permission format"),
            PluginPermissionError::InsufficientImplicitPermission => write!(f, "This plugin is missing implicit permissions"),
            PluginPermissionError::InsufficientExplicitPermission => write!(f, "This plugin is missing explicit permissions"),
            PluginPermissionError::InvalidPlugin(plugin) => write!(f, "Invalid plugin: {}", plugin),
        }
    }
}