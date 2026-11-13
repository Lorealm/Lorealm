use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use itertools::Itertools;
use kitoken::Kitoken;
use serde::Serialize;

use crate::{
    errors::{ContextError, TokenizerError},
    services::{
        chat::PromptMessage,
        context_template::{BlockPart, OriginKind, PromptRole, TracedBlock, Trim},
    },
};

/// Counts tokens for budgeting. Abstracted over [`Kitoken`] so the evaluator's
/// arithmetic can be exercised without loading a vocabulary.
pub trait TokenCounter: Send + Sync {
    fn count(&self, text: &str) -> Result<u32, TokenizerError>;
}

impl TokenCounter for Kitoken {
    fn count(&self, text: &str) -> Result<u32, TokenizerError> {
        Ok(self
            .encode(text, false)
            .map_err(|e| TokenizerError::Tokenize(e.to_string()))?
            .len() as u32)
    }
}

/// A source of a certain piece of context.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSource {
    /// The chat history
    Conversation,
    /// Anything produced by nodes in a Context Template.
    ///
    /// This only includes text nodes, group prefixes or other flat nodes.
    ContextTemplate,
    /// Context produced by a lorebook
    Lorebook,
    /// Character description, personality and scenario
    Character,
    /// Example dialogue, which the character reads as style guidance rather
    /// than fact, and can therefore lose without becoming a different character
    CharacterExamples,
    /// System prompts, Weaver [`effect`] injections and other things
    /// that aren't otherwise listed
    ///
    /// weaver-effect: crate::services::commands::CommandEffect
    Miscellaneous,
    /// Anything produced by a plugin
    Plugin,
}

impl ContextSource {
    pub fn of(origin: &OriginKind) -> Self {
        let OriginKind::Source { source_id, node_id } = origin else {
            // Text nodes and group prefixes/suffixes are the template's own words.
            return Self::ContextTemplate;
        };
        match (source_id.as_str(), node_id.as_str()) {
            ("chat_history", _) => Self::Conversation,
            ("world_info", _) => Self::Lorebook,
            ("character", "example_messages") => Self::CharacterExamples,
            ("character", _) => Self::Character,
            ("persona", _) | ("widgets", _) => Self::Miscellaneous,
            _ => Self::Plugin,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Conversation => "chat history",
            Self::ContextTemplate => "template",
            Self::Lorebook => "lorebook",
            Self::Character => "character",
            Self::CharacterExamples => "example messages",
            Self::Miscellaneous => "system prompts",
            Self::Plugin => "plugins",
        }
    }
}

/// What a source keeps no matter how tight the budget gets.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Floor {
    /// Trimmable all the way to nothing.
    #[default]
    Nothing,
    /// At least this many tokens survive.
    #[allow(dead_code)]
    Tokens(u32),
    /// The last N blocks the source emitted survive.
    Blocks(usize),
    /// Incompressible.
    Everything,
}

/// The most a source may be given, however much budget is going spare.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Cap {
    #[default]
    Unlimited,
    #[allow(dead_code)]
    Tokens(u32),
    /// A fraction of the total budget.
    Share(f32),
}

/// How a source sheds blocks once it is over its allocation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum TrimOrder {
    #[default]
    Oldest,
    Newest,
    #[allow(dead_code)]
    MiddleOut,
}

#[derive(Debug, Clone, Copy)]
pub struct SourceBudget {
    pub floor: Floor,
    pub cap: Cap,
    /// How important this entry is during the budget phase
    pub priority: i32,
    pub order: TrimOrder,
}

impl SourceBudget {
    const fn fixed() -> Self {
        Self {
            floor: Floor::Everything,
            cap: Cap::Unlimited,
            priority: i32::MAX,
            order: TrimOrder::Oldest,
        }
    }

    fn default_for(source: ContextSource) -> Self {
        match source {
            ContextSource::ContextTemplate
            | ContextSource::Character
            | ContextSource::Miscellaneous => Self::fixed(),

            ContextSource::Conversation => Self {
                floor: Floor::Blocks(6),
                cap: Cap::Unlimited,
                priority: 100,
                order: TrimOrder::Oldest,
            },

            ContextSource::Lorebook => Self {
                floor: Floor::Nothing,
                cap: Cap::Share(0.25),
                priority: 80,
                order: TrimOrder::Newest,
            },

            ContextSource::CharacterExamples => Self {
                floor: Floor::Nothing,
                cap: Cap::Unlimited,
                priority: 40,
                order: TrimOrder::Newest,
            },

            ContextSource::Plugin => Self {
                floor: Floor::Nothing,
                cap: Cap::Unlimited,
                priority: 20,
                order: TrimOrder::Newest,
            },
        }
    }
}

#[derive(Debug)]
pub struct ContextEvalResult {
    pub messages: Vec<PromptMessage>,
    pub trace: Vec<TracedBlock>,
    pub usage: UsageReport,
}

/// What the turn actually cost. Grouped by source.
#[derive(Debug, Default, Clone, Serialize)]
pub struct UsageReport {
    pub budget: Option<u32>,
    pub kept: CountResult,
    pub trimmed: CountResult,
}

#[cfg(test)]
impl UsageReport {
    fn was_trimmed(&self) -> bool {
        self.trimmed.total() > 0
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct CountResult(HashMap<ContextSource, u32>);

#[cfg(test)]
impl CountResult {
    fn total(&self) -> u32 {
        self.0.values().sum()
    }
}

impl CountResult {
    pub fn get(&self, source: ContextSource) -> u32 {
        self.0.get(&source).copied().unwrap_or(0)
    }

    /// Per-source difference, floored at zero.
    fn less(&self, other: &Self) -> Self {
        Self(
            self.0
                .iter()
                .map(|(&s, &n)| (s, n.saturating_sub(other.get(s))))
                .filter(|&(_, n)| n > 0)
                .collect(),
        )
    }

    /// Sources and their token counts, largest first. Deterministic.
    pub fn breakdown(&self) -> Vec<(ContextSource, u32)> {
        self.0
            .iter()
            .filter(|&(_, &n)| n > 0)
            .map(|(&s, &n)| (s, n))
            .sorted_by_key(|&(s, n)| (std::cmp::Reverse(n), s))
            .collect()
    }
}

/// Evaluates some context by conditionally:
/// - Applying context limits
/// - Merging/squashing messages
///
/// Note: This is applied *after* equivalent nodes in a [`ContextTemplate`],
/// on the flattened trace they produced.
///
/// [`ContextTemplate`]: crate::services::context::ContextTemplate
pub struct ContextEvaluator {
    counter: Arc<dyn TokenCounter>,
    config: ContextEvalConfig,
}

impl ContextEvaluator {
    pub fn builder(counter: Arc<dyn TokenCounter>) -> ContextEvalBuilder {
        ContextEvalBuilder::new(counter)
    }

    /// Fits `trace` inside the configured budget and flattens it to a prompt.
    ///
    /// This operation converts blocks to [`PromptMessage`], meaning it's
    /// non-recoverable
    pub fn evaluate(&self, mut trace: Vec<TracedBlock>) -> Result<ContextEvalResult, ContextError> {
        let mut cost = self.costs(&trace)?;
        let mut live = vec![true; trace.len()];

        // Taken before `fit`, which both drops blocks and shrinks the cost of
        // any it thins, so the difference is what trimming actually removed.
        let before = self.tally(&trace, &cost, &live);

        if let Some(budget) = self.config.budget {
            self.fit(&mut trace, &mut cost, &mut live, budget)?;
        }

        let kept = self.tally(&trace, &cost, &live);
        let trimmed = before.less(&kept);
        let trace: Vec<TracedBlock> = trace
            .into_iter()
            .zip(&live)
            .filter_map(|(b, &keep)| keep.then_some(b))
            .collect();

        Ok(ContextEvalResult {
            messages: self.to_messages(&trace),
            trace,
            usage: UsageReport {
                budget: self.config.budget,
                kept,
                trimmed,
            },
        })
    }

    /// Token counts per source before trimming.
    pub fn _grouped_count(&self, trace: &[TracedBlock]) -> Result<CountResult, ContextError> {
        let cost = self.costs(trace)?;
        Ok(self.tally(trace, &cost, &vec![true; trace.len()]))
    }

    /// Counts the final strings
    fn costs(&self, trace: &[TracedBlock]) -> Result<Vec<u32>, ContextError> {
        trace
            .iter()
            .map(|b| Ok(self.counter.count(&b.block.content)?))
            .collect()
    }

    fn tally(&self, trace: &[TracedBlock], cost: &[u32], live: &[bool]) -> CountResult {
        let mut counts: HashMap<ContextSource, u32> = HashMap::new();
        for (i, block) in trace.iter().enumerate() {
            if live[i] {
                *counts
                    .entry(ContextSource::of(&block.origin.kind))
                    .or_default() += cost[i];
            }
        }
        CountResult(counts)
    }

    fn budget_for(&self, source: ContextSource) -> SourceBudget {
        self.config
            .policies
            .get(&source)
            .copied()
            .unwrap_or_else(|| SourceBudget::default_for(source))
    }

    fn fit(
        &self,
        trace: &mut [TracedBlock],
        cost: &mut [u32],
        live: &mut [bool],
        budget: u32,
    ) -> Result<(), ContextError> {
        let mut groups: HashMap<ContextSource, Vec<usize>> = HashMap::new();
        for (i, block) in trace.iter().enumerate() {
            groups
                .entry(ContextSource::of(&block.origin.kind))
                .or_default()
                .push(i);
        }

        let mut floors = HashMap::new();
        let mut total_floor: u32 = 0;
        for (&source, idx) in &groups {
            let used: u32 = idx.iter().map(|&i| cost[i]).sum();
            let floor = match self.budget_for(source).floor {
                Floor::Nothing => 0,
                Floor::Tokens(t) => t.min(used),
                Floor::Blocks(n) => idx.iter().rev().take(n).map(|&i| cost[i]).sum(),
                Floor::Everything => used,
            };
            total_floor += floor;
            floors.insert(source, floor);
        }

        if total_floor > budget {
            let counts = self.tally(trace, cost, &vec![true; trace.len()]);
            return Err(ContextError::BudgetExceeded {
                budget,
                required: total_floor,
                breakdown: counts
                    .breakdown()
                    .into_iter()
                    .map(|(s, n)| (s.label().to_string(), n))
                    .collect(),
            });
        }

        // Descending priority
        let order = groups
            .keys()
            .copied()
            .sorted_by_key(|&s| (std::cmp::Reverse(self.budget_for(s).priority), s))
            .collect::<Vec<_>>();

        let mut spare = budget - total_floor;
        for source in order {
            let policy = self.budget_for(source);
            let idx = &groups[&source];
            let used: u32 = idx.iter().map(|&i| cost[i]).sum();
            let floor = floors[&source];

            let cap = match policy.cap {
                Cap::Unlimited => u32::MAX,
                Cap::Tokens(t) => t,
                Cap::Share(f) => (budget as f32 * f) as u32,
            }
            .max(floor);

            let want = used.min(cap).saturating_sub(floor);
            let give = want.min(spare);
            spare -= give;

            let alloc = floor + give;
            if used > alloc {
                self.shed(trace, cost, live, idx, alloc, policy);
            }
        }

        Ok(())
    }

    fn shed(
        &self,
        trace: &mut [TracedBlock],
        cost: &mut [u32],
        live: &mut [bool],
        idx: &[usize],
        alloc: u32,
        policy: SourceBudget,
    ) {
        let mut used: u32 = idx.iter().map(|&i| cost[i]).sum();

        let _ = self.thin(trace, cost, live, idx, alloc, &mut used);

        let protected = match policy.floor {
            Floor::Blocks(n) => n.min(idx.len()),
            _ => 0,
        };
        let droppable = &idx[..idx.len() - protected];

        for &i in Self::drop_order(droppable, policy.order) {
            if used <= alloc {
                break;
            }
            if !live[i] {
                continue;
            }
            live[i] = false;
            used -= cost[i];
        }
    }

    fn thin(
        &self,
        trace: &mut [TracedBlock],
        cost: &mut [u32],
        live: &mut [bool],
        idx: &[usize],
        alloc: u32,
        used: &mut u32,
    ) -> Result<(), TokenizerError> {
        let mut candidates: Vec<(usize, usize, i32)> = Vec::new();
        for &i in idx {
            if let Trim::Joined { parts, .. } = &trace[i].block.trim {
                for (j, part) in parts.iter().enumerate() {
                    candidates.push((i, j, part.priority));
                }
            }
        }
        if candidates.is_empty() {
            return Ok(());
        }

        // Ascending priority
        candidates.sort_by_key(|&(i, j, priority)| {
            (priority, std::cmp::Reverse(i), std::cmp::Reverse(j))
        });

        let mut dropped: HashMap<usize, HashSet<usize>> = HashMap::new();
        for (i, j, _) in candidates {
            if *used <= alloc {
                break;
            }
            dropped.entry(i).or_default().insert(j);
            let tokens = {
                let Trim::Joined { parts, separator } = &trace[i].block.trim else {
                    continue;
                };
                let text = join_kept(parts, &dropped[&i], separator);
                if text.is_empty() {
                    0
                } else {
                    self.counter.count(&text)?
                }
            };
            *used = *used - cost[i] + tokens;
            cost[i] = tokens;
        }

        for (i, gone) in dropped {
            let Trim::Joined { parts, separator } = &trace[i].block.trim else {
                continue;
            };
            let separator = separator.clone();
            let kept: Vec<BlockPart> = parts
                .iter()
                .enumerate()
                .filter(|(j, _)| !gone.contains(j))
                .map(|(_, p)| p.clone())
                .collect();
            if kept.is_empty() {
                live[i] = false;
                cost[i] = 0;
                continue;
            }
            trace[i].block.content = kept.iter().map(|p| p.content.as_str()).join(&separator);
            trace[i].block.trim = Trim::Joined {
                parts: kept,
                separator,
            };
        }

        Ok(())
    }

    fn drop_order(idx: &[usize], order: TrimOrder) -> Box<dyn Iterator<Item = &usize> + '_> {
        match order {
            TrimOrder::Oldest => Box::new(idx.iter()),
            TrimOrder::Newest => Box::new(idx.iter().rev()),
            TrimOrder::MiddleOut => {
                let mid = idx.len() / 2;
                let mut out: Vec<&usize> = Vec::with_capacity(idx.len());
                let (mut lo, mut hi) = (mid as isize - 1, mid);
                while hi < idx.len() || lo >= 0 {
                    if hi < idx.len() {
                        out.push(&idx[hi]);
                        hi += 1;
                    }
                    if lo >= 0 {
                        out.push(&idx[lo as usize]);
                        lo -= 1;
                    }
                }
                Box::new(out.into_iter())
            }
        }
    }

    fn to_messages(&self, trace: &[TracedBlock]) -> Vec<PromptMessage> {
        let messages = trace.iter().map(|t| PromptMessage {
            role: t.block.role.clone(),
            content: t.block.content.clone(),
        });

        if !self.config.squash_system_messages {
            return messages.collect();
        }

        messages
            .coalesce(|a, b| match (&a.role, &b.role) {
                (PromptRole::System, PromptRole::System) => Ok(PromptMessage {
                    role: PromptRole::System,
                    content: format!("{}\n{}", a.content, b.content),
                }),
                _ => Err((a, b)),
            })
            .collect()
    }
}

fn join_kept(parts: &[BlockPart], gone: &HashSet<usize>, separator: &str) -> String {
    parts
        .iter()
        .enumerate()
        .filter(|(j, _)| !gone.contains(j))
        .map(|(_, p)| p.content.as_str())
        .join(separator)
}

#[derive(Default)]
pub struct ContextEvalConfig {
    squash_system_messages: bool,
    budget: Option<u32>,
    /// Overrides for [`SourceBudget::default_for`].
    policies: HashMap<ContextSource, SourceBudget>,
}

pub struct ContextEvalBuilder {
    counter: Arc<dyn TokenCounter>,
    config: ContextEvalConfig,
}

impl ContextEvalBuilder {
    pub fn new(counter: Arc<dyn TokenCounter>) -> Self {
        Self {
            counter,
            config: ContextEvalConfig::default(),
        }
    }

    pub fn _squash_system_messages(mut self, squash: bool) -> Self {
        self.config.squash_system_messages = squash;
        self
    }

    /// The total token budget for the assembled prompt. Unset = unlimited.
    pub fn budget(mut self, budget: u32) -> Self {
        self.config.budget = Some(budget);
        self
    }

    pub fn _policy(mut self, source: ContextSource, budget: SourceBudget) -> Self {
        self.config.policies.insert(source, budget);
        self
    }

    pub fn build(self) -> ContextEvaluator {
        ContextEvaluator {
            counter: self.counter,
            config: self.config,
        }
    }
}

#[cfg(test)]
mod tests {
    use lorealm_storage::models::Snowflake;

    use crate::services::context_template::{BlockOrigin, ContentRef, ResolvedBlock};

    use super::*;

    /// One token per whitespace-separated word
    struct Words;

    impl TokenCounter for Words {
        fn count(&self, text: &str) -> Result<u32, TokenizerError> {
            Ok(text.split_whitespace().count() as u32)
        }
    }

    fn eval(budget: u32) -> ContextEvaluator {
        ContextEvaluator::builder(Arc::new(Words))
            .budget(budget)
            .build()
    }

    fn traced(kind: OriginKind, block: ResolvedBlock) -> TracedBlock {
        TracedBlock {
            block,
            origin: BlockOrigin {
                node_id: Snowflake(1),
                node_title: "n".into(),
                path: Vec::new(),
                kind,
            },
        }
    }

    fn from(source: &str, node: &str, content: &str) -> TracedBlock {
        traced(
            OriginKind::Source {
                source_id: source.into(),
                node_id: node.into(),
            },
            ResolvedBlock::atomic(PromptRole::System, content, ContentRef::Verbatim),
        )
    }

    fn msg(content: &str) -> TracedBlock {
        from("chat_history", "main", content)
    }

    fn text(content: &str) -> TracedBlock {
        traced(
            OriginKind::Text,
            ResolvedBlock::atomic(PromptRole::System, content, ContentRef::Verbatim),
        )
    }

    fn lore(node: &str, entries: &[(&str, i32)]) -> TracedBlock {
        let parts = entries
            .iter()
            .map(|(content, priority)| BlockPart {
                content: (*content).into(),
                priority: *priority,
            })
            .collect();
        traced(
            OriginKind::Source {
                source_id: "world_info".into(),
                node_id: node.into(),
            },
            ResolvedBlock::joined(PromptRole::System, parts, "\n", ContentRef::Verbatim),
        )
    }

    fn contents(result: &ContextEvalResult) -> Vec<&str> {
        result
            .trace
            .iter()
            .map(|t| t.block.content.as_str())
            .collect()
    }

    fn sized(word: &str, n: usize) -> String {
        vec![word; n].join(" ")
    }

    #[test]
    fn unknown_sources_classify_as_plugins() {
        assert_eq!(
            ContextSource::of(&OriginKind::Source {
                source_id: "some_extension".into(),
                node_id: "main".into(),
            }),
            ContextSource::Plugin
        );
        assert_eq!(
            ContextSource::of(&OriginKind::Text),
            ContextSource::ContextTemplate
        );
        assert_eq!(
            ContextSource::of(&OriginKind::GroupWrapper),
            ContextSource::ContextTemplate
        );
    }

    #[test]
    fn character_examples_are_scored_apart_from_the_sheet() {
        let of = |node: &str| {
            ContextSource::of(&OriginKind::Source {
                source_id: "character".into(),
                node_id: node.into(),
            })
        };
        assert_eq!(of("definition"), ContextSource::Character);
        assert_eq!(of("personality"), ContextSource::Character);
        assert_eq!(of("example_messages"), ContextSource::CharacterExamples);
    }

    #[test]
    fn an_unbudgeted_trace_passes_through_untouched() {
        let trace = vec![text("a b c"), msg("d e f"), lore("setting", &[("g h", 0)])];
        let out = ContextEvaluator::builder(Arc::new(Words))
            .build()
            .evaluate(trace)
            .unwrap();

        assert_eq!(contents(&out), ["a b c", "d e f", "g h"]);
        assert_eq!(out.usage.kept.total(), 8);
    }

    #[test]
    fn a_trace_that_already_fits_is_left_alone() {
        let trace = vec![text("a b"), msg("c d"), msg("e f")];
        let out = eval(100).evaluate(trace).unwrap();

        assert_eq!(contents(&out), ["a b", "c d", "e f"]);
    }

    #[test]
    fn chat_history_sheds_its_oldest_messages_first() {
        let trace: Vec<TracedBlock> = (1..=9).map(|i| msg(&sized(&format!("m{i}"), 2))).collect();
        let out = eval(14).evaluate(trace).unwrap();

        assert_eq!(out.usage.kept.get(ContextSource::Conversation), 14);
        assert_eq!(contents(&out)[0], "m3 m3");
        assert_eq!(contents(&out).len(), 7);
    }

    #[test]
    fn a_large_lorebook_cannot_erase_the_conversation() {
        let mut trace: Vec<TracedBlock> =
            (1..=8).map(|i| msg(&sized(&format!("m{i}"), 2))).collect();
        let entries: Vec<(String, i32)> = (0..10).map(|i| (sized("lore", 5), i)).collect();
        trace.push(lore(
            "setting",
            &entries
                .iter()
                .map(|(c, p)| (c.as_str(), *p))
                .collect::<Vec<_>>(),
        ));

        let out = eval(14).evaluate(trace).unwrap();

        assert_eq!(out.usage.kept.get(ContextSource::Conversation), 14);
        assert_eq!(out.usage.kept.get(ContextSource::Lorebook), 0);
        assert_eq!(contents(&out)[0], "m2 m2");
        assert_eq!(contents(&out).len(), 7);
    }

    #[test]
    fn the_lorebook_is_capped_at_its_share_even_with_budget_to_spare() {
        let entries: Vec<(String, i32)> = (0..10).map(|i| (sized("x", 5), i)).collect();
        let trace = vec![
            msg("a b"),
            lore(
                "setting",
                &entries
                    .iter()
                    .map(|(c, p)| (c.as_str(), *p))
                    .collect::<Vec<_>>(),
            ),
        ];
        let out = eval(100).evaluate(trace).unwrap();

        assert_eq!(out.usage.kept.get(ContextSource::Lorebook), 25);
    }

    #[test]
    fn lorebook_entries_are_shed_by_ascending_priority() {
        let trace = vec![lore(
            "setting",
            &[("low low", 1), ("high high", 9), ("mid mid", 5)],
        )];
        let out = eval(8).evaluate(trace).unwrap();

        assert_eq!(contents(&out), ["high high"]);
    }

    #[test]
    fn an_oversized_entry_is_dropped_rather_than_cut_short() {
        let trace = vec![lore("setting", &[(&sized("x", 100), 0)])];
        let out = eval(100).evaluate(trace).unwrap();

        assert_eq!(out.usage.kept.get(ContextSource::Lorebook), 0);
        assert!(out.trace.is_empty());
    }

    #[test]
    fn entry_priority_outranks_slot_order() {
        let trace = vec![
            lore("prelude", &[("keep keep", 9)]),
            lore("coda", &[("shed shed", 1)]),
        ];
        let out = eval(8).evaluate(trace).unwrap();

        assert_eq!(contents(&out), ["keep keep"]);
    }

    #[test]
    fn spare_budget_is_handed_out_by_priority() {
        let mut trace: Vec<TracedBlock> = (0..5)
            .map(|_| from("character", "example_messages", "e e"))
            .collect();
        trace.extend((0..5).map(|_| from("some_extension", "main", "p p")));

        let out = eval(14).evaluate(trace).unwrap();

        assert_eq!(out.usage.kept.get(ContextSource::CharacterExamples), 10);
        assert_eq!(out.usage.kept.get(ContextSource::Plugin), 4);
    }

    #[test]
    fn the_lorebook_is_funded_before_example_messages() {
        let entries: Vec<(String, i32)> = (0..10).map(|i| (sized("lore", 2), i)).collect();
        let mut trace = vec![lore(
            "setting",
            &entries
                .iter()
                .map(|(c, p)| (c.as_str(), *p))
                .collect::<Vec<_>>(),
        )];
        trace.extend((0..5).map(|_| from("character", "example_messages", "e e")));

        let out = eval(12).evaluate(trace).unwrap();

        assert_eq!(out.usage.kept.get(ContextSource::Lorebook), 2);
        assert_eq!(out.usage.kept.get(ContextSource::CharacterExamples), 8);
    }

    #[test]
    fn an_incompressible_floor_fails_before_anything_is_trimmed() {
        let trace = vec![
            text(&sized("tpl", 30)),
            from("character", "definition", &sized("char", 50)),
            msg("a b"),
        ];

        let err = eval(40).evaluate(trace).unwrap_err();
        let ContextError::BudgetExceeded {
            budget,
            required,
            breakdown,
        } = err
        else {
            panic!("expected BudgetExceeded, got {err:?}");
        };

        assert_eq!(budget, 40);
        assert_eq!(required, 82);
        assert_eq!(breakdown[0], ("character".to_string(), 50));
        assert_eq!(breakdown[1], ("template".to_string(), 30));
    }

    #[test]
    fn a_policy_override_replaces_the_default() {
        let entries: Vec<(String, i32)> = (0..10).map(|i| (sized("x", 2), i)).collect();
        let trace = vec![
            msg("a b"),
            lore(
                "setting",
                &entries
                    .iter()
                    .map(|(c, p)| (c.as_str(), *p))
                    .collect::<Vec<_>>(),
            ),
        ];
        let out = ContextEvaluator::builder(Arc::new(Words))
            .budget(100)
            ._policy(
                ContextSource::Lorebook,
                SourceBudget {
                    floor: Floor::Nothing,
                    cap: Cap::Tokens(4),
                    priority: 80,
                    order: TrimOrder::Newest,
                },
            )
            .build()
            .evaluate(trace)
            .unwrap();

        assert_eq!(out.usage.kept.get(ContextSource::Lorebook), 4);
    }

    #[test]
    fn adjacent_system_blocks_squash_when_asked() {
        let trace = vec![text("a"), text("b")];
        let out = ContextEvaluator::builder(Arc::new(Words))
            ._squash_system_messages(true)
            .build()
            .evaluate(trace)
            .unwrap();

        assert_eq!(out.messages.len(), 1);
        assert_eq!(out.messages[0].content, "a\nb");
        assert_eq!(out.trace.len(), 2);
    }

    /// The report is what the inspector reads, so it has to name both halves.
    #[test]
    fn the_report_records_what_was_shed_and_by_whom() {
        let mut trace: Vec<TracedBlock> =
            (1..=9).map(|i| msg(&sized(&format!("m{i}"), 2))).collect();
        trace.push(from("character", "definition", "sheet sheet"));

        // 18 tokens of history + 2 fixed, budget 16: two messages go.
        let out = eval(16).evaluate(trace).unwrap();

        assert!(out.usage.was_trimmed());
        assert_eq!(out.usage.trimmed.get(ContextSource::Conversation), 4);
        assert_eq!(out.usage.kept.get(ContextSource::Conversation), 14);
        // The incompressible sheet is untouched and absent from the shed side.
        assert_eq!(out.usage.kept.get(ContextSource::Character), 2);
        assert_eq!(out.usage.trimmed.get(ContextSource::Character), 0);
    }

    /// The inspector parses these keys by name (`types/context.ts`), so the
    /// serialized shape is a contract, not an implementation detail.
    #[test]
    fn the_report_serializes_to_the_shape_the_inspector_parses() {
        let trace = vec![msg("a b c d"), from("world_info", "setting", "e f")];
        let out = eval(4).evaluate(trace).unwrap();

        assert_eq!(
            serde_json::to_value(&out.usage).unwrap(),
            serde_json::json!({
                "budget": 4,
                "kept": { "conversation": 4 },
                "trimmed": { "lorebook": 2 },
            })
        );
    }

    #[test]
    fn an_untrimmed_turn_reports_nothing_shed() {
        let out = eval(100).evaluate(vec![msg("a b"), text("c")]).unwrap();

        assert!(!out.usage.was_trimmed());
        assert!(out.usage.trimmed.breakdown().is_empty());
    }

    #[test]
    fn the_returned_trace_matches_the_returned_prompt() {
        let trace: Vec<TracedBlock> = (1..=9).map(|i| msg(&sized(&format!("m{i}"), 2))).collect();
        let out = eval(14).evaluate(trace).unwrap();

        let from_trace: Vec<&str> = out.trace.iter().map(|t| t.block.content.as_str()).collect();
        let from_prompt: Vec<&str> = out.messages.iter().map(|m| m.content.as_str()).collect();
        assert_eq!(from_trace, from_prompt);
    }
}
