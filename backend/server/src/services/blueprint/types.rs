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
    #[serde(flatten)]
    pub ty: InputSpecType,
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
    List {
        of: Box<InputSpecType>,
    },
    /// An id the client picks and the server resolves.
    Reference {
        kind: Option<RealmResourceKind>,
    },
}

impl InputSpec {
    pub fn of(declaration: &InputDecl) -> Self {
        Self {
            name: declaration.name.clone(),
            declared: declaration.ty.to_string(),
            required: declaration.is_required(),
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
            InputType::List(inner) => Self::List {
                of: Box::new(Self::of(inner)),
            },
            InputType::Ref(kind) => Self::Reference {
                kind: RealmResourceKind::from_ref_name(kind),
            },
        }
    }
}
