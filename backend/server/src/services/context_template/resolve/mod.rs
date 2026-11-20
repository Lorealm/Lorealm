use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use lorealm_storage::models::Snowflake;
use weaver_lang::ast::template::NodeKind;

use crate::{
    errors::ContextError,
    services::{
        context_template::GroupWrapMode,
        plugins::PLUGIN_SOURCE_PREFIX,
        variables::substitute,
        weaver::{WeaverHost, WeaverProfile, compile_template},
    },
};

use super::{
    PromptRole,
    template::{ContextNode, ContextTemplate, NodeContent, SourceDefinition},
};

pub mod builtin;
mod convert;
mod types;

pub use convert::to_weaver_lorebook;
pub use types::*;

/// A source of context blocks.
#[async_trait]
pub trait ContextSource: Send + Sync {
    fn definition(&self) -> SourceDefinition;

    async fn resolve(
        &self,
        node_id: &str,
        settings: &HashMap<String, serde_json::Value>,
        ctx: &ResolveContext<'_>,
    ) -> Result<Vec<ResolvedBlock>, ContextError>;
}

/// All known context sources, keyed by [`SourceDefinition::source_id`].
#[derive(Clone, Default)]
pub struct SourceRegistry {
    sources: HashMap<String, Arc<dyn ContextSource>>,
}

impl SourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, source: Arc<dyn ContextSource>) {
        self.sources.insert(source.definition().source_id, source);
    }

    pub fn get(&self, source_id: &str) -> Option<&Arc<dyn ContextSource>> {
        self.sources.get(source_id)
    }

    /// Every source's metadata
    pub fn definitions(&self) -> Vec<SourceDefinition> {
        self.sources.values().map(|s| s.definition()).collect()
    }
}

/// Walk the template depth-first, producing blocks in flattened tree order.
pub async fn assemble(
    template: &ContextTemplate,
    registry: &SourceRegistry,
    ctx: &ResolveContext<'_>,
) -> Result<Vec<TracedBlock>, ContextError> {
    resolve_nodes(&template.nodes, registry, ctx, Vec::new()).await
}

// Boxed because the recursion is async (groups await child resolution).
fn resolve_nodes<'a>(
    nodes: &'a [ContextNode],
    registry: &'a SourceRegistry,
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
                    block: {
                        let (text, content_ref) = render_text(&node.title, content, ctx);
                        ResolvedBlock::atomic(role.clone(), text, content_ref)
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
                    let Some(resolver) = registry.get(source_id) else {
                        // A plugin that isn't loaded. Its log says why.
                        if source_id.starts_with(PLUGIN_SOURCE_PREFIX) {
                            continue;
                        }
                        return Err(ContextError::ResolverNotFound(source_id.clone()));
                    };
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

        for block in out.iter_mut() {
            apply_variables(&mut block.block.content, ctx);
        }

        Ok(out)
    })
}

fn render_text(title: &str, content: &str, ctx: &ResolveContext) -> (String, ContentRef) {
    let mut host = WeaverHost::builder()
        .profile(WeaverProfile::TemplateText)
        .pool(Arc::new(Mutex::new(ctx.state.persistent.clone())))
        .actor(ctx.actor.clone())
        .variables(ctx.variables.clone())
        .plugins(ctx.plugins.clone());
    if let Some(realm) = &ctx.realm {
        host = host.realm(realm.clone());
    }
    let host = host.build();
    let run = |source: &str| {
        compile_template(source)
            .and_then(|body| host.evaluate_inline(body))
            .map(|outcome| outcome.to_output_string())
    };
    match run(content) {
        Ok(text) => (text, expansions(content, run)),
        Err(e) => {
            tracing::warn!("template text node '{title}' kept as written: {e}");
            (content.to_string(), ContentRef::Verbatim)
        }
    }
}

fn expansions<E>(source: &str, run: impl Fn(&str) -> Result<String, E>) -> ContentRef {
    let Ok(template) = weaver_lang::parse(source) else {
        return ContentRef::Verbatim;
    };
    let literal = |s: &str| TemplatePart {
        source: s.to_string(),
        value: None,
    };
    let mut parts = Vec::new();
    let mut at = 0;
    for node in &template.nodes {
        if matches!(node.node, NodeKind::Literal(_)) {
            continue;
        }
        let Some(piece) = source.get(node.span.start..node.span.end) else {
            continue;
        };
        if node.span.start > at {
            parts.push(literal(&source[at..node.span.start]));
        }
        parts.push(TemplatePart {
            source: piece.to_string(),
            value: run(piece).ok(),
        });
        at = node.span.end;
    }
    if parts.is_empty() {
        return ContentRef::Verbatim;
    }
    if at < source.len() {
        parts.push(literal(&source[at..]));
    }
    ContentRef::Template(parts)
}

fn apply_variables(content: &mut String, ctx: &ResolveContext) {
    *content = substitute(content, &ctx.variables);
}

fn wrapper_block(
    node: &ContextNode,
    path: &[Snowflake],
    role: PromptRole,
    content: String,
) -> TracedBlock {
    TracedBlock {
        block: ResolvedBlock::atomic(role, content, ContentRef::Verbatim),
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
