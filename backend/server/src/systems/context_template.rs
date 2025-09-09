// context_template.rs
use std::{any::Any, fmt::{self, Debug}, usize};

use dyn_clone::DynClone;
use serde::{de::{self, Visitor}, Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// Important distinction, as message roles are not available in Text Completion
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ContextType {
    ChatCompletion,
    TextCompletion
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContextRole {
    System,
    Assistant,
    User
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "item")]
#[serde(rename_all = "camelCase")]
pub enum TemplateItem {
    Entry(TemplateEntry),
    Group(TemplateGroup),
}

impl TemplateItem {
    pub fn order(&self) -> usize {
        match self {
            TemplateItem::Entry(e) => e.order,
            TemplateItem::Group(g) => g.order,
        }
    }
}

/// Root level template.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextTemplate {
    pub id: Uuid,
    pub context_type: ContextType,
    pub name: String,
    pub description: Option<String>,
    pub items: Vec<TemplateItem>,
}

impl ContextTemplate {
    pub fn new(name: &str) -> Self {
        Self { context_type: ContextType::ChatCompletion,
            id: Uuid::new_v4(),
            name: name.to_owned(),
            description: None,
            items: Vec::new(),
        }
    }
    /// Sorts groups and entries (recursively) by their `order` field.
    pub fn sort(&mut self) {
        // Sort root items
        self.items.sort_by_key(|item| item.order());

        // Sort entries inside each group
        for item in &mut self.items {
            if let TemplateItem::Group(group) = item {
                group.entries.sort_by_key(|e| e.order);
            }
        }
    }

    pub fn reassign_orders(&mut self) {
        // Root items
        for (i, item) in self.items.iter_mut().enumerate() {
            match item {
                TemplateItem::Entry(entry) => entry.order = i,
                TemplateItem::Group(group) => group.order = i,
            }
        }

        // Entries inside each group
        for item in self.items.iter_mut() {
            if let TemplateItem::Group(group) = item {
                for (j, entry) in group.entries.iter_mut().enumerate() {
                    entry.order = j;
                }
            }
        }
    }
}

/// A group of template entries. Can be nested or used directly in the root
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemplateGroup {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub id: Uuid,
    /// The order of the group, relative to it's root
    pub order: usize,
    pub enabled: bool,
    pub entries: Vec<TemplateEntry>
}

impl TemplateGroup {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            description: None,
            color: None,
            id: Uuid::new_v4(),
            order: usize::MAX,
            entries: Vec::new(),
            enabled: true
        }
    }
}

/// A template entry. Can be within a group or directly in the root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateEntry {
    pub name: String,
    pub role: ContextRole,
    pub id: Uuid,
    pub order: usize,
    pub enabled: bool,
    pub content: Box<dyn ContextSource>
}

impl TemplateEntry {
    pub fn new(name: &str) -> Self {
        Self { 
            name: name.to_owned(),
            role: ContextRole::System,
            id: Uuid::new_v4(),
            content: "".into(),
            order: usize::MAX,
            enabled: true
        }
    }

    pub fn set_content<T: Into<Box<dyn ContextSource>>>(&mut self, content: T) {
        self.content = content.into();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("Content Resolution Error: {0}")]
    ContentResolution(String),
    #[error("Out of bounds")]
    OutOfBounds,
    #[error("Not Initialized")]
    NotInitialized,
    #[error("Not Found")]
    NotFound,
    #[error("Negative Index not allowed: {0}")]
    NegativeIndexNotAllowed(i32),
    #[error("Integer overflow found: {0}")]
    Overflow(i32),
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

// Wrapper for serializing/deserializing Box<dyn ContextSource>
#[derive(Serialize, Deserialize)]
struct ContextSourceWrapper {
    // Type identifier (e.g., "PlainTextSource")
    type_name: String,
    // The serialized data of the concrete type
    data: serde_json::Value,
}

#[async_trait::async_trait]
pub trait ContextSource: DynClone + Send + Sync + Debug {
    /// The content of the given source
    async fn content(&self) -> Result<String, ContextError>;
    fn source(&self) -> String;
    fn substitute_variables(&self) -> Result<String, ContextError> {
        todo!()
    }
    fn as_any(&self) -> &dyn Any;
}

impl Serialize for dyn ContextSource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Determine the concrete type and serialize it
        let wrapper = if self.source() == "PlainTextSource" {
            // Downcast to PlainTextSource (requires some way to access the concrete type)
            // For simplicity, assume `source()` returns a unique identifier
            let plain_text = self.as_any();
            let plain_text = plain_text
                .downcast_ref::<PlainTextSource>()
                .ok_or_else(|| serde::ser::Error::custom("Failed to downcast to PlainTextSource"))?;
            ContextSourceWrapper {
                type_name: "PlainTextSource".to_string(),
                data: serde_json::to_value(plain_text)
                    .map_err(serde::ser::Error::custom)?,
            }
        } else {
            return Err(serde::ser::Error::custom("Unknown ContextSource type"));
        };

        wrapper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Box<dyn ContextSource> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Define a visitor for deserialization
        struct ContextSourceVisitor;

        impl<'de> Visitor<'de> for ContextSourceVisitor {
            type Value = Box<dyn ContextSource>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a ContextSource trait object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut type_name = None;
                let mut data = None;

                // Read the type_name and data fields
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type_name" => type_name = Some(map.next_value::<String>()?),
                        "data" => data = Some(map.next_value::<serde_json::Value>()?),
                        _ => return Err(de::Error::unknown_field(&key, &["type_name", "data"])),
                    }
                }

                let type_name = type_name.ok_or_else(|| de::Error::missing_field("type_name"))?;
                let data = data.ok_or_else(|| de::Error::missing_field("data"))?;

                // Deserialize based on type_name
                match type_name.as_str() {
                    "PlainTextSource" => {
                        let plain_text: PlainTextSource =
                            serde_json::from_value(data).map_err(de::Error::custom)?;
                        Ok(Box::new(plain_text) as Box<dyn ContextSource>)
                    }
                    _ => Err(de::Error::custom(format!(
                        "Unknown type_name: {}",
                        type_name
                    ))),
                }
            }
        }

        deserializer.deserialize_map(ContextSourceVisitor)
    }
}

impl From<&str> for Box<dyn ContextSource> {
    fn from(value: &str) -> Self {
        Box::new(PlainTextSource::new(value))
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlainTextSource {
    text: String
}

impl PlainTextSource {
    pub fn new(text: &str) -> Self {
        Self { text: text.to_owned() }
    }
}

#[async_trait::async_trait]
impl ContextSource for PlainTextSource {
    async fn content(&self) -> Result<String, ContextError> {
        Ok(self.text.clone())
    }
    fn source(&self) -> String {
        String::from("eldoria.core.text")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

dyn_clone::clone_trait_object!(ContextSource);