//! Character export: gather a character and everything attached to it into a
//! [`CharacterBundle`], then render the bundle with an [`ExportProfile`].

use std::io::Cursor;

use bytes::Bytes;
use lorealm_storage::{
    Storage,
    models::{Character, Lorebook, Snowflake, UserTool, UserWidget},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::{
    extension::{
        EXTENSION_KEY, EXTENSION_VERSION, LorealmCardExtension, LorebookData, NativeFields,
        TextDialect, ToolData, WidgetData,
    },
    lorebook::{ConvertedBook, to_character_book},
    tokens::{sillytavern_to_weaver, weaver_to_sillytavern},
};
use crate::{
    errors::CharacterError,
    services::{
        character::{CharacterService, card_parser::embed_character_card_png},
        tools::builtin_tool,
    },
};

const PLACEHOLDER_PORTRAIT: &[u8] = include_bytes!("../../../assets/card-placeholder.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportProfile {
    /// SillyTavern-style text and `character_book`, readable by card sites.
    Compatibility,
    /// Lorealm's own text, unchanged.
    Lorealm,
}

impl ExportProfile {
    pub const ALL: [Self; 2] = [Self::Compatibility, Self::Lorealm];

    pub fn label(self) -> &'static str {
        match self {
            Self::Compatibility => "Compatibility",
            Self::Lorealm => "Lorealm",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Compatibility => {
                "Works in SillyTavern and on card sites like chub.ai. Lorealm-only features come back when the card is imported into Lorealm again, unless a site removed them."
            }
            Self::Lorealm => {
                "Keeps everything exactly as written. Other apps show Weaver code as plain text."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// The avatar with the card embedded, which card sites accept.
    #[default]
    Png,
    Json,
}

/// Something the export could not carry over, shown before downloading.
#[derive(Debug, Clone, Serialize)]
pub struct ExportWarning {
    pub code: &'static str,
    pub message: String,
}

pub struct ExportedCard {
    pub file_name: String,
    pub content_type: &'static str,
    pub bytes: Vec<u8>,
}

/// A character with the full content of everything attached to it.
pub struct CharacterBundle {
    pub character: Character,
    /// Main lorebook first.
    pub lorebooks: Vec<Lorebook>,
    /// Attached tools, then tools only widgets need (`false`).
    pub tools: Vec<(UserTool, bool)>,
    pub widgets: Vec<UserWidget>,
    pub avatar: Option<Bytes>,
}

impl CharacterBundle {
    pub async fn load(
        storage: &dyn Storage,
        characters: &CharacterService,
        character_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Self, CharacterError> {
        let version = characters.get_latest_version_id(character_id).await?;
        let character = characters
            .load_version(character_id, version, user_id)
            .await?;

        let mut lorebooks = Vec::new();
        for id in &character.lorebook_ids {
            if let Some(book) = storage.get_lorebook(*id).await?
                && book.owner_id == user_id
            {
                lorebooks.push(book);
            }
        }

        let mut widgets = Vec::new();
        for id in &character.widget_ids {
            if let Some(widget) = storage.get_widget(*id).await?
                && widget.user_id == user_id
            {
                widgets.push(widget);
            }
        }

        let mut tools: Vec<(UserTool, bool)> = Vec::new();
        let mut needed: Vec<(Snowflake, bool)> =
            character.tool_ids.iter().map(|id| (*id, true)).collect();
        for widget in &widgets {
            needed.extend(widget.tools.iter().map(|id| (*id, false)));
        }
        for (id, attached) in needed {
            if tools.iter().any(|(t, _)| t.id == id) {
                continue;
            }
            let tool = match builtin_tool(id) {
                Some(tool) => Some(tool),
                None => storage.get_tool(id).await?.filter(|t| t.user_id == user_id),
            };
            if let Some(tool) = tool {
                tools.push((tool, attached));
            }
        }

        let avatar = match character.metadata.avatar_uri.clone() {
            Some(uri) => characters.avatar_from_uri(Some(uri)).await.ok(),
            None => None,
        };

        Ok(Self {
            character,
            lorebooks,
            tools,
            widgets,
            avatar,
        })
    }
}

/// What the profile's text fields look like, and what didn't survive.
struct Composed {
    card_v3: Value,
    /// The same card for the legacy `chara` PNG chunk (Compatibility only).
    card_v2: Option<Value>,
    warnings: Vec<ExportWarning>,
}

pub fn warnings(
    bundle: &CharacterBundle,
    profile: ExportProfile,
    format: ExportFormat,
) -> Vec<ExportWarning> {
    let mut warnings = compose(bundle, profile).warnings;
    if format == ExportFormat::Png && portrait(bundle).1 {
        warnings.push(placeholder_warning());
    }
    warnings
}

pub fn render(
    bundle: &CharacterBundle,
    profile: ExportProfile,
    format: ExportFormat,
) -> Result<ExportedCard, CharacterError> {
    let composed = compose(bundle, profile);
    let stem = file_stem(&bundle.character.definition.name);

    match format {
        ExportFormat::Json => Ok(ExportedCard {
            file_name: format!("{stem}.json"),
            content_type: "application/json",
            bytes: serde_json::to_vec_pretty(&composed.card_v3)?,
        }),
        ExportFormat::Png => {
            let (png, _) = portrait(bundle);
            let mut png =
                embed_character_card_png(&png, &serde_json::to_vec(&composed.card_v3)?, "ccv3")?;
            if let Some(v2) = &composed.card_v2 {
                png = embed_character_card_png(&png, &serde_json::to_vec(v2)?, "chara")?;
            }
            Ok(ExportedCard {
                file_name: format!("{stem}.png"),
                content_type: "image/png",
                bytes: png,
            })
        }
    }
}

/// The avatar as a clean PNG (re-encoded, so no old card chunks ride along),
/// or the placeholder portrait. The flag says whether it's the placeholder.
fn portrait(bundle: &CharacterBundle) -> (Vec<u8>, bool) {
    let encoded = bundle.avatar.as_deref().and_then(|bytes| {
        let image = image::load_from_memory(bytes).ok()?;
        let mut out = Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).ok()?;
        Some(out.into_inner())
    });
    match encoded {
        Some(png) => (png, false),
        None => (PLACEHOLDER_PORTRAIT.to_vec(), true),
    }
}

/// What a lorebook lost on its way into a SillyTavern format.
pub fn lorebook_warnings(lorebook: &Lorebook, converted: &ConvertedBook) -> Vec<ExportWarning> {
    let mut warnings = Vec::new();
    if converted.non_portable_entries > 0 {
        warnings.push(ExportWarning {
            code: "weaver_lorebook_entries",
            message: format!(
                "{} entries in \"{}\" use Weaver features other platforms can't run.",
                converted.non_portable_entries, lorebook.name
            ),
        });
    }
    if converted.skipped > 0 {
        warnings.push(ExportWarning {
            code: "lorebook_entries_skipped",
            message: format!(
                "{} entries in \"{}\" couldn't be read and were left out of the SillyTavern lorebook.",
                converted.skipped, lorebook.name
            ),
        });
    }
    warnings
}

fn placeholder_warning() -> ExportWarning {
    ExportWarning {
        code: "placeholder_avatar",
        message: "This character has no usable avatar, so the card uses a placeholder image."
            .into(),
    }
}

fn file_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = stem.trim();
    if stem.is_empty() {
        "character".into()
    } else {
        stem.into()
    }
}

/// One text field in the profile's dialect. Records whether it could be
/// written exactly, and whether it relies on Weaver the dialect can't run.
struct FieldOut {
    text: String,
    portable: bool,
}

fn write_text(profile: ExportProfile, native: &str) -> FieldOut {
    match profile {
        ExportProfile::Lorealm => FieldOut {
            text: native.to_owned(),
            portable: true,
        },
        ExportProfile::Compatibility => {
            let converted = weaver_to_sillytavern(native);
            FieldOut {
                text: converted.text,
                portable: converted.portable,
            }
        }
    }
}

/// What the importer will make of a written field (see `import_text`).
fn read_text(profile: ExportProfile, written: &str) -> String {
    match profile {
        ExportProfile::Lorealm => written.to_owned(),
        ExportProfile::Compatibility => sillytavern_to_weaver(written),
    }
}

/// Greetings as CCv3 `first_mes` + `alternate_greetings`, read back the way
/// import does: the first message always becomes a greeting.
pub fn join_greetings(greetings: &[String]) -> (String, Vec<String>) {
    match greetings.split_first() {
        Some((first, rest)) => (first.clone(), rest.to_vec()),
        None => (String::new(), Vec::new()),
    }
}

pub fn split_greetings(first: &str, alternates: &[String]) -> Vec<String> {
    std::iter::once(first.to_owned())
        .chain(alternates.iter().cloned())
        .collect()
}

/// Examples as one `mes_example`, each opened by `<START>` (SillyTavern's
/// separator), read back by splitting on it and dropping blank pieces.
pub fn join_examples(examples: &[String]) -> String {
    examples.iter().map(|e| format!("<START>{e}")).collect()
}

pub fn split_examples(mes_example: &str) -> Vec<String> {
    mes_example
        .split("<START>")
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

fn compose(bundle: &CharacterBundle, profile: ExportProfile) -> Composed {
    let character = &bundle.character;
    let def = &character.definition;
    let meta = &character.metadata;
    let mut fields = NativeFields::default();
    let mut weaver_fields: Vec<&'static str> = Vec::new();

    // Scalar fields: written in the profile's dialect; the native text goes in
    // the extension whenever reading it back wouldn't give the same text.
    let mut scalar = |label: &'static str, native: &str, slot: &mut Option<String>| -> String {
        let out = write_text(profile, native);
        if !out.portable {
            weaver_fields.push(label);
        }

        // If a field does not match the native representation, we put it into the `fields` object.
        // This is later inserted into the `extensions` section of the CCv3 format if needed.
        // This is how we (theoretically... looking at you, chub) prevent data loss.
        if read_text(profile, &out.text) != native {
            *slot = Some(native.to_owned());
        }
        out.text
    };
    let description = scalar("Description", &def.description, &mut fields.description);
    let personality = scalar("Personality", &def.personality, &mut fields.personality);
    let scenario = scalar("Scenario", &def.scenario, &mut fields.scenario);
    let system_prompt = scalar(
        "System prompt",
        &def.system_prompt,
        &mut fields.system_prompt,
    );
    let post_history = scalar(
        "Post-history instructions",
        &def.post_history_instructions,
        &mut fields.post_history_instructions,
    );

    let mut greetings_native: Vec<&_> = def.greetings.iter().collect();
    greetings_native.sort_by_key(|g| g.order);
    let greetings_native: Vec<String> = greetings_native.iter().map(|g| g.text.clone()).collect();
    let greetings_out: Vec<FieldOut> = greetings_native
        .iter()
        .map(|g| write_text(profile, g))
        .collect();
    if greetings_out.iter().any(|g| !g.portable) {
        weaver_fields.push("Greetings");
    }
    let greetings_written: Vec<String> = greetings_out.into_iter().map(|g| g.text).collect();
    let (first_mes, alternate_greetings) = join_greetings(&greetings_written);
    let greetings_read: Vec<String> = split_greetings(&first_mes, &alternate_greetings)
        .iter()
        .map(|g| read_text(profile, g))
        .collect();
    if greetings_read != greetings_native {
        fields.greetings = Some(greetings_native);
    }

    let mut examples_native: Vec<&_> = def.example_messages.iter().collect();
    examples_native.sort_by_key(|e| e.order);
    let examples_native: Vec<String> = examples_native.iter().map(|e| e.content.clone()).collect();
    let examples_out: Vec<FieldOut> = examples_native
        .iter()
        .map(|e| write_text(profile, e))
        .collect();
    if examples_out.iter().any(|e| !e.portable) {
        weaver_fields.push("Example messages");
    }
    let mes_example = join_examples(&examples_out.into_iter().map(|e| e.text).collect::<Vec<_>>());
    let examples_read: Vec<String> = split_examples(&mes_example)
        .iter()
        .map(|e| read_text(profile, e))
        .collect();
    if examples_read != examples_native {
        fields.examples = Some(examples_native);
    }

    // Attachments, with tools referenced from widgets by hash.
    let tool_data: Vec<ToolData> = bundle
        .tools
        .iter()
        .map(|(tool, attached)| ToolData::from_tool(tool, *attached))
        .collect();
    let hash_of = |id: Snowflake| {
        bundle
            .tools
            .iter()
            .zip(&tool_data)
            .find(|((tool, _), _)| tool.id == id)
            .map(|(_, data)| data.hash.clone())
    };
    let widget_data: Vec<WidgetData> = bundle
        .widgets
        .iter()
        .map(|w| WidgetData::from_widget(w, w.tools.iter().filter_map(|id| hash_of(*id)).collect()))
        .collect();

    let extension = LorealmCardExtension {
        version: EXTENSION_VERSION,
        dialect: match profile {
            ExportProfile::Compatibility => TextDialect::Sillytavern,
            ExportProfile::Lorealm => TextDialect::Weaver,
        },
        display_name: meta.display_name.clone(),
        alt_names: (!meta.alt_names.is_empty()).then(|| meta.alt_names.clone()),
        template_injection: (!def.template_injection.is_empty())
            .then(|| def.template_injection.clone()),
        fields,
        lorebooks: bundle
            .lorebooks
            .iter()
            .map(LorebookData::from_lorebook)
            .collect(),
        tools: tool_data,
        widgets: widget_data,
    };

    // Keep whatever other apps stored on the card when it was imported.
    let mut extensions = match &character.extensions {
        Value::Object(map) => map.clone(),
        _ => Map::new(),
    };
    extensions.insert(
        EXTENSION_KEY.into(),
        serde_json::to_value(&extension).expect("the extension always serializes"),
    );

    let mut warnings = Vec::new();
    let mut data = json!({
        "name": def.name,
        "description": description,
        "personality": personality,
        "scenario": scenario,
        "first_mes": first_mes,
        "mes_example": mes_example,
        "creator_notes": meta.creator_notes,
        "system_prompt": system_prompt,
        "post_history_instructions": post_history,
        "alternate_greetings": alternate_greetings,
        "tags": meta.tags,
        "creator": meta.creator,
        "character_version": meta.card_version,
        "extensions": extensions,
    });

    if profile == ExportProfile::Compatibility {
        if !weaver_fields.is_empty() {
            warnings.push(ExportWarning {
                code: "weaver_fields",
                message: format!(
                    "\"{}\" uses Weaver features other platforms likely can't run. They're kept as written, and work again when the card is imported back into Lorealm.",
                    weaver_fields.join(", ")
                ),
            });
        }
        if let Some(main) = bundle.lorebooks.first() {
            let converted = to_character_book(main);
            warnings.extend(lorebook_warnings(main, &converted));
            data["character_book"] = converted.book;
        }
        let extra = bundle.lorebooks.len().saturating_sub(1);
        if extra > 0 {
            warnings.push(ExportWarning {
                code: "extra_lorebooks",
                message: format!(
                    "SillyTavern cards hold one lorebook. The other {extra} are added as extensions, which some sites remove."
                ),
            });
        }
        if !bundle.tools.is_empty() || !bundle.widgets.is_empty() {
            warnings.push(ExportWarning {
                code: "lorealm_only_attachments",
                message: "Tools and widgets only work in Lorealm, and are lost if a site removes Lorealm's data from the card.".into(),
            });
        }
    }

    let card_v2 = (profile == ExportProfile::Compatibility)
        .then(|| json!({ "spec": "chara_card_v2", "spec_version": "2.0", "data": data.clone() }));

    data["group_only_greetings"] = json!([]);
    data["creation_date"] = json!(meta.created_at.timestamp());
    data["modification_date"] = json!(meta.updated_at.timestamp());

    Composed {
        card_v3: json!({ "spec": "chara_card_v3", "spec_version": "3.0", "data": data }),
        card_v2,
        warnings,
    }
}
