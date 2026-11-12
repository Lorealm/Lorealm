use lorealm_storage::models::RealmResourceKind;
use serde::Serialize;
use weaver_lang::{InputDecl, InputType};

/// One `#inputs` declaration, projected for a client that has to render a
/// field for it.
#[derive(Debug, Serialize)]
pub struct InputSpec {
    pub name: String,
    /// The declaration as written, e.g. `[Ref<Character>]`.
    pub declared: String,
    pub required: bool,
    /// The `//` lines above the declaration: what the author says it is for.
    pub doc: Option<String>,
    #[serde(flatten)]
    pub ty: InputSpecType,
}

/// What someone building a realm from a blueprint is shown.
#[derive(Debug, Serialize)]
pub struct BuildForm {
    /// The author's markdown, shown before the fields.
    pub introduction: Option<String>,
    pub inputs: Vec<InputSpec>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputSpecType {
    String,
    Number,
    Bool,
    Enum {
        variants: Vec<String>,
    },
    /// A number inside an inclusive space: one slider handle.
    Range {
        min: f64,
        max: f64,
    },
    /// A `{from, to}` pair inside an inclusive space: two handles.
    Span {
        min: f64,
        max: f64,
    },
    List {
        of: Box<InputSpecType>,
    },
    /// Named fields, in declaration order — the order a form renders them in.
    Object {
        fields: Vec<InputField>,
    },
    /// An id the client picks and the server resolves.
    Reference {
        kind: Option<RealmResourceKind>,
        /// The kind as written in `Ref<...>`, such as a plugin's API key type.
        /// Not `name`, which the flattened input already has.
        ref_name: String,
    },
}

#[derive(Debug, Serialize)]
pub struct InputField {
    pub name: String,
    #[serde(flatten)]
    pub ty: InputSpecType,
}

impl InputSpec {
    pub fn of(declaration: &InputDecl) -> Self {
        Self {
            name: declaration.name.clone(),
            declared: declaration.ty.to_string(),
            required: declaration.is_required(),
            doc: declaration.doc.clone(),
            ty: InputSpecType::of(&declaration.ty),
        }
    }
}

impl InputSpecType {
    fn of(ty: &InputType) -> Self {
        match ty {
            InputType::String => Self::String,
            InputType::Number => Self::Number,
            InputType::Bool => Self::Bool,
            InputType::Enum(variants) => Self::Enum {
                variants: variants.clone(),
            },
            InputType::Range(min, max) => Self::Range {
                min: *min,
                max: *max,
            },
            InputType::Span(min, max) => Self::Span {
                min: *min,
                max: *max,
            },
            InputType::Object(fields) => Self::Object {
                fields: fields
                    .iter()
                    .map(|(name, ty)| InputField {
                        name: name.clone(),
                        ty: Self::of(ty),
                    })
                    .collect(),
            },
            InputType::List(inner) => Self::List {
                of: Box::new(Self::of(inner)),
            },
            InputType::Ref(kind) => Self::Reference {
                kind: RealmResourceKind::from_ref_name(kind),
                ref_name: kind.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reference_keeps_the_inputs_name() {
        let doc = weaver_lang::parse_value_doc("#inputs\nkey: Ref<elevenlabs>\n\n{}").unwrap();

        let spec = serde_json::to_value(InputSpec::of(&doc.inputs[0])).unwrap();

        assert_eq!(spec["name"], "key");
        assert_eq!(spec["ref_name"], "elevenlabs");
    }
}
