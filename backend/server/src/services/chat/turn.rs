use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex as StdMutex},
};

use futures_util::{Stream, StreamExt};
use lorealm_storage::models::{
    ActivationState, AgentKind, AgentSpec, AgentToolBinding, ChatRole, Conversation,
    ConversationMessage, EventKind, RealmAgent, RealmCast, ResolvedRealmConfig, Snowflake,
    StateDelta, TurnEvent, UserPersona, UserTool,
};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::{
    errors::{ChatError, RealmError, TargetType},
    services::{
        chat::{
            ChatService, ChatStreamEvent, PlayerAction, PromptMessage, TurnAccumulator,
            TurnRequest, WidgetRenderer,
            perceive::{observe_events, observers, perceive},
            stage::{Stage, handle_of},
            types::Prepared,
            weaver,
            widgets::InferEvent,
        },
        commands::CommandEffect,
        context_template::{PromptRole, TracedBlock},
        plugins::UserVocabulary,
        schedule::{FilterChain, StageCx},
        state::{CapturedState, ReconstructedState, StatePool, apply_delta},
        tools::{EmittedEvent, EventSink, RealmVariables, ToolPolicy, builtin_tool},
    },
    util::context_eval::UsageReport,
};

type TurnStream = Pin<Box<dyn Stream<Item = Result<ChatStreamEvent, ChatError>> + Send>>;

/// Where a turn's message lands in the tree.
enum Relationship {
    /// A new child of this message. `None` starts the tree.
    Child(Option<Snowflake>),
    /// A new alternative beside this message. e.g., a swipe
    Sibling(ConversationMessage),
    /// An extension/contiuniation of this message
    Continuation(ConversationMessage),
}

/// A message produced by a turn. Derived from [`ActiveTurn`].
struct TurnMessage {
    agent: RealmAgent,
    place: Relationship,
    role: ChatRole,
    content: String,
    meta: serde_json::Value,
    /// The prompt a model reply was built from, kept apart from `meta`.
    context: Option<serde_json::Value>,
    state: Option<CapturedState>,
    events: Vec<TurnEvent>,
    tool_results: Vec<String>,
}

/// An in-progress turn.
struct ActiveTurn {
    stream: Pin<Box<dyn Stream<Item = Result<InferEvent, ChatError>> + Send>>,
    /// Undoes on the reply whatever the outbound schedule did to the prompt.
    filter: FilterChain,
    acc: TurnAccumulator,
    spec: AgentSpec,
    trace: Vec<TracedBlock>,
    usage: UsageReport,
    squash_messages: bool,
    available_tools: Vec<String>,
    pool: Arc<StdMutex<StatePool>>,
    diff_base: StatePool,
    activation: HashMap<Snowflake, ActivationState>,
    had_assembly: bool,
    emitted: EventSink,
}

fn message_ids(path: &[ConversationMessage]) -> Vec<Snowflake> {
    path.iter().map(|m| m.id).collect()
}

fn extract_captured_state(
    activation: HashMap<Snowflake, ActivationState>,
    delta: StateDelta,
) -> Option<CapturedState> {
    if !delta.writes.is_empty() || !delta.deletes.is_empty() {
        Some(CapturedState {
            activation,
            persistent: delta,
        })
    } else {
        None
    }
}

fn newest_player_images(path: &[ConversationMessage]) -> Vec<String> {
    path.iter()
        .rev()
        .find(|m| m.role == ChatRole::User)
        .and_then(|m| m.metadata.get("images"))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

fn emitted_events(emitted: Vec<EmittedEvent>) -> impl Iterator<Item = TurnEvent> {
    emitted.into_iter().map(|e| TurnEvent {
        channel: e.scope,
        kind: e.kind,
        content: e.content,
        observers: None,
    })
}

impl ChatService {
    /// Advance a conversation by one request and stream what happens.
    pub async fn turn(
        self: &Arc<Self>,
        convo_id: Snowflake,
        user_id: Snowflake,
        request: TurnRequest,
    ) -> Result<TurnStream, ChatError> {
        let convo = self.get_conversation(convo_id, user_id).await?;
        let cast = self.cast_of(convo_id).await?;
        let (cancel, guard) = self.begin_cancel(convo_id);
        let service = Arc::clone(self);

        let stream = async_stream::try_stream! {
            let _guard = guard; // the stream owns the cancellation slot
            let TurnRequest { content, actions, images, mut swipe_from, continued } = request;
            let mut leaf = convo.active_leaf_id;

            // The player's own turn, if the request carries one.
            let content = content.filter(|c| !c.trim().is_empty());
            if content.is_some() || !actions.is_empty() || !images.is_empty() {
                let place = match swipe_from.take() {
                    Some(id) => Relationship::Sibling(service.owned_message(id, convo.id).await?),
                    None => Relationship::Child(leaf),
                };
                let message = service
                    .player_turn(&convo, &cast, place, content.unwrap_or_default(), actions, images)
                    .await?;
                leaf = Some(message.id);
                yield ChatStreamEvent::Message { message };
            }

            // Infer the new message's relationship to the target message
            let place = match (continued, swipe_from) {
                (true, id) => {
                    let id = id.or(leaf).ok_or(ChatError::NotFound(TargetType::Message, convo.id))?;
                    Relationship::Continuation(service.owned_message(id, convo.id).await?)
                }
                (false, Some(id)) => Relationship::Sibling(service.owned_message(id, convo.id).await?),
                (false, None) => Relationship::Child(leaf),
            };
            let stem = service.path_to(convo.id, &place).await?;
            let before = match &place {
                Relationship::Continuation(_) => &stem[..stem.len() - 1],
                _ => &stem[..],
            };
            let world = service.state.reconstruct(&message_ids(before)).await?;
            let plugins = service.plugins.vocabulary_with(convo.user_id, &cast.modules).await;

            let (acting_agent, view) = {
                let stage = Stage::new(&cast, before, &world.persistent).with_plugins(plugins.clone());
                let agent = match &place {
                    Relationship::Child(_) => stage.next_up()?,
                    Relationship::Sibling(m) | Relationship::Continuation(m) => match handle_of(m) {
                        Some(handle) => cast.agents.iter().find(|a| a.handle == handle),
                        None => stage.next_up()?,
                    },
                };
                (agent.cloned(), stage.view.clone())
            };

            // A player being up, or nobody, leaves nothing to run.
            if let Some(agent) = acting_agent.filter(|a| a.kind != AgentKind::Player) {
                service.announce_turn_start(&convo, &agent);
                yield ChatStreamEvent::TurnStarted {
                    handle: agent.handle.clone(),
                    name: agent.name.clone(),
                    observers: observers(&cast, &world.persistent, &agent.handle),
                };
                let before_turn = world.persistent.clone();

                let is_llm = agent.kind == AgentKind::Llm;
                let mut notes = Vec::new();

                let mut images: Vec<String> = match &place {
                    Relationship::Continuation(prior) => prior
                        .metadata
                        .get("images")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default(),
                    _ => Vec::new(),
                };
                let production = if is_llm {
                    let mut open = service
                        .open_model_turn(&convo, &cast, &agent, &place, &stem, view, cancel.clone())
                        .await?;

                    loop {
                        // NOTE: `biased;` means `tokio::select!` checks branches in branches in the order
                        // they were created. This guarantees cancellation always wins.
                        let next = tokio::select! {
                            biased;
                            _ = cancel.cancelled() => None,
                            next = open.stream.next() => next,
                        };
                        let Some(event) = next else { break };
                        // `?` aborts the stream
                        match event? {
                            InferEvent::Text(delta) => {
                                let emitted = open.filter.push(&delta);
                                if !emitted.is_empty() {
                                    open.acc.text(&emitted);
                                    yield ChatStreamEvent::Delta { content: emitted };
                                }
                            }
                            InferEvent::Reasoning(delta) => {
                                let filtered = open.filter.apply(&delta);
                                if !filtered.is_empty() {
                                    open.acc.reasoning(&filtered);
                                    yield ChatStreamEvent::Reasoning { content: filtered };
                                }
                            }
                            InferEvent::Widget(widget) => {
                                let widget = widget.filtered(|s| open.filter.apply(s));
                                yield ChatStreamEvent::Widget { widget: widget.clone() };
                                let marker = open.acc.widget(widget);
                                yield ChatStreamEvent::Delta { content: marker };
                            }
                            InferEvent::ToolCall(record) => open.acc.tool_call(record),
                            InferEvent::Effect(CommandEffect::Sys { content }) => notes.push(content),
                            InferEvent::Effect(CommandEffect::Image { uri }) => images.push(uri),
                            InferEvent::Effect(effect) => yield ChatStreamEvent::Effect { effect },
                        }
                    }
                    let tail = open.filter.finalize();
                    if !tail.is_empty() {
                        open.acc.text(&tail);
                        yield ChatStreamEvent::Delta { content: tail };
                    }

                    let mut production = service.close_model_turn(&convo, &cast, agent, place, open).await?;
                    if !images.is_empty() {
                        production.meta["images"] = json!(images);
                    }
                    production
                } else {
                    if let Relationship::Continuation(prior) = &place {
                        Err(ChatError::NotContinuable(prior.id))?;
                    }
                    service.script_turn(agent, place, &view, world, plugins.as_ref()).await?
                };

                let message = service.commit(&convo, &cast, &before_turn, production).await?;
                if is_llm {
                    let plugins = Arc::clone(&service.plugins);
                    let (owner, convo_id) = (convo.user_id, convo.id);
                    let (msg_id, msg_content) = (message.id, message.content.clone());
                    tokio::spawn(async move {
                        plugins.after_generate(owner, convo_id, msg_id, msg_content).await;
                    });
                }
                yield ChatStreamEvent::Message { message };

                for content in notes {
                    let message = service
                        .conversations
                        .append_message(convo.id, ChatRole::System, content, json!({}))
                        .await?;
                    yield ChatStreamEvent::Message { message };
                }
            }

            yield ChatStreamEvent::Done { next: service.whose_turn(convo.id, &cast).await? };
        };

        Ok(Box::pin(stream))
    }

    /// Whose turn it is at the active leaf.
    pub(super) async fn whose_turn(
        &self,
        convo_id: Snowflake,
        cast: &RealmCast,
    ) -> Result<Option<String>, ChatError> {
        let convo = self
            .storage
            .get_conversation(convo_id)
            .await?
            .ok_or(ChatError::NotFound(TargetType::Conversation, convo_id))?;
        let path = self
            .path_to(convo_id, &Relationship::Child(convo.active_leaf_id))
            .await?;
        let world = self.state.reconstruct(&message_ids(&path)).await?;
        let plugins = self
            .plugins
            .vocabulary_with(convo.user_id, &cast.modules)
            .await;

        Ok(Stage::new(cast, &path, &world.persistent)
            .with_plugins(plugins)
            .next_up()?
            .map(|a| a.handle.clone()))
    }

    /// A list of messages that will act as the new stem of the message. Depends
    /// on the [`Relationship`] parameter:
    ///
    /// - [`Relationship::Child`] => returns itself
    /// - [`Relationship::Sibling`] => returns its parent
    /// - [`Relationship::Continuation`] => returns itself
    async fn path_to(
        &self,
        convo_id: Snowflake,
        place: &Relationship,
    ) -> Result<Vec<ConversationMessage>, ChatError> {
        let end = match place {
            Relationship::Child(leaf) => *leaf,
            Relationship::Sibling(message) => message.parent,
            Relationship::Continuation(message) => Some(message.id),
        };
        Ok(match end {
            Some(id) => self.conversations.path_to(convo_id, id).await?,
            None => Vec::new(),
        })
    }

    /// Fires the `turn-start` hook in all the owner's plugins.
    fn announce_turn_start(&self, convo: &Conversation, agent: &RealmAgent) {
        let plugins = Arc::clone(&self.plugins);
        let (user_id, convo_id, handle) = (convo.user_id, convo.id, agent.handle.clone());
        tokio::spawn(async move { plugins.turn_start(user_id, convo_id, handle).await });
    }

    /// A player says something, triggers a bound tool, or both.
    async fn player_turn(
        &self,
        convo: &Conversation,
        cast: &RealmCast,
        place: Relationship,
        content: String,
        actions: Vec<PlayerAction>,
        images: Vec<String>,
    ) -> Result<ConversationMessage, ChatError> {
        self.check_images(&images).await?;
        let path = self.path_to(convo.id, &place).await?;
        let world = self.state.reconstruct(&message_ids(&path)).await?;
        let plugins = self
            .plugins
            .vocabulary_with(convo.user_id, &cast.modules)
            .await;
        let stage = Stage::new(cast, &path, &world.persistent).with_plugins(plugins.clone());
        let agent = stage.player()?.clone();

        let content = match content.is_empty() {
            true => content,
            false => {
                self.plugins
                    .user_message(convo.user_id, convo.id, content)
                    .await?
            }
        };
        self.announce_turn_start(convo, &agent);

        let config = ResolvedRealmConfig::from_config(&cast.config);
        let mut working = world.persistent.clone();
        let mut events = Vec::new();
        let mut notes = Vec::new();
        let mut done = Vec::new();
        for action in actions {
            match action {
                PlayerAction::Tool { tool_id, args } => {
                    // A player may trigger any tool its agent is bound to,
                    // including those bundled by a widget.
                    let binding = self
                        .tool_bindings(&agent)
                        .await?
                        .into_iter()
                        .find(|b| b.tool_id == tool_id)
                        .ok_or(RealmError::ToolNotBound(tool_id))?;
                    let tool = self
                        .bound_tool(binding.tool_id)
                        .await?
                        .ok_or(ChatError::NotFound(TargetType::UserTool, binding.tool_id))?;
                    let (output, changed, emitted) = weaver::run_scoped(
                        &tool.body,
                        &tool.params,
                        args.clone(),
                        &agent,
                        &working,
                        &stage.view,
                        plugins.as_ref(),
                    )
                    .await?;
                    apply_delta(&mut working, &changed);

                    events.push(TurnEvent {
                        channel: binding.event_channel.unwrap_or_default(),
                        kind: EventKind::Acted,
                        content: config.action(&agent.name, &tool.name),
                        observers: None,
                    });
                    events.extend(emitted_events(emitted));
                    notes.push(config.tool_result(&tool.name, &output));
                    done.push(json!({ "tool": tool.name, "input": args, "output": output }));
                }
                PlayerAction::Run { run } => {
                    // `run_scoped` uses the RealmScript profile: no
                    // `generation`, no `effects`. Its output goes nowhere.
                    let (_, changed, emitted) = weaver::run_scoped(
                        &run,
                        &[],
                        json!({}),
                        &agent,
                        &working,
                        &stage.view,
                        plugins.as_ref(),
                    )
                    .await?;
                    apply_delta(&mut working, &changed);
                    events.extend(emitted_events(emitted));
                    done.push(json!({ "run": run }));
                }
            }
        }
        let mut meta = json!({});
        if !done.is_empty() {
            meta["actions"] = json!(done);
        }
        if !images.is_empty() {
            meta["images"] = json!(images);
        }
        let delta = self.state.diff(&world.persistent, &working);

        self.commit(
            convo,
            cast,
            &world.persistent,
            TurnMessage {
                agent,
                place,
                role: ChatRole::User,
                content,
                meta,
                context: None,
                // Carried forward so a player's state row never resets
                // lorebook activation for the turns after it.
                state: extract_captured_state(world.activation, delta),
                events,
                tool_results: notes,
            },
        )
        .await
    }

    /// A script agent's turn: its body runs over the world and says nothing.
    async fn script_turn(
        &self,
        agent: RealmAgent,
        place: Relationship,
        view: &RealmVariables,
        world: ReconstructedState,
        plugins: Option<&UserVocabulary>,
    ) -> Result<TurnMessage, ChatError> {
        let (output, delta, emitted) = weaver::run_scoped(
            agent.script.as_deref().unwrap_or_default(),
            &[],
            json!({}),
            &agent,
            &world.persistent,
            view,
            plugins,
        )
        .await?;

        Ok(TurnMessage {
            agent,
            place,
            role: ChatRole::Assistant,
            content: String::new(),
            meta: json!({ "script": true, "output": output }),
            context: None,
            state: extract_captured_state(world.activation, delta),
            events: emitted_events(emitted).collect(),
            tool_results: Vec::new(),
        })
    }

    /// Assemble what `agent` perceives and start the model on it.
    #[allow(clippy::too_many_arguments)]
    async fn open_model_turn(
        &self,
        convo: &Conversation,
        cast: &RealmCast,
        agent: &RealmAgent,
        place: &Relationship,
        path: &[ConversationMessage],
        view: RealmVariables,
        cancel: CancellationToken,
    ) -> Result<ActiveTurn, ChatError> {
        let owner = convo.user_id;
        let acc = match place {
            Relationship::Continuation(prior) if prior.role == ChatRole::Assistant => {
                TurnAccumulator::resuming(prior)
            }
            Relationship::Continuation(prior) => return Err(ChatError::NotContinuable(prior.id)),
            _ => TurnAccumulator::default(),
        };
        let (spec, persona) = self.spec_for(owner, cast, agent).await?;

        let mut widget_ids = agent.widgets.clone();
        // The final produce is a union of: the globally active tools, the agent's,
        // all widgets' and the character's attached tools.
        let mut tool_ids: Vec<Snowflake> = agent.tools.iter().map(|b| b.tool_id).collect();

        // Extend with the character's attached widgets/tools
        if let Some((id, version)) = spec.character {
            let character = self.characters.load_version(id, version, owner).await?;
            widget_ids.extend(character.widget_ids);
            tool_ids.extend(character.tool_ids);
        }
        let active_widgets = self.active_widgets(owner, &widget_ids).await?;
        tool_ids.extend(active_widgets.tools);

        // The agent reads the world through its grants.
        let scoped = |state: ReconstructedState| {
            let mut persistent = weaver::slice(&state.persistent, &weaver::read_scopes_of(agent));
            weaver::inject_self_alias(agent, &mut persistent);
            ReconstructedState {
                persistent,
                activation: state.activation,
            }
        };
        let baseline = scoped(self.state.reconstruct(&message_ids(path)).await?);
        // A continuation rewrites the state row of the message it extends.
        let before = match place {
            Relationship::Continuation(_) => &path[..path.len() - 1],
            _ => path,
        };
        let diff_base = scoped(self.state.reconstruct(&message_ids(before)).await?).persistent;

        let mut renderer =
            WidgetRenderer::new(&active_widgets.attachments, baseline.persistent.clone())
                .with_plugins(self.plugins.vocabulary_with(owner, &cast.modules).await);
        let turns = perceive(cast, agent, path, &mut renderer);

        let Prepared {
            mut prompt,
            trace,
            captured,
            usage,
            squash_messages,
        } = self
            .assemble_context(
                &spec,
                cast,
                agent,
                &view,
                Some(convo.id),
                &turns,
                &baseline,
                &active_widgets.prompts,
                &active_widgets.renders,
                &convo.author_note,
            )
            .await?;

        let mut working = diff_base.clone();
        let (activation, had_assembly) = match captured {
            Some(captured) => {
                apply_delta(&mut working, &captured.persistent);
                (captured.activation, true)
            }
            None => (baseline.activation, false),
        };
        let pool = Arc::new(StdMutex::new(working));

        let filter = self.schedule.outbound(&mut prompt, &StageCx { persona });
        if matches!(place, Relationship::Continuation(_)) {
            // This is an ephemeral prompt. It's never persisted.
            prompt.push(PromptMessage {
                role: PromptRole::Assistant,
                // TODO: Should be folded into a config
                content: "[Continue your latest message without repeating its content]".into(),
            });
        }

        let images = newest_player_images(path);
        let emitted = EventSink::default();
        let (available_tools, stream) = self
            .infer(
                owner,
                spec.profile_id,
                prompt,
                &images,
                Arc::clone(&pool),
                &ToolPolicy {
                    tool_ids: None,
                    extra_tool_ids: tool_ids,
                    actor: Some(weaver::actor_of(agent)),
                    realm: Some(view),
                    event_sink: Some(Arc::clone(&emitted)),
                    attachments: active_widgets.attachments,
                    conversation: Some(convo.id),
                    cancel: Some(cancel),
                    modules: cast.modules.clone(),
                },
            )
            .await?;

        Ok(ActiveTurn {
            stream,
            filter,
            acc,
            spec,
            trace,
            usage,
            squash_messages,
            available_tools,
            pool,
            diff_base,
            activation,
            had_assembly,
            emitted,
        })
    }

    /// Turn a finished model stream into the turn it produced.
    async fn close_model_turn(
        &self,
        convo: &Conversation,
        cast: &RealmCast,
        agent: RealmAgent,
        place: Relationship,
        open: ActiveTurn,
    ) -> Result<TurnMessage, ChatError> {
        let mut outcome = open.acc.finish();
        outcome.spoken = self
            .plugins
            .message_finalize(convo.user_id, convo.id, outcome.spoken)
            .await;

        let config = ResolvedRealmConfig::from_config(&cast.config);

        let mut bound_tools = HashMap::new();
        for binding in &agent.tools {
            if let Some(tool) = self.bound_tool(binding.tool_id).await? {
                bound_tools.insert(tool.name, binding.event_channel.clone());
            }
        }

        let mut events = Vec::new();
        let mut tool_results = Vec::new();
        for call in outcome.tool_calls.iter().filter(|c| !c.is_error) {
            let Some(channel) = bound_tools.get(&call.name) else {
                continue;
            };
            events.push(TurnEvent {
                channel: channel.clone().unwrap_or_default(),
                kind: EventKind::Acted,
                content: config.action(&agent.name, &call.name),
                observers: None,
            });
            tool_results.push(config.tool_result(&call.name, &call.output.to_string()));
        }
        events.extend(emitted_events(std::mem::take(
            &mut *open.emitted.lock().unwrap(),
        )));

        let mut stored = open.trace;
        stored.iter_mut().for_each(TracedBlock::strip_recoverable);

        let delta = {
            let pool = open.pool.lock().unwrap();
            self.state.diff(&open.diff_base, &pool)
        };
        let changed = !delta.writes.is_empty() || !delta.deletes.is_empty();

        Ok(TurnMessage {
            agent,
            place,
            role: ChatRole::Assistant,
            content: outcome.spoken,
            meta: json!({
                "has_context": true,
                "context_usage": open.usage,
                "squash_messages": open.squash_messages,
                "reasoning": outcome.reasoning,
                "tool_calls": outcome.tool_calls,
                "widgets": outcome.widgets,
                "conn_profile": open.spec.profile_id,
                "tok_id": open.spec.tokenizer_id,
                "template_id": open.spec.template_id,
                "available_tools": open.available_tools,
            }),
            context: Some(json!(stored)),
            state: (open.had_assembly || changed).then_some(CapturedState {
                activation: open.activation,
                persistent: delta,
            }),
            events,
            tool_results,
        })
    }

    /// Commit a turn message to the conversation. `before` is the world the
    /// turn started from; who observed it is decided by the world it left.
    async fn commit(
        &self,
        convo: &Conversation,
        cast: &RealmCast,
        before: &StatePool,
        msg: TurnMessage,
    ) -> Result<ConversationMessage, ChatError> {
        let TurnMessage {
            agent,
            place,
            role,
            content,
            mut meta,
            context,
            state,
            mut events,
            tool_results,
        } = msg;

        let mut after = before.clone();
        if let Some(captured) = &state {
            apply_delta(&mut after, &captured.persistent);
        }
        observe_events(cast, &after, &agent.handle, &mut events);

        // TODO: there is probably a world where a seperate metadata factory makes
        // sense. It would handle conflicts and error reporting, while also cleaning
        // up the top level API. Alas, I am lazy.
        meta["handle"] = json!(agent.handle);
        meta["agent_name"] = json!(agent.name);
        meta["observers"] = json!(observers(cast, &after, &agent.handle));
        if !events.is_empty() {
            meta["events"] = json!(events);
        }
        if !tool_results.is_empty() {
            meta["tool_results"] = json!(tool_results);
        }

        let message = match place {
            Relationship::Child(Some(parent)) => {
                self.conversations
                    .branch_from(convo.id, parent, role, content, meta)
                    .await?
            }
            Relationship::Child(None) => {
                self.conversations
                    .append_message(convo.id, role, content, meta)
                    .await?
            }
            Relationship::Sibling(sibling) => {
                self.conversations
                    .create_alternate(convo.id, sibling.id, role, content, meta)
                    .await?
            }
            Relationship::Continuation(prior) => {
                self.conversations
                    .rewrite_message(prior.id, content, meta)
                    .await?
            }
        };

        if let Some(context) = context {
            self.storage
                .put_message_context(message.id, &context)
                .await?;
        }
        if let Some(captured) = state {
            self.state.persist(message.id, captured).await?;
        }

        let plugins = Arc::clone(&self.plugins);
        let (user_id, convo_id, handle) = (convo.user_id, convo.id, agent.handle);
        let (message_id, said) = (message.id, message.content.clone());
        tokio::spawn(async move {
            plugins
                .turn_end(user_id, convo_id, handle, message_id, said)
                .await;
        });

        Ok(message)
    }

    /// Returns an `AgentSpec` and the effective `UserPersona` for the given agent,
    /// within the given cast.
    pub(super) async fn spec_for(
        &self,
        owner: Snowflake,
        cast: &RealmCast,
        agent: &RealmAgent,
    ) -> Result<(AgentSpec, UserPersona), ChatError> {
        let pinned = agent.persona_id.or_else(|| {
            cast.agents
                .iter()
                .find(|a| a.kind == AgentKind::Player)
                .and_then(|player| player.persona_id)
        });
        let persona = match (pinned, agent.character_id) {
            (Some(id), _) => self.personas.get_persona(id, owner).await?,
            (None, Some(character)) => {
                let active = self.state.get_active_persona(owner).await?;
                self.personas
                    .get_persona_for_character(owner, character, active.id)
                    .await?
            }
            (None, None) => self.state.get_active_persona(owner).await?,
        };

        let spec = AgentSpec {
            owner,
            character: agent.character_id.zip(agent.char_version_id),
            template_id: match agent.template_id {
                Some(id) => id,
                None => self
                    .state
                    .get_active_template_id(owner)
                    .await?
                    .ok_or(ChatError::State(
                        crate::errors::StateError::NoActiveTemplate,
                    ))?,
            },
            persona_id: Some(persona.id),
            tokenizer_id: self.state.get_active_tokenizer_id(owner).await?.ok_or(
                ChatError::State(crate::errors::StateError::NoActiveTokenizer),
            )?,
            profile_id: match agent.profile_id {
                Some(id) => id,
                None => self.state.get_active_profile(owner).await?.id,
            },
            lorebook_ids: (!agent.lorebooks.is_empty()).then(|| agent.lorebooks.clone()),
        };
        Ok((spec, persona))
    }

    pub(super) async fn bound_tool(
        &self,
        tool_id: Snowflake,
    ) -> Result<Option<UserTool>, ChatError> {
        Ok(match builtin_tool(tool_id) {
            Some(tool) => Some(tool),
            None => self.storage.get_tool(tool_id).await?,
        })
    }

    pub(super) async fn tool_bindings(
        &self,
        agent: &RealmAgent,
    ) -> Result<Vec<AgentToolBinding>, ChatError> {
        let mut bindings = agent.tools.clone();
        for &widget_id in &agent.widgets {
            let Some(widget) = self.storage.get_widget(widget_id).await? else {
                continue;
            };
            for tool_id in widget.tools {
                if !bindings.iter().any(|b| b.tool_id == tool_id) {
                    bindings.push(AgentToolBinding {
                        tool_id,
                        event_channel: None,
                    });
                }
            }
        }
        Ok(bindings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(id: i64, role: ChatRole, metadata: serde_json::Value) -> ConversationMessage {
        ConversationMessage {
            id: Snowflake(id),
            conversation_id: Snowflake(100),
            parent: None,
            role,
            content: String::new(),
            order: id as i32,
            metadata,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn only_the_newest_player_message_shows_its_images() {
        let replied = [
            message(1, ChatRole::User, json!({ "images": ["sha256:old"] })),
            message(
                2,
                ChatRole::Assistant,
                json!({ "images": ["sha256:drawn"] }),
            ),
        ];
        let moved_on = [
            message(1, ChatRole::User, json!({ "images": ["sha256:old"] })),
            message(2, ChatRole::Assistant, json!({})),
            message(3, ChatRole::User, json!({})),
        ];

        assert_eq!(newest_player_images(&replied), ["sha256:old"]);
        assert!(newest_player_images(&moved_on).is_empty());
    }
}
