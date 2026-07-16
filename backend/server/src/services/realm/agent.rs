//! Cast members as behaviors. The turn loop resolves whose turn it is along
//! the path, then dispatches to that member's [`Agent`] impl to plan the turn.
//! Kinds: player, LLM, script, each registered in [`AgentRegistry`].

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use lorealm_storage::models::{
    ActivationState, AgentKind, AgentSpec, ChatRole, Conversation, EventKind, RealmAgent,
    RealmCast, ResolvedRealmConfig, Snowflake, StateDelta,
};
use serde_json::json;

use crate::{
    errors::RealmError,
    services::{
        context::PromptTurn,
        realm::{RealmService, weaver},
        state::{CapturedState, ReconstructedState},
        tools::ToolPolicy,
    },
};

/// The default broadcast channel every listener is granted on.
pub(super) const MAIN_SCOPE: &str = "main";

// ------------------------------------------------------------- turn shapes

/// Where a committed message lands in the conversation tree.
#[derive(Clone, Copy)]
pub(super) enum PersistPlan {
    /// Continue the active path (`None` = the playthrough's first message).
    Append(Option<Snowflake>),
    /// Regenerate: a new sibling of this message.
    Alternate(Snowflake),
}

/// One event a turn emits, before it is anchored to a message. The committer
/// stamps id/conversation/message/seq when it persists the turn.
pub(super) struct EventDraft {
    pub source: Snowflake,
    pub scope: String,
    pub kind: EventKind,
    pub content: String,
    pub payload: serde_json::Value,
}

/// The turn payload every agent kind resolves to, before persistence:
/// `events` are drafts and `state` persists only when present. The service's
/// `commit` turns this into a message + events + observations.
pub(super) struct Production {
    pub agent: RealmAgent,
    pub persist: PersistPlan,
    pub role: ChatRole,
    pub content: String,
    pub meta: serde_json::Value,
    pub state: Option<CapturedState>,
    pub events: Vec<EventDraft>,
    pub actor_notes: Vec<String>,
}

/// A turn already resolved into a production, with the conversation + cast
/// the committer needs.
pub(super) struct ReadyTurn {
    pub convo: Conversation,
    pub cast: RealmCast,
    pub production: Production,
}

/// A streamed LLM turn: assembled context inputs + tool policy, ready to infer
/// and then finalize into a production.
pub(super) struct LlmTurn {
    pub convo: Conversation,
    pub cast: RealmCast,
    pub agent: RealmAgent,
    pub spec: AgentSpec,
    pub baseline: ReconstructedState,
    pub policy: ToolPolicy,
    pub persist: PersistPlan,
    pub turns: Vec<PromptTurn>,
}

/// The resolved turn before execution; the turn loop matches on the shape.
pub(super) enum TurnPlan {
    /// A human is up; nothing runs server-side until they submit.
    AwaitPlayer { agent_id: Snowflake },
    /// A turn resolved and ready to commit.
    Ready(Box<ReadyTurn>),
    /// A streamed LLM turn.
    Stream(Box<LlmTurn>),
}

/// Everything an [`Agent`] needs to plan a turn: the resolved turn context
/// plus a handle back to the service for state/spec resolution.
pub(super) struct PlanCx<'a> {
    pub service: &'a RealmService,
    pub convo: &'a Conversation,
    pub cast: RealmCast,
    /// The cast member whose turn it is.
    pub agent: RealmAgent,
    /// Message ids along the active path (turn history + observation scope).
    pub path_ids: Vec<Snowflake>,
    /// The reconstructed world at the turn's baseline.
    pub world: ReconstructedState,
    pub persist: PersistPlan,
}

// ------------------------------------------------------------------- trait

/// One cast-member kind's turn behavior.
#[async_trait]
pub(super) trait Agent: Send + Sync {
    /// Resolve this agent's turn against the reconstructed world into a plan
    /// the turn loop can execute.
    async fn plan(&self, cx: PlanCx<'_>) -> Result<TurnPlan, RealmError>;
}

/// A human cast member: their turn is driven from the client
/// (`submit_player_turn`), so planning only signals the loop to wait.
pub(super) struct PlayerAgent;

#[async_trait]
impl Agent for PlayerAgent {
    async fn plan(&self, cx: PlanCx<'_>) -> Result<TurnPlan, RealmError> {
        Ok(TurnPlan::AwaitPlayer {
            agent_id: cx.agent.id,
        })
    }
}

/// A cast member that runs its inline weaver source against its granted state
/// slice; the resulting delta fans out as `StateChanged` events.
pub(super) struct ScriptAgent;

#[async_trait]
impl Agent for ScriptAgent {
    async fn plan(&self, cx: PlanCx<'_>) -> Result<TurnPlan, RealmError> {
        let PlanCx {
            service,
            convo,
            cast,
            agent,
            world,
            persist,
            ..
        } = cx;

        let (output, delta) = weaver::run_scoped(
            &service.state,
            agent.script.as_deref().unwrap_or_default(),
            &[],
            json!({}),
            &agent.grants,
            &world.persistent,
        )
        .await?;

        let meta = json!({
            "agent_id": agent.id,
            "agent_name": agent.name,
            "script": true,
            "output": output,
        });

        // A script speaks nothing; its only events are the state changes it
        // made.
        let config = service.resolve_config(convo).await?;
        let mut events = Vec::new();
        push_state_events(agent.id, &delta, &config, &mut events);
        let state = capture(world.activation, delta);

        Ok(TurnPlan::Ready(Box::new(ReadyTurn {
            convo: convo.clone(),
            cast,
            production: Production {
                agent,
                persist,
                role: ChatRole::Assistant,
                content: String::new(),
                meta,
                state,
                events,
                actor_notes: Vec::new(),
            },
        })))
    }
}

/// An LLM cast member: project its granted view of the world into a prompt and
/// hand back a streamed turn the loop drives to completion.
pub(super) struct LlmAgent;

#[async_trait]
impl Agent for LlmAgent {
    async fn plan(&self, cx: PlanCx<'_>) -> Result<TurnPlan, RealmError> {
        let PlanCx {
            service,
            convo,
            cast,
            agent,
            path_ids,
            world,
            persist,
        } = cx;

        let baseline_slice = weaver::slice(&world.persistent, &weaver::read_scopes(&agent.grants));

        let mut observations = service
            .storage
            .list_observations_for_messages(agent.id, &path_ids)
            .await?;
        // Path order == id order: along one path every message commits after
        // its ancestors, and observations are created with their message.
        if observations.len() > super::OBSERVATION_WINDOW as usize {
            observations.drain(..observations.len() - super::OBSERVATION_WINDOW as usize);
        }
        let turns: Vec<PromptTurn> = observations
            .iter()
            .map(PromptTurn::from_observation)
            .collect();

        // Turns use the conversation owner's active tokenizer.
        let tokenizer_id = service.state.get_active_tokenizer(convo.user_id).await?.id;
        let spec = service.spec_for(convo.user_id, tokenizer_id, &agent)?;

        let policy = ToolPolicy {
            write_scopes: Some(Arc::new(weaver::write_scopes(&agent.grants))),
            tool_ids: Some(agent.tools.iter().map(|b| b.tool_id).collect()),
        };
        let baseline = ReconstructedState {
            persistent: baseline_slice,
            activation: world.activation,
        };

        Ok(TurnPlan::Stream(Box::new(LlmTurn {
            convo: convo.clone(),
            cast,
            agent,
            spec,
            baseline,
            policy,
            persist,
            turns,
        })))
    }
}

// ---------------------------------------------------------------- registry

/// Maps each agent kind to its behavior.
pub(super) struct AgentRegistry {
    behaviors: HashMap<AgentKind, Arc<dyn Agent>>,
}

impl AgentRegistry {
    pub(super) fn with_defaults() -> Self {
        let mut behaviors: HashMap<AgentKind, Arc<dyn Agent>> = HashMap::new();
        behaviors.insert(AgentKind::Player, Arc::new(PlayerAgent));
        behaviors.insert(AgentKind::Llm, Arc::new(LlmAgent));
        behaviors.insert(AgentKind::Script, Arc::new(ScriptAgent));
        Self { behaviors }
    }

    /// The behavior for `kind`. Every `AgentKind` is registered in
    /// `with_defaults`, so a miss is a construction bug.
    pub(super) fn get(&self, kind: &AgentKind) -> Arc<dyn Agent> {
        Arc::clone(&self.behaviors[kind])
    }
}

// ----------------------------------------------------------------- helpers

/// The scope an agent's speech carries: its configured channel, or the
/// broadcast `main` channel by default.
pub(super) fn speech_scope(agent: &RealmAgent) -> String {
    agent
        .speech_scope
        .clone()
        .unwrap_or_else(|| MAIN_SCOPE.to_string())
}

/// Wrap a turn's writes as persistable state, only when there is something to
/// persist. `activation` is carried forward so a state row never resets
/// lorebook activation for later turns.
pub(super) fn capture(
    activation: HashMap<Snowflake, ActivationState>,
    delta: StateDelta,
) -> Option<CapturedState> {
    (!delta.writes.is_empty() || !delta.deletes.is_empty()).then(|| CapturedState {
        activation,
        persistent: delta,
    })
}

/// Append one `StateChanged` event draft per namespace the delta touched.
pub(super) fn push_state_events(
    source: Snowflake,
    delta: &StateDelta,
    config: &ResolvedRealmConfig,
    events: &mut Vec<EventDraft>,
) {
    let mut namespaces: Vec<&String> = delta.writes.keys().chain(delta.deletes.keys()).collect();
    namespaces.sort();
    namespaces.dedup();

    for ns in namespaces {
        let mut parts = Vec::new();
        if let Some(vars) = delta.writes.get(ns) {
            let mut keys: Vec<&String> = vars.keys().collect();
            keys.sort();
            for k in keys {
                parts.push(config.state_write(k, &vars[k].to_string()));
            }
        }
        if let Some(names) = delta.deletes.get(ns) {
            for n in names {
                parts.push(config.state_delete(n));
            }
        }
        events.push(EventDraft {
            source,
            scope: ns.clone(),
            kind: EventKind::StateChanged,
            content: config.state_change(ns, &parts.join(", ")),
            payload: json!({ "writes": delta.writes.get(ns), "deletes": delta.deletes.get(ns) }),
        });
    }
}
