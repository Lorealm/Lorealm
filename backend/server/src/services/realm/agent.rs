use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use lorealm_storage::models::{
    ActivationState, AgentKind, AgentSpec, ChatRole, Conversation, EventKind, RealmAgent,
    RealmCast, Snowflake, StateDelta,
};
use serde_json::json;

use crate::{
    errors::RealmError,
    services::{
        context::PromptTurn,
        plugins::PluginVocabulary,
        realm::{RealmService, weaver},
        state::{CapturedState, ReconstructedState},
        tools::{EmittedEvent, RealmView, ToolPolicy},
    },
};

/// The default broadcast channel every listener is granted on.
pub(super) const MAIN_CHANNEL: &str = "main";

/// Where a committed message lands in the conversation tree.
#[derive(Clone, Copy)]
pub(super) enum PersistPlan {
    /// Continue the active path (`None` = the playthrough's first message).
    Append(Option<Snowflake>),
    /// Regenerate: a new sibling of this message.
    Alternate(Snowflake),
}

pub(super) struct EventDraft {
    pub source: String,
    pub channel: String,
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

pub(super) struct ReadyTurn {
    pub convo: Conversation,
    pub cast: RealmCast,
    pub production: Production,
}

pub(super) struct LlmTurn {
    pub convo: Conversation,
    pub cast: RealmCast,
    pub agent: RealmAgent,
    pub spec: AgentSpec,
    pub baseline: ReconstructedState,
    pub policy: ToolPolicy,
    pub persist: PersistPlan,
    pub turns: Vec<PromptTurn>,
    pub widget_prompts: Vec<String>,
    pub widget_renders: Vec<String>,
}

/// The resolved turn before execution; the turn loop matches on the shape.
pub(super) enum TurnPlan {
    /// A human is up; nothing runs server-side until they submit.
    AwaitPlayer { handle: String },
    /// A turn resolved and ready to commit.
    Ready(Box<ReadyTurn>),
    /// A streamed LLM turn.
    Stream(Box<LlmTurn>),
}

/// Everything an [`Agent`] needs to plan a turn
pub(super) struct PlanCx<'a> {
    pub service: &'a RealmService,
    pub convo: &'a Conversation,
    pub cast: RealmCast,
    /// The cast member whose turn it is.
    pub agent: RealmAgent,
    /// The `cast:`/`turn:` mount every body on this turn reads.
    pub view: RealmView,
    /// Message ids along the active path (turn history + observation scope).
    pub path_ids: Vec<Snowflake>,
    /// The reconstructed world at the turn's baseline.
    pub world: ReconstructedState,
    pub persist: PersistPlan,
    /// The owner's plugin vocabulary, for script bodies.
    pub plugins: Option<PluginVocabulary>,
}

// An Agent capable of acting within a Realm
#[async_trait]
pub(super) trait Agent: Send + Sync {
    async fn plan(&self, cx: PlanCx<'_>) -> Result<TurnPlan, RealmError>;
}

/// A human cast member: their turn is driven from the client
/// (`submit_player_turn`), so planning only signals the loop to wait.
pub(super) struct PlayerAgent;

#[async_trait]
impl Agent for PlayerAgent {
    async fn plan(&self, cx: PlanCx<'_>) -> Result<TurnPlan, RealmError> {
        Ok(TurnPlan::AwaitPlayer {
            handle: cx.agent.handle.clone(),
        })
    }
}

/// An agent that runs a script and nothing else
pub(super) struct ScriptAgent;

#[async_trait]
impl Agent for ScriptAgent {
    async fn plan(&self, cx: PlanCx<'_>) -> Result<TurnPlan, RealmError> {
        let PlanCx {
            service: _,
            convo,
            cast,
            agent,
            view,
            world,
            persist,
            plugins,
            ..
        } = cx;

        let (output, delta, emitted) = weaver::run_scoped(
            agent.script.as_deref().unwrap_or_default(),
            &[],
            json!({}),
            &agent,
            &world.persistent,
            &view,
            plugins.as_ref(),
        )
        .await?;

        let meta = json!({
            "handle": agent.handle,
            "agent_name": agent.name,
            "script": true,
            "output": output,
        });

        let mut events = Vec::new();
        push_emitted_events(&agent.handle, emitted, &mut events);
        push_state_events(&agent.handle, &delta, &mut events);
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

/// An LLM-driven agent
pub(super) struct LlmAgent;

#[async_trait]
impl Agent for LlmAgent {
    async fn plan(&self, cx: PlanCx<'_>) -> Result<TurnPlan, RealmError> {
        let PlanCx {
            service,
            convo,
            cast,
            agent,
            view,
            path_ids,
            world,
            persist,
            plugins: _,
        } = cx;

        let mut baseline_slice = weaver::slice(&world.persistent, &weaver::read_scopes_of(&agent));
        weaver::inject_self_alias(&agent, &mut baseline_slice);

        let mut observations = service
            .storage
            .list_observations_for_messages(&agent.handle, &path_ids)
            .await?;

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

        // The agent's bound tools plus the tools its widgets bundle.
        let bindings = service.effective_bindings(&agent).await?;
        let (widget_prompts, widget_renders, _) = service.widget_directives(&agent).await?;

        let policy = ToolPolicy {
            tool_ids: Some(bindings.iter().map(|b| b.tool_id).collect()),
            extra_tool_ids: Vec::new(),
            actor: Some(weaver::actor_of(&agent)),
            realm: Some(view),
            event_sink: Some(Arc::new(std::sync::Mutex::new(Vec::new()))),
            // Realm agents do not offer attachments yet.
            attachments: Vec::new(),
            conversation: None,
            cancel: None,
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
            widget_prompts,
            widget_renders,
        })))
    }
}

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

/// The agent's configured channel
pub(super) fn speech_channel(agent: &RealmAgent) -> String {
    agent
        .speech_channel
        .clone()
        .unwrap_or_else(|| MAIN_CHANNEL.to_string())
}

pub(super) fn capture(
    activation: HashMap<Snowflake, ActivationState>,
    delta: StateDelta,
) -> Option<CapturedState> {
    (!delta.writes.is_empty() || !delta.deletes.is_empty()).then(|| CapturedState {
        activation,
        persistent: delta,
    })
}

/// Append one draft per event a weaver body emitted via `emit_event`.
pub(super) fn push_emitted_events(
    source: &str,
    emitted: Vec<EmittedEvent>,
    events: &mut Vec<EventDraft>,
) {
    events.extend(emitted.into_iter().map(|e| EventDraft {
        source: source.to_string(),
        channel: e.scope,
        kind: e.kind,
        content: e.content,
        payload: json!({}),
    }));
}

pub(super) fn push_state_events(
    source: &str,
    delta: &StateDelta,
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
                parts.push(format!("{k} = {}", vars[k]));
            }
        }
        if let Some(names) = delta.deletes.get(ns) {
            for n in names {
                parts.push(format!("-{n}"));
            }
        }
        events.push(EventDraft {
            source: source.to_string(),
            channel: ns.clone(),
            kind: EventKind::StateChanged,
            content: format!("{ns}: {}", parts.join(", ")),
            payload: json!({ "writes": delta.writes.get(ns), "deletes": delta.deletes.get(ns) }),
        });
    }
}
