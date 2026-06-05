use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use async_trait::async_trait;
use eldoria_storage::models::{Character, ConversationMessage, Snowflake};
use serde::Serialize;

use crate::{errors::ContextError, services::context::GroupWrapMode};

use super::{
    PromptRole,
    template::{ContextNode, ContextTemplate, NodeContent},
};

pub mod builtin;
mod convert;

pub use convert::to_weaver_lorebook;

#[derive(Debug, Clone, Serialize)]
pub enum ContentRef {
    /// Non-recoverable; the text must be stored verbatim. Used for unpredictable/dynamic sources
    /// that can't be reconstructed reliably
    Verbatim,
    Message(Snowflake),
    Character(CharacterPart),
    /// Recoverable reference defined by a plugin source.
    /// The frontend dispatches on source_id to interpret `data`.
    Plugin {
        source_id: String,
        data: serde_json::Value,
    },
}

impl ContentRef {
    /// Whether the live content can be dropped before persisting.
    pub fn is_recoverable(&self) -> bool {
        !matches!(self, ContentRef::Verbatim)
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum CharacterPart {
    Definition,
    Personality,
    Scenario,
    Greeting { order: i64 },   // the *chosen* greeting, by its stable key
    Example { order: i64 },    // each *included* example, by its stable key
}

/// One resolved piece of context, tagged with the role it occupies.
/// Tree order is preserved by the walk.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedBlock {
    pub role: PromptRole,
    pub content: String,
    pub content_ref: ContentRef,
}

#[derive(Debug, Clone, Serialize)]
pub struct TracedBlock {
    #[serde(flatten)]
    pub block: ResolvedBlock,
    pub origin: BlockOrigin,
}

#[derive(Debug, Clone, Serialize)]
pub struct BlockOrigin {
    /// The ContextNode that emitted this block.
    pub node_id: Snowflake,
    pub node_title: String,
    /// Enclosing group ids, outermost first. Empty = root-level.
    pub path: Vec<Snowflake>,
    pub kind: OriginKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OriginKind {
    Text,
    Source { source_id: String, node_id: String },
    GroupWrapper, // a prefix/suffix block, belongs to the group node
}

/// Per-turn input handed to every resolver. Resolvers hold their own
/// long-lived deps (service Arcs); this only carries what changes per assembly.
pub struct ResolveContext<'a> {
    pub character: &'a Character,
    pub messages: &'a [ConversationMessage],
    pub user_id: Snowflake,
}

#[async_trait]
pub trait SourceResolver: Send + Sync {
    fn source_id(&self) -> &str;

    async fn resolve(
        &self,
        node_id: &str,
        settings: &HashMap<String, serde_json::Value>,
        ctx: &ResolveContext<'_>,
    ) -> Result<Vec<ResolvedBlock>, ContextError>;
}

/// Runtime counterpart to `SourceRegistry`: id -> producer, where
/// `SourceRegistry` is id -> metadata.
#[derive(Clone, Default)]
pub struct ResolverRegistry {
    resolvers: HashMap<String, Arc<dyn SourceResolver>>,
}

impl ResolverRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, resolver: Arc<dyn SourceResolver>) {
        self.resolvers
            .insert(resolver.source_id().to_string(), resolver);
    }

    pub fn get(&self, source_id: &str) -> Option<&Arc<dyn SourceResolver>> {
        self.resolvers.get(source_id)
    }
}

/// Walk the template depth-first, producing blocks in flattened tree order.
pub async fn assemble(
    template: &ContextTemplate,
    registry: &ResolverRegistry,
    ctx: &ResolveContext<'_>,
) -> Result<Vec<TracedBlock>, ContextError> {
    resolve_nodes(&template.nodes, registry, ctx, Vec::new()).await
}

// Boxed because the recursion is async (groups await child resolution).
fn resolve_nodes<'a>(
    nodes: &'a [ContextNode],
    registry: &'a ResolverRegistry,
    ctx: &'a ResolveContext<'a>,
    path: Vec<Snowflake>, // <- new: current group ancestry
) -> Pin<Box<dyn Future<Output = Result<Vec<TracedBlock>, ContextError>> + Send + 'a>> {
    Box::pin(async move {
        let mut out = Vec::new();
        for node in nodes {
            if !node.enabled {
                continue;
            }
            match &node.content {
                NodeContent::Text { content, role } => out.push(TracedBlock {
                    block: ResolvedBlock {
                        role: role.clone(),
                        content: content.clone(),
                        content_ref: ContentRef::Verbatim,
                    },
                    origin: BlockOrigin {
                        node_id: node.id,
                        node_title: node.title.clone(),
                        path: path.to_vec(),
                        kind: OriginKind::Text,
                    },
                }),
                NodeContent::Source {
                    source_id,
                    node_id,
                    settings,
                } => {
                    let resolver = registry
                        .get(source_id)
                        .ok_or_else(|| ContextError::ResolverNotFound(source_id.clone()))?;
                    let kind = OriginKind::Source {
                        source_id: source_id.clone(),
                        node_id: node_id.clone(),
                    };
                    for block in resolver.resolve(node_id, settings, ctx).await? {
                        out.push(TracedBlock {
                            block,
                            origin: BlockOrigin {
                                node_id: node.id,
                                node_title: node.title.clone(),
                                path: path.to_vec(),
                                kind: kind.clone(),
                            },
                        });
                    }
                }
                NodeContent::Group {
                    children,
                    prefix,
                    suffix,
                    wrap_mode,
                    ..
                } => {
                    let mut child_path = path.to_vec();
                    child_path.push(node.id);
                    let child_blocks = resolve_nodes(children, registry, ctx, child_path).await?;
                    out.extend(apply_wrap(
                        node,
                        &path,
                        wrap_mode,
                        prefix,
                        suffix,
                        child_blocks,
                    ));
                }
            }
        }
        Ok(out)
    })
}

fn wrapper_block(
    node: &ContextNode,
    path: &[Snowflake],
    role: PromptRole,
    content: String,
) -> TracedBlock {
    TracedBlock {
        block: ResolvedBlock {
            role,
            content,
            content_ref: ContentRef::Verbatim,
        },
        origin: BlockOrigin {
            node_id: node.id,
            node_title: node.title.clone(),
            path: path.to_vec(),
            kind: OriginKind::GroupWrapper,
        },
    }
}

fn apply_wrap(
    node: &ContextNode,
    path: &[Snowflake],
    mode: &GroupWrapMode,
    prefix: &Option<String>,
    suffix: &Option<String>,
    children: Vec<TracedBlock>,
) -> Vec<TracedBlock> {
    if children.is_empty() {
        return children;
    }
    match mode {
        GroupWrapMode::NoMixed => match common_role(&children) {
            Some(role) => wrap_once(node, path, role, prefix, suffix, children),
            None => children,
        },
        GroupWrapMode::Majority => {
            let role = majority_role(&children);
            wrap_once(node, path, role, prefix, suffix, children)
        }
        GroupWrapMode::All => wrap_runs(node, path, prefix, suffix, children, None),
        GroupWrapMode::Only(t) => wrap_runs(node, path, prefix, suffix, children, Some(t)),
    }
}

fn wrap_once(
    node: &ContextNode,
    path: &[Snowflake],
    role: PromptRole,
    prefix: &Option<String>,
    suffix: &Option<String>,
    mut children: Vec<TracedBlock>,
) -> Vec<TracedBlock> {
    let mut out = Vec::with_capacity(children.len() + 2);
    if let Some(p) = prefix {
        out.push(wrapper_block(node, path, role.clone(), p.clone()));
    }
    out.append(&mut children);
    if let Some(s) = suffix {
        out.push(wrapper_block(node, path, role, s.clone()));
    }
    out
}

fn wrap_runs(
    node: &ContextNode,
    path: &[Snowflake],
    prefix: &Option<String>,
    suffix: &Option<String>,
    children: Vec<TracedBlock>,
    only: Option<&PromptRole>,
) -> Vec<TracedBlock> {
    let mut out = Vec::with_capacity(children.len());
    let mut iter = children.into_iter().peekable();

    while let Some(first) = iter.next() {
        let role = first.block.role.clone();
        let mut run = vec![first];
        while iter.peek().map(|b| b.block.role == role).unwrap_or(false) {
            run.push(iter.next().unwrap());
        }

        let wrap = only.map(|t| t == &role).unwrap_or(true);
        if wrap {
            if let Some(p) = prefix {
                out.push(wrapper_block(node, path, role.clone(), p.clone()));
            }
            out.append(&mut run);
            if let Some(s) = suffix {
                out.push(wrapper_block(node, path, role, s.clone()));
            }
        } else {
            out.append(&mut run);
        }
    }
    out
}

fn common_role(blocks: &[TracedBlock]) -> Option<PromptRole> {
    let first = blocks.first()?.block.role.clone();
    blocks
        .iter()
        .all(|b| b.block.role == first)
        .then_some(first)
}

fn majority_role(blocks: &[TracedBlock]) -> PromptRole {
    let mut counts: Vec<(&PromptRole, usize)> = Vec::new();
    for b in blocks {
        match counts.iter_mut().find(|(r, _)| *r == &b.block.role) {
            Some(e) => e.1 += 1,
            None => counts.push((&b.block.role, 1)),
        }
    }
    let mut best = &counts[0];
    for c in &counts[1..] {
        if c.1 > best.1 {
            best = c;
        }
    }
    best.0.clone()
}
