use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use lorealm_storage::models::Snowflake;

use super::dto::{ImportResponse, Item, Kind, TRANSFER_VERSION, TransferFile};
use crate::{
    core::AppState,
    errors::{AppError, ContextError},
    extract::AuthenticatedSession,
    services::{
        cards::{
            extension::{LorealmCardExtension, LorebookData, ToolData, WidgetData},
            import::{AttachmentReport, ImportOptions, import_attachments},
            lorebook::lorebook_from_data,
        },
        context_template::ContextTemplate,
        persona::UpdatePersona,
        tools::builtin_tool,
        transfer::fresh_ids,
    },
};

pub async fn export(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((kind, id)): Path<(Kind, Snowflake)>,
) -> Result<Json<TransferFile>, AppError> {
    let services = state.services();
    let user_id = session.user_id;

    let item = match kind {
        Kind::Template => {
            let t = services.context().get_template(id, user_id).await?;
            Item::Template {
                name: t.name,
                description: t.description,
                nodes: t.nodes,
            }
        }
        Kind::Persona => {
            let p = services.persona().get_persona(id, user_id).await?;
            Item::Persona {
                name: p.name,
                display_name: p.display_name,
                pseudonym: p.pseudonym,
                description: p.description,
            }
        }
        Kind::Tool => {
            let tool = services.tools().get_tool(id, user_id).await?;
            Item::Tool {
                tools: vec![ToolData::from_tool(&tool, true)],
            }
        }
        Kind::Widget => {
            let widget = services.widgets().get_widget(id, user_id).await?;
            let mut tools = Vec::new();
            for tool_id in &widget.tools {
                let tool = match builtin_tool(*tool_id) {
                    Some(tool) => Some(tool),
                    // A tool deleted since it was attached is left out.
                    None => services.tools().get_tool(*tool_id, user_id).await.ok(),
                };
                if let Some(tool) = tool {
                    tools.push(ToolData::from_tool(&tool, false));
                }
            }
            let hashes = tools.iter().map(|t| t.hash.clone()).collect();
            Item::Widget {
                widgets: vec![WidgetData::from_widget(&widget, hashes)],
                tools,
            }
        }
        Kind::Lorebook => {
            let lorebook = services.lorebooks().get_lorebook(id, user_id).await?;
            Item::Lorebook {
                lorebook: LorebookData::from_lorebook(&lorebook),
            }
        }
        Kind::Module => {
            let module = services.modules().get(id, user_id).await?;
            Item::Module {
                name: module.name,
                description: module.description,
                source: module.source,
            }
        }
    };

    Ok(Json(TransferFile {
        lorealm: TRANSFER_VERSION,
        item,
    }))
}

pub async fn import(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(file): Json<TransferFile>,
) -> Result<Json<ImportResponse>, AppError> {
    let services = state.services();
    let user_id = session.user_id;

    let (kind, tools, widgets) = match file.item {
        Item::Template {
            name,
            description,
            mut nodes,
        } => {
            fresh_ids(&mut nodes, services.snowflake());
            let mut template = ContextTemplate::new(name, user_id, services.snowflake());
            template.description = description;
            template.nodes = nodes;
            services
                .storage()
                .save_template(&template.to_db()?)
                .await
                .map_err(ContextError::from)?;
            return Ok(Json(ImportResponse {
                kind: Kind::Template,
                id: Some(template.id),
                report: None,
            }));
        }
        Item::Persona {
            name,
            display_name,
            pseudonym,
            description,
        } => {
            let id = services.persona().create_default(user_id, name).await?;
            let update = UpdatePersona {
                name: None,
                display_name: Some(display_name),
                pseudonym: Some(pseudonym),
                description: Some(description),
            };
            services
                .persona()
                .update_persona(id, user_id, update)
                .await?;
            return Ok(Json(ImportResponse {
                kind: Kind::Persona,
                id: Some(id),
                report: None,
            }));
        }
        Item::Lorebook { lorebook } => {
            let mut report = AttachmentReport::default();
            let id = match lorebook_from_data(&lorebook, user_id, services.snowflake()) {
                Some(book) => {
                    services
                        .storage()
                        .save_lorebook(&book)
                        .await
                        .map_err(ContextError::from)?;
                    report.created += 1;
                    Some(book.id)
                }
                None => {
                    report.skipped.push(lorebook.name);
                    None
                }
            };
            return Ok(Json(ImportResponse {
                kind: Kind::Lorebook,
                id,
                report: Some(report),
            }));
        }
        Item::Module {
            name,
            description,
            source,
        } => {
            let module = services
                .modules()
                .create(user_id, name, description, source)
                .await?;
            return Ok(Json(ImportResponse {
                kind: Kind::Module,
                id: Some(module.id),
                report: None,
            }));
        }
        Item::Tool { tools } => (Kind::Tool, tools, Vec::new()),
        Item::Widget { tools, widgets } => (Kind::Widget, tools, widgets),
    };

    // Tools and widgets go through the same path as a card's attachments.
    let extension = LorealmCardExtension {
        tools,
        widgets,
        ..Default::default()
    };
    let imported = import_attachments(
        services.storage().as_ref(),
        services.snowflake(),
        user_id,
        &extension,
        ImportOptions::default(),
    )
    .await?;
    let id = match kind {
        Kind::Widget => imported.widget_ids.first(),
        _ => imported.tool_ids.first(),
    };

    Ok(Json(ImportResponse {
        kind,
        id: id.copied(),
        report: Some(imported.report),
    }))
}
