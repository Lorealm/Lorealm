use std::collections::{HashMap, HashSet};

use chrono::Utc;
use lorealm_storage::{
    Storage,
    models::{Snowflake, UserTool, UserWidget},
};
use serde::{Deserialize, Serialize};

use super::{
    extension::{LorealmCardExtension, LorebookData, ToolData, WidgetData},
    lorebook::lorebook_from_data,
};
use crate::{
    errors::CharacterError,
    services::{
        snowflake::SnowflakeService,
        tools::{UserToolService, builtin_tools, unique_tool_name},
        user_widgets::UserWidgetService,
    },
};

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ImportOptions {
    /// Attach an existing tool or widget instead of importing an identical copy.
    #[serde(default = "yes")]
    pub reuse_tools_and_widgets: bool,
    /// The same for lorebooks.
    #[serde(default)]
    pub reuse_lorebooks: bool,
}

fn yes() -> bool {
    true
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            reuse_tools_and_widgets: true,
            reuse_lorebooks: false,
        }
    }
}

/// What happened to a card's attachments.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AttachmentReport {
    /// Existing resources attached instead of importing a copy.
    pub reused: usize,
    pub created: usize,
    /// Tools imported under a new name: `(original name, new name)`.
    pub renamed: Vec<(String, String)>,
    /// Attachments that were invalid and left out.
    pub skipped: Vec<String>,
}

impl AttachmentReport {
    pub fn is_empty(&self) -> bool {
        self.reused == 0 && self.created == 0 && self.renamed.is_empty() && self.skipped.is_empty()
    }
}

pub struct ImportedAttachments {
    pub lorebook_ids: Vec<Snowflake>,
    pub tool_ids: Vec<Snowflake>,
    pub widget_ids: Vec<Snowflake>,
    pub report: AttachmentReport,
}

pub async fn import_attachments(
    storage: &dyn Storage,
    snowflake: &SnowflakeService,
    user_id: Snowflake,
    extension: &LorealmCardExtension,
    options: ImportOptions,
) -> Result<ImportedAttachments, CharacterError> {
    let mut report = AttachmentReport::default();

    let library = storage.list_tools(user_id).await?;
    let mut taken: HashSet<String> = library.iter().map(|t| t.name.clone()).collect();

    let mut tool_by_hash: HashMap<String, Snowflake> = HashMap::new();
    for t in builtin_tools().iter().chain(&library) {
        keep_oldest(&mut tool_by_hash, ToolData::from_tool(t, false).hash, t.id);
    }
    let tool_hash_by_id: HashMap<Snowflake, String> = tool_by_hash
        .iter()
        .map(|(h, id)| (*id, h.clone()))
        .collect();

    let mut card_tools: HashMap<String, Snowflake> = HashMap::new();
    let mut tool_ids = Vec::new();

    for data in &extension.tools {
        let hash = data.content_hash();
        let existing = tool_by_hash.get(&hash).copied();
        let id = match existing {
            Some(id)
                if options.reuse_tools_and_widgets
                    || builtin_tools().iter().any(|t| t.id == id) =>
            {
                report.reused += 1;
                id
            }
            _ => match create_tool(storage, snowflake, user_id, data, &mut taken, &mut report)
                .await?
            {
                Some(id) => {
                    tool_by_hash.insert(hash.clone(), id);
                    id
                }
                None => continue,
            },
        };
        card_tools.insert(data.hash.clone(), id);
        card_tools.insert(hash, id);
        if data.attached && !tool_ids.contains(&id) {
            tool_ids.push(id);
        }
    }

    let mut widget_by_hash: HashMap<String, Snowflake> = HashMap::new();
    if options.reuse_tools_and_widgets {
        for widget in storage.list_widgets(user_id).await? {
            let tools: Vec<String> = widget
                .tools
                .iter()
                .filter_map(|id| tool_hash_by_id.get(id).cloned())
                .collect();
            keep_oldest(
                &mut widget_by_hash,
                WidgetData::from_widget(&widget, tools).hash,
                widget.id,
            );
        }
    }

    let mut widget_ids = Vec::new();
    for data in &extension.widgets {
        let hash = data.content_hash();
        if let Some(id) = widget_by_hash.get(&hash) {
            report.reused += 1;
            widget_ids.push(*id);
            continue;
        }
        if UserWidgetService::validate_name(&data.name).is_err()
            || UserWidgetService::validate_props(&data.props).is_err()
        {
            report.skipped.push(data.name.clone());
            continue;
        }
        let widget = UserWidget {
            id: snowflake.generate(),
            user_id,
            name: data.name.clone(),
            description: data.description.clone(),
            source: data.source.clone(),
            prompt: data.prompt.clone(),
            render: data.render.clone(),
            tools: data
                .tools
                .iter()
                .filter_map(|h| card_tools.get(h).copied())
                .collect(),
            props: data.props.clone(),
            placement: data.placement,
            mode: data.mode,
            enabled: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        storage.save_widget(widget.clone()).await?;
        widget_by_hash.insert(hash, widget.id);
        report.created += 1;
        widget_ids.push(widget.id);
    }

    let mut lorebook_by_hash: HashMap<String, Snowflake> = HashMap::new();
    if options.reuse_lorebooks {
        for meta in storage.list_lorebooks_by_owner(user_id).await? {
            if let Some(book) = storage.get_lorebook(meta.id).await? {
                keep_oldest(
                    &mut lorebook_by_hash,
                    LorebookData::from_lorebook(&book).hash,
                    book.id,
                );
            }
        }
    }

    let mut lorebook_ids = Vec::new();
    for data in &extension.lorebooks {
        if let Some(id) = lorebook_by_hash.get(&data.content_hash()) {
            report.reused += 1;
            lorebook_ids.push(*id);
            continue;
        }
        match lorebook_from_data(data, user_id, snowflake) {
            Some(book) => {
                storage.save_lorebook(&book).await?;
                report.created += 1;
                lorebook_ids.push(book.id);
            }
            None => report.skipped.push(data.name.clone()),
        }
    }

    Ok(ImportedAttachments {
        lorebook_ids,
        tool_ids,
        widget_ids,
        report,
    })
}

fn keep_oldest(map: &mut HashMap<String, Snowflake>, hash: String, id: Snowflake) {
    map.entry(hash)
        .and_modify(|existing| *existing = (*existing).min(id))
        .or_insert(id);
}

async fn create_tool(
    storage: &dyn Storage,
    snowflake: &SnowflakeService,
    user_id: Snowflake,
    data: &ToolData,
    taken: &mut HashSet<String>,
    report: &mut AttachmentReport,
) -> Result<Option<Snowflake>, CharacterError> {
    let valid = UserToolService::validate_name(&data.name).is_ok()
        && UserToolService::validate_body(&data.body).is_ok()
        && UserToolService::validate_params(&data.params).is_ok();
    if !valid {
        report.skipped.push(data.name.clone());
        return Ok(None);
    }

    let name = unique_tool_name(&data.name, taken);
    if UserToolService::validate_command_name(&name, data.surface).is_err() {
        report.skipped.push(data.name.clone());
        return Ok(None);
    }
    if name != data.name {
        report.renamed.push((data.name.clone(), name.clone()));
    }
    taken.insert(name.clone());

    let tool = UserTool {
        user_id,
        name,
        id: snowflake.generate(),
        display_name: data.display_name.clone(),
        description: data.description.clone(),
        params: data.params.clone(),
        body: data.body.clone(),
        surface: data.surface,
        enabled: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    storage.save_tool(tool.clone()).await?;
    report.created += 1;
    Ok(Some(tool.id))
}
