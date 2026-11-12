//! Blueprint bundles. A blueprint consists of the source WTN document and
//! opaque imports. When exporting, a blueprint is converted to a zip file,
//! with a custom `.lrlm` extension.
//!
//! ```text
//! manifest.json          what the archive holds
//! blueprint.wtn          the source, ids as the author had them
//! characters/<id>.png    a character card, attachments embedded
//! personas/<id>.json     a transfer file, plus personas/<id>.<ext> for the avatar
//! <kind>s/<id>.json      a transfer file
//! ```

use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Read, Write},
};

use bytes::Bytes;
use lorealm_storage::models::{Blueprint, RealmResourceKind, Snowflake};
use serde::{Deserialize, Serialize};
use weaver_lang::{
    Span, ValueDoc,
    ast::{
        expr::{ArrayItem, Expr, ExprKind, ObjectItem},
        value::Value,
    },
};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

use super::BlueprintService;
use crate::{
    errors::{ContextError, DomainError, ModuleError, RealmError, TargetType},
    services::{
        cards::{
            export::{self, CharacterBundle, ExportFormat, ExportProfile, file_stem},
            extension::{LorealmCardExtension, LorebookData, ToolData, WidgetData},
            import::{AttachmentReport, ImportOptions, import_attachments},
            lorebook::lorebook_from_data,
        },
        character::card_parser::{CARD_IDENTIFIERS, parse_character_card_png},
        context_template::ContextTemplate,
        persona::UpdatePersona,
        realm::wtn,
        tools::builtin_tool,
        transfer::{Item, TRANSFER_VERSION, TransferFile, fresh_ids},
    },
};

pub const BUNDLE_VERSION: u32 = 1;
pub const BUNDLE_EXTENSION: &str = "lrlm";
pub const BUNDLE_CONTENT_TYPE: &str = "application/zip";

const MANIFEST_FILE: &str = "manifest.json";
const SOURCE_FILE: &str = "blueprint.wtn";
/// TODO: branch into settings?
const MAX_UNPACKED: u64 = 512 << 20;
const MAX_RESOURCES: usize = 1024;
const AVATAR_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif"];

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub lorealm: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub resources: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub kind: RealmResourceKind,
    pub id: Snowflake,
    pub name: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

pub struct ExportedBundle {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct BundlePreview {
    pub name: String,
    pub description: String,
    pub resources: Vec<ResourcePreview>,
}

#[derive(Debug, Serialize)]
pub struct ResourcePreview {
    pub kind: RealmResourceKind,
    pub name: String,
    pub status: ResourceStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    pub includes: Vec<IncludedResource>,
    pub has_image: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceStatus {
    New,
    Existing,
    Blocked,
}

#[derive(Debug, Serialize)]
pub struct IncludedResource {
    pub kind: RealmResourceKind,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ImportedBundle {
    pub blueprint: Blueprint,
    pub resources: Vec<ImportedResource>,
    /// Tools saved under a new name: `(original name, new name)`.
    pub renamed: Vec<(String, String)>,
    /// Attachments that were invalid and left out.
    pub skipped: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportedResource {
    pub kind: RealmResourceKind,
    pub name: String,
    pub id: Snowflake,
    /// An identical resource was already in the library and was used.
    pub reused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticRef {
    pub kind: RealmResourceKind,
    pub id: Snowflake,
    pub span: Span,
}

/// Where in the document the walk is.
#[derive(Clone, Copy)]
enum Place {
    Document,
    Cast,
    Tools,
    Ids(RealmResourceKind),
    Elsewhere,
}

/// Every id the document writes out in source order.
pub fn static_refs(doc: &ValueDoc) -> Vec<StaticRef> {
    let mut found = Vec::new();
    walk_expression(&doc.value, Place::Document, &mut found);
    found.sort_by_key(|r| r.span.start);
    found
}

fn walk_expression(expr: &Expr, place: Place, found: &mut Vec<StaticRef>) {
    match (&expr.node, place) {
        (_, Place::Elsewhere) => {}
        (ExprKind::Literal(Value::String(text)), Place::Ids(kind)) => {
            if let Ok(id) = text.parse::<i64>() {
                found.push(StaticRef {
                    kind,
                    id: Snowflake(id),
                    span: expr.span,
                });
            }
        }
        (ExprKind::ArrayLiteral(items), _) => {
            for item in items {
                walk_element(item, place, found);
            }
        }
        (ExprKind::ObjectLiteral(items), Place::Document | Place::Cast | Place::Tools) => {
            for item in items {
                walk_entry(item, place, found);
            }
        }
        _ => {}
    }
}

fn walk_element(item: &ArrayItem, place: Place, found: &mut Vec<StaticRef>) {
    match item {
        ArrayItem::Element(expr) => walk_expression(expr, place, found),
        ArrayItem::ForEach(each) => {
            for item in &each.body {
                walk_element(item, place, found);
            }
        }
        ArrayItem::If(cond) => {
            let bodies = cond.branches.iter().map(|(_, body)| body);
            for item in bodies.chain(&cond.else_body).flatten() {
                walk_element(item, place, found);
            }
        }
    }
}

fn walk_entry(item: &ObjectItem, place: Place, found: &mut Vec<StaticRef>) {
    match item {
        ObjectItem::Entry(entry) => {
            use RealmResourceKind as Kind;
            let inside = match (place, entry.static_key()) {
                (Place::Document, Some("agents")) => Place::Cast,
                (Place::Document, Some("modules")) => Place::Ids(Kind::Module),
                (Place::Cast, Some("character_id")) => Place::Ids(Kind::Character),
                (Place::Cast, Some("template_id")) => Place::Ids(Kind::Template),
                (Place::Cast, Some("persona_id")) => Place::Ids(Kind::Persona),
                (Place::Cast, Some("profile_id")) => Place::Ids(Kind::ConnectionProfile),
                (Place::Cast, Some("lorebooks")) => Place::Ids(Kind::Lorebook),
                (Place::Cast, Some("widgets")) => Place::Ids(Kind::Widget),
                (Place::Cast, Some("tools")) => Place::Tools,
                (Place::Tools, Some("tool_id")) => Place::Ids(Kind::Tool),
                _ => Place::Elsewhere,
            };
            walk_expression(&entry.value, inside, found);
        }
        ObjectItem::ForEach(each) => {
            for item in &each.body {
                walk_entry(item, place, found);
            }
        }
        ObjectItem::If(cond) => {
            let bodies = cond.branches.iter().map(|(_, body)| body);
            for item in bodies.chain(&cond.else_body).flatten() {
                walk_entry(item, place, found);
            }
        }
    }
}

pub fn rewrite(
    source: &str,
    refs: &[StaticRef],
    ids: &HashMap<(RealmResourceKind, Snowflake), Snowflake>,
) -> String {
    let mut out = String::with_capacity(source.len());
    let mut at = 0;
    for r in refs {
        let Some(new) = ids.get(&(r.kind, r.id)) else {
            continue;
        };
        out.push_str(&source[at..r.span.start]);
        out.push_str(&format!("r\"{}\"", new.0));
        at = r.span.end;
    }
    out.push_str(&source[at..]);
    out
}

fn folder(kind: RealmResourceKind) -> &'static str {
    match kind {
        RealmResourceKind::Character => "characters",
        RealmResourceKind::Lorebook => "lorebooks",
        RealmResourceKind::Template => "templates",
        RealmResourceKind::Tool => "tools",
        RealmResourceKind::Persona => "personas",
        RealmResourceKind::ConnectionProfile => "profiles",
        RealmResourceKind::Widget => "widgets",
        RealmResourceKind::Module => "modules",
    }
}

fn line_number_from_span(source: &str, offset: usize) -> usize {
    source.as_bytes()[..offset.min(source.len())]
        .iter()
        .filter(|b| **b == b'\n')
        .count()
        + 1
}

fn invalid(why: impl Into<String>) -> RealmError {
    RealmError::InvalidBundle(why.into())
}

/// One resource, ready to be written into an archive.
struct PackedResource {
    name: String,
    path: String,
    bytes: Vec<u8>,
    image: Option<(String, Vec<u8>)>,
}

fn pack(manifest: &Manifest, source: &str, files: Vec<(String, Vec<u8>)>) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    let manifest = serde_json::to_vec_pretty(manifest).expect("a manifest serializes");
    let text = [
        (MANIFEST_FILE.to_string(), manifest),
        (SOURCE_FILE.to_string(), source.as_bytes().to_vec()),
    ];
    for (path, bytes) in text.into_iter().chain(files) {
        // Images are compressed already.
        let options = if path.ends_with(".json") || path.ends_with(".wtn") {
            deflated
        } else {
            stored
        };
        zip.start_file(path, options).expect("an entry starts");
        zip.write_all(&bytes).expect("an entry is written");
    }
    zip.finish().expect("an archive finishes").into_inner()
}

/// An unpacked Blueprint bundle
struct UnpackedBundle {
    manifest: Manifest,
    source: String,
    files: HashMap<String, Vec<u8>>,
}

fn unpack(bytes: &[u8]) -> Result<UnpackedBundle, RealmError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| invalid("This file isn't a blueprint bundle"))?;
    let mut budget = MAX_UNPACKED;

    let manifest = read_entry(&mut archive, MANIFEST_FILE, &mut budget)?;
    let manifest: Manifest = serde_json::from_slice(&manifest)
        .map_err(|_| invalid("This bundle's manifest can't be read"))?;
    if manifest.lorealm > BUNDLE_VERSION {
        return Err(invalid(
            "This bundle was made by a newer version of Lorealm. Update to import it",
        ));
    }
    if manifest.resources.len() > MAX_RESOURCES {
        return Err(invalid("This bundle holds too many resources"));
    }

    let source = read_entry(&mut archive, SOURCE_FILE, &mut budget)?;
    let source =
        String::from_utf8(source).map_err(|_| invalid("This bundle's blueprint isn't text"))?;

    let mut files = HashMap::new();
    let paths = manifest
        .resources
        .iter()
        .flat_map(|r| std::iter::once(&r.path).chain(&r.image));
    for path in paths {
        if !files.contains_key(path) {
            let bytes = read_entry(&mut archive, path, &mut budget)?;
            files.insert(path.clone(), bytes);
        }
    }

    Ok(UnpackedBundle {
        manifest,
        source,
        files,
    })
}

fn read_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    budget: &mut u64,
) -> Result<Vec<u8>, RealmError> {
    let file = archive
        .by_name(name)
        .map_err(|_| invalid(format!("This bundle is missing `{name}`")))?;
    let mut bytes = Vec::new();
    file.take(*budget + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid(format!("`{name}` in this bundle is damaged")))?;
    if bytes.len() as u64 > *budget {
        return Err(invalid(format!(
            "This bundle unpacks to over {} MiB",
            MAX_UNPACKED >> 20
        )));
    }
    *budget -= bytes.len() as u64;
    Ok(bytes)
}

enum Payload {
    Card {
        name: String,
        extension: Box<LorealmCardExtension>,
    },
    Item(Item),
}

fn parse_payload(entry: &ManifestEntry, bytes: &[u8]) -> Result<Payload, RealmError> {
    let unreadable = || invalid(format!("`{}` in this bundle can't be read", entry.path));

    if entry.kind == RealmResourceKind::Character {
        let json = if entry.path.ends_with(".json") {
            bytes.to_vec()
        } else {
            parse_character_card_png(bytes, CARD_IDENTIFIERS)
                .map_err(|_| unreadable())?
                .json_bytes
        };
        let card: serde_json::Value = serde_json::from_slice(&json).map_err(|_| unreadable())?;
        let name = card
            .pointer("/data/name")
            .and_then(|n| n.as_str())
            .unwrap_or_default()
            .to_string();
        let extension = card
            .pointer("/data/extensions/lorealm")
            .and_then(|e| serde_json::from_value(e.clone()).ok())
            .unwrap_or_default();
        return Ok(Payload::Card {
            name,
            extension: Box::new(extension),
        });
    }

    let file: TransferFile = serde_json::from_slice(bytes).map_err(|_| unreadable())?;
    let matches = matches!(
        (entry.kind, &file.item),
        (RealmResourceKind::Template, Item::Template { .. })
            | (RealmResourceKind::Persona, Item::Persona { .. })
            | (RealmResourceKind::Tool, Item::Tool { .. })
            | (RealmResourceKind::Widget, Item::Widget { .. })
            | (RealmResourceKind::Lorebook, Item::Lorebook { .. })
            | (RealmResourceKind::Module, Item::Module { .. })
    );
    let whole = match &file.item {
        Item::Tool { tools } => tools.len() == 1,
        Item::Widget { widgets, .. } => widgets.len() == 1,
        _ => true,
    };
    if !matches || !whole {
        return Err(invalid(format!(
            "`{}` in this bundle isn't the {} its manifest says it is",
            entry.path,
            entry.kind.as_str()
        )));
    }
    Ok(Payload::Item(file.item))
}

impl Payload {
    fn name(&self) -> &str {
        match self {
            Self::Card { name, .. } => name,
            Self::Item(Item::Template { name, .. })
            | Self::Item(Item::Persona { name, .. })
            | Self::Item(Item::Module { name, .. }) => name,
            Self::Item(Item::Tool { tools }) => &tools[0].name,
            Self::Item(Item::Widget { widgets, .. }) => &widgets[0].name,
            Self::Item(Item::Lorebook { lorebook }) => &lorebook.name,
        }
    }

    fn includes(&self) -> Vec<IncludedResource> {
        let named = |kind, name: &String| IncludedResource {
            kind,
            name: name.clone(),
        };
        match self {
            Self::Card { extension, .. } => {
                let lorebooks = extension
                    .lorebooks
                    .iter()
                    .map(|l| named(RealmResourceKind::Lorebook, &l.name));
                let tools = extension
                    .tools
                    .iter()
                    .map(|t| named(RealmResourceKind::Tool, &t.name));
                let widgets = extension
                    .widgets
                    .iter()
                    .map(|w| named(RealmResourceKind::Widget, &w.name));
                lorebooks.chain(tools).chain(widgets).collect()
            }
            Self::Item(Item::Widget { tools, .. }) => tools
                .iter()
                .map(|t| named(RealmResourceKind::Tool, &t.name))
                .collect(),
            Self::Item(_) => Vec::new(),
        }
    }
}

fn to_file(item: Item) -> Result<Vec<u8>, RealmError> {
    Ok(serde_json::to_vec_pretty(&TransferFile {
        lorealm: TRANSFER_VERSION,
        item,
    })?)
}

impl BlueprintService {
    /// The blueprint and everything its source names by id, as one archive.
    pub async fn export_bundle(
        &self,
        id: Snowflake,
        user_id: Snowflake,
    ) -> Result<ExportedBundle, RealmError> {
        let blueprint = self.get(id, user_id).await?;
        let doc = wtn::compile(&blueprint.source)?;
        let refs = static_refs(doc.document());

        if let Some(profile) = refs
            .iter()
            .find(|r| r.kind == RealmResourceKind::ConnectionProfile)
        {
            return Err(RealmError::NotExportable(format!(
                "Line {} uses one of your connection profiles directly. Connection profiles \
                 hold API keys, so they are never exported.\n\nConsider replacing it with a \
                 Ref<ConnectionProfile> input, which lets users pick their own when building \
                 this blueprint.",
                line_number_from_span(&blueprint.source, profile.span.start)
            )));
        }

        let mut seen = HashSet::new();
        let mut resources = Vec::new();
        let mut files = Vec::new();
        for r in &refs {
            let builtin = r.kind == RealmResourceKind::Tool && builtin_tool(r.id).is_some();
            if builtin || !seen.insert((r.kind, r.id)) {
                continue;
            }
            let packed = match self.pack_resource(user_id, r.kind, r.id).await {
                Ok(packed) => packed,
                Err(e) if e.status() == axum::http::StatusCode::NOT_FOUND => {
                    return Err(RealmError::NotExportable(format!(
                        "Line {} uses a {} that no longer exists. Pick another one or clear \
                         the field, then export again",
                        line_number_from_span(&blueprint.source, r.span.start),
                        r.kind.as_str()
                    )));
                }
                Err(e) => return Err(e),
            };
            resources.push(ManifestEntry {
                kind: r.kind,
                id: r.id,
                name: packed.name,
                path: packed.path.clone(),
                image: packed.image.as_ref().map(|(path, _)| path.clone()),
            });
            files.push((packed.path, packed.bytes));
            files.extend(packed.image);
        }

        let manifest = Manifest {
            lorealm: BUNDLE_VERSION,
            name: blueprint.name.clone(),
            description: blueprint.description.clone(),
            resources,
        };
        Ok(ExportedBundle {
            file_name: format!(
                "{}.{BUNDLE_EXTENSION}",
                file_stem(&blueprint.name, "blueprint")
            ),
            bytes: pack(&manifest, &blueprint.source, files),
        })
    }

    async fn pack_resource(
        &self,
        user_id: Snowflake,
        kind: RealmResourceKind,
        id: Snowflake,
    ) -> Result<PackedResource, RealmError> {
        let dir = folder(kind);
        let json = format!("{dir}/{}.json", id.0);
        let mut image = None;

        let (name, item) = match kind {
            RealmResourceKind::Character => {
                self.storage
                    .get_character(id)
                    .await?
                    .filter(|c| c.user_id == user_id)
                    .ok_or(RealmError::NotFound(TargetType::Character, id))?;
                let bundle =
                    CharacterBundle::load(self.storage.as_ref(), &self.characters, id, user_id)
                        .await?;
                let format = match bundle.avatar {
                    Some(_) => ExportFormat::Png,
                    None => ExportFormat::Json,
                };
                let extension = match format {
                    ExportFormat::Png => "png",
                    ExportFormat::Json => "json",
                };
                let card = export::render(&bundle, ExportProfile::Lorealm, format)?;
                return Ok(PackedResource {
                    name: bundle.character.definition.name.clone(),
                    path: format!("{dir}/{}.{extension}", id.0),
                    bytes: card.bytes,
                    image: None,
                });
            }
            RealmResourceKind::Template => {
                let t = self.templates.get_template(id, user_id).await?;
                (
                    t.name.clone(),
                    Item::Template {
                        name: t.name,
                        description: t.description,
                        nodes: t.nodes,
                    },
                )
            }
            RealmResourceKind::Persona => {
                let p = self.personas.get_persona(id, user_id).await?;
                if let Ok(avatar) = self.personas.avatar_from_uri(p.avatar_uri.clone()).await
                    && let Some(kind) = infer::get(&avatar)
                    && AVATAR_TYPES.contains(&kind.mime_type())
                {
                    image = Some((
                        format!("{dir}/{}.{}", id.0, kind.extension()),
                        avatar.to_vec(),
                    ));
                }
                (
                    p.name.clone(),
                    Item::Persona {
                        name: p.name,
                        display_name: p.display_name,
                        pseudonym: p.pseudonym,
                        description: p.description,
                    },
                )
            }
            RealmResourceKind::Tool => {
                let tool = self
                    .storage
                    .get_tool(id)
                    .await?
                    .filter(|t| t.user_id == user_id)
                    .ok_or(RealmError::NotFound(TargetType::UserTool, id))?;
                (
                    tool.name.clone(),
                    Item::Tool {
                        tools: vec![ToolData::from_tool(&tool, true)],
                    },
                )
            }
            RealmResourceKind::Widget => {
                let widget = self
                    .storage
                    .get_widget(id)
                    .await?
                    .filter(|w| w.user_id == user_id)
                    .ok_or(RealmError::NotFound(TargetType::UserWidget, id))?;
                let mut tools = Vec::new();
                for tool_id in &widget.tools {
                    let tool = match builtin_tool(*tool_id) {
                        Some(tool) => Some(tool),
                        // A tool deleted since it was attached is left out.
                        None => self
                            .storage
                            .get_tool(*tool_id)
                            .await?
                            .filter(|t| t.user_id == user_id),
                    };
                    if let Some(tool) = tool {
                        tools.push(ToolData::from_tool(&tool, false));
                    }
                }
                let hashes = tools.iter().map(|t| t.hash.clone()).collect();
                (
                    widget.name.clone(),
                    Item::Widget {
                        widgets: vec![WidgetData::from_widget(&widget, hashes)],
                        tools,
                    },
                )
            }
            RealmResourceKind::Lorebook => {
                let lorebook = self
                    .storage
                    .get_lorebook(id)
                    .await?
                    .filter(|l| l.owner_id == user_id)
                    .ok_or(RealmError::NotFound(TargetType::Lorebook, id))?;
                (
                    lorebook.name.clone(),
                    Item::Lorebook {
                        lorebook: LorebookData::from_lorebook(&lorebook),
                    },
                )
            }
            RealmResourceKind::Module => {
                let module = self
                    .modules
                    .get(id, user_id)
                    .await
                    .map_err(|_| RealmError::NotFound(TargetType::Module, id))?;
                (
                    module.name.clone(),
                    Item::Module {
                        name: module.name,
                        description: module.description,
                        source: module.source,
                    },
                )
            }
            RealmResourceKind::ConnectionProfile => {
                return Err(RealmError::NotExportable(
                    "Connection profiles hold API keys, so they are never exported".into(),
                ));
            }
        };

        Ok(PackedResource {
            name,
            path: json,
            bytes: to_file(item)?,
            image,
        })
    }

    pub async fn preview_bundle(
        &self,
        user_id: Snowflake,
        bytes: &[u8],
    ) -> Result<BundlePreview, RealmError> {
        let unpacked = unpack(bytes)?;
        Self::check(&unpacked.source)?;
        let payloads = payloads(&unpacked)?;
        self.inspect(user_id, &unpacked.manifest, &payloads).await
    }

    async fn inspect(
        &self,
        user_id: Snowflake,
        manifest: &Manifest,
        payloads: &[Payload],
    ) -> Result<BundlePreview, RealmError> {
        let modules = self.storage.list_modules(user_id).await?;
        let tools: HashSet<String> = self
            .storage
            .list_tools(user_id)
            .await?
            .iter()
            .map(|t| ToolData::from_tool(t, false).hash)
            .collect();

        let mut resources = Vec::new();
        for (entry, payload) in manifest.resources.iter().zip(payloads) {
            let mut status = ResourceStatus::New;
            let mut problem = None;

            match payload {
                Payload::Item(Item::Module { name, source, .. }) => {
                    match modules.iter().find(|m| &m.name == name) {
                        Some(mine) if &mine.source == source => status = ResourceStatus::Existing,
                        Some(_) => {
                            problem = Some(format!(
                                "You already have a different module called \"{name}\". \
                                 Rename or remove yours to import this one."
                            ));
                        }
                        None => match self.modules.check_name(user_id, None, name).await {
                            Ok(()) => {}
                            Err(ModuleError::Storage(e)) => return Err(e.into()),
                            Err(_) => {
                                problem = Some(format!(
                                    "The module name \"{name}\" is already used by a plugin \
                                     or by Lorealm itself."
                                ));
                            }
                        },
                    }
                }
                Payload::Item(Item::Tool { tools: data })
                    if tools.contains(&data[0].content_hash()) =>
                {
                    status = ResourceStatus::Existing;
                }
                _ => {}
            }
            if problem.is_some() {
                status = ResourceStatus::Blocked;
            }

            resources.push(ResourcePreview {
                kind: entry.kind,
                name: payload.name().to_string(),
                status,
                problem,
                includes: payload.includes(),
                has_image: entry.image.is_some()
                    || (entry.kind == RealmResourceKind::Character
                        && !entry.path.ends_with(".json")),
            });
        }

        Ok(BundlePreview {
            name: manifest.name.clone(),
            description: manifest.description.clone(),
            resources,
        })
    }

    /// Import a bundle, along with all the bundled resources.
    pub async fn import_bundle(
        &self,
        user_id: Snowflake,
        bytes: &[u8],
    ) -> Result<ImportedBundle, RealmError> {
        let unpacked = unpack(bytes)?;
        Self::check(&unpacked.source)?;
        let payloads = payloads(&unpacked)?;
        let preview = self.inspect(user_id, &unpacked.manifest, &payloads).await?;
        if let Some(problem) = preview.resources.into_iter().find_map(|r| r.problem) {
            return Err(RealmError::InvalidBundle(problem));
        }

        let mut ids = HashMap::new();
        let mut resources = Vec::new();
        let mut report = AttachmentReport::default();
        for (entry, payload) in unpacked.manifest.resources.iter().zip(payloads) {
            let name = payload.name().to_string();
            let (id, reused) = self
                .import_resource(user_id, entry, payload, &unpacked.files, &mut report)
                .await?;
            ids.insert((entry.kind, entry.id), id);
            resources.push(ImportedResource {
                kind: entry.kind,
                name,
                id,
                reused,
            });
        }

        let doc = wtn::compile(&unpacked.source)?;
        let source = rewrite(&unpacked.source, &static_refs(doc.document()), &ids);
        let Manifest {
            name, description, ..
        } = unpacked.manifest;
        let mut blueprint = self.create(user_id, name, source).await?;
        if !description.is_empty() {
            blueprint = self
                .update(blueprint.id, user_id, None, Some(description), None)
                .await?;
        }

        Ok(ImportedBundle {
            blueprint,
            resources,
            renamed: report.renamed,
            skipped: report.skipped,
        })
    }

    async fn import_resource(
        &self,
        user_id: Snowflake,
        entry: &ManifestEntry,
        payload: Payload,
        files: &HashMap<String, Vec<u8>>,
        report: &mut AttachmentReport,
    ) -> Result<(Snowflake, bool), RealmError> {
        let unusable = |name: &str| {
            invalid(format!(
                "The {} \"{name}\" in this bundle isn't valid, so the blueprint can't be imported",
                entry.kind.as_str()
            ))
        };
        let mut absorb = |from: AttachmentReport| {
            report.renamed.extend(from.renamed);
            report.skipped.extend(from.skipped);
            from.reused > 0
        };

        let item = match payload {
            Payload::Card { .. } => {
                let card = self
                    .characters
                    .import_card(&files[&entry.path], user_id, ImportOptions::default())
                    .await?;
                absorb(card.attachments);
                return Ok((card.id, false));
            }
            Payload::Item(item) => item,
        };

        match item {
            Item::Template {
                name,
                description,
                mut nodes,
            } => {
                fresh_ids(&mut nodes, &self.snowflake);
                let mut template = ContextTemplate::new(name, user_id, &self.snowflake);
                template.description = description;
                template.nodes = nodes;
                self.storage
                    .save_template(&template.to_db()?)
                    .await
                    .map_err(ContextError::from)?;
                Ok((template.id, false))
            }
            Item::Persona {
                name,
                display_name,
                pseudonym,
                description,
            } => {
                let id = self.personas.create_default(user_id, name).await?;
                let update = UpdatePersona {
                    name: None,
                    display_name: Some(display_name),
                    pseudonym: Some(pseudonym),
                    description: Some(description),
                };
                self.personas.update_persona(id, user_id, update).await?;

                if let Some(avatar) = entry.image.as_ref().and_then(|path| files.get(path))
                    && infer::get(avatar).is_some_and(|k| AVATAR_TYPES.contains(&k.mime_type()))
                {
                    self.personas
                        .update_image(id, user_id, Bytes::copy_from_slice(avatar))
                        .await?;
                }
                Ok((id, false))
            }
            Item::Tool { mut tools } => {
                let name = tools[0].name.clone();
                tools[0].attached = true;
                let extension = LorealmCardExtension {
                    tools,
                    ..Default::default()
                };
                let imported = self.attach(user_id, &extension).await?;
                let id = imported.tool_ids.first().copied();
                let reused = absorb(imported.report);
                Ok((id.ok_or_else(|| unusable(&name))?, reused))
            }
            Item::Widget { tools, widgets } => {
                let name = widgets[0].name.clone();
                let extension = LorealmCardExtension {
                    tools,
                    widgets,
                    ..Default::default()
                };
                let imported = self.attach(user_id, &extension).await?;
                let id = imported.widget_ids.first().copied();
                absorb(imported.report);
                Ok((id.ok_or_else(|| unusable(&name))?, false))
            }
            Item::Lorebook { lorebook } => {
                let book = lorebook_from_data(&lorebook, user_id, &self.snowflake)
                    .ok_or_else(|| unusable(&lorebook.name))?;
                self.storage.save_lorebook(&book).await?;
                Ok((book.id, false))
            }
            Item::Module {
                name,
                description,
                source,
            } => {
                let mine = self.storage.list_modules(user_id).await?;
                if let Some(same) = mine.iter().find(|m| m.name == name && m.source == source) {
                    return Ok((same.id, true));
                }
                let module = self
                    .modules
                    .create(user_id, name, description, source)
                    .await
                    .map_err(|e| match e {
                        ModuleError::Storage(e) => RealmError::Storage(e),
                        other => invalid(other.message()),
                    })?;
                Ok((module.id, false))
            }
        }
    }

    async fn attach(
        &self,
        user_id: Snowflake,
        extension: &LorealmCardExtension,
    ) -> Result<crate::services::cards::import::ImportedAttachments, RealmError> {
        Ok(import_attachments(
            self.storage.as_ref(),
            &self.snowflake,
            user_id,
            extension,
            ImportOptions::default(),
        )
        .await?)
    }
}

fn payloads(unpacked: &UnpackedBundle) -> Result<Vec<Payload>, RealmError> {
    unpacked
        .manifest
        .resources
        .iter()
        .map(|entry| parse_payload(entry, &unpacked.files[&entry.path]))
        .collect()
}
