use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio_util::sync::CancellationToken;
use weaver_lang::{Registry, Value as WeaverValue};

use crate::services::{
    chat::{AttachmentSpec, WidgetInstance, WidgetSink, register_attachment_commands},
    commands::{
        CommandEffect, EffectSink, GenerateBridge, GenerateCommand, GenerateRawCommand,
        register_effect_commands,
    },
    context::weaver::stdlib,
    state::{StatePool, diff},
    tools::{Actor, EmitEventCommand, EmittedEvent, EventSink, RealmView, ToolContext},
    weaver::{
        Caps, WeaverProfile,
        eval::{Evaluable, Limits, Outcome, WeaverError, cancellation_flag},
    },
};

/// Where a body's non-state output collects.
///
/// Always present. A host without the matching capability simply drains an
/// empty sink.
#[derive(Clone, Default)]
pub struct Sinks {
    pub effects: EffectSink,
    pub events: EventSink,
    pub widgets: WidgetSink,
}

/// Everything the sinks held, taken in one go.
///
/// A host that lacked the matching capability contributes an empty vector.
#[derive(Debug, Default)]
pub struct Drained {
    pub effects: Vec<CommandEffect>,
    pub events: Vec<EmittedEvent>,
    pub widgets: Vec<WidgetInstance>,
}

impl Sinks {
    /// Take everything buffered so far, leaving the sinks empty.
    pub fn drain(&self) -> Drained {
        Drained {
            effects: self.effects.lock().unwrap().drain(..).collect(),
            events: self.events.lock().unwrap().drain(..).collect(),
            widgets: self.widgets.lock().unwrap().drain(..).collect(),
        }
    }
}

/// A weaver registry with its sinks and the context a body will run against.
///
/// Build one with [`WeaverHost::builder`]. Then either evaluate something
/// one-shot with [`WeaverHost::evaluate`], or hand the registry out with
/// [`WeaverHost::into_shared`] for a caller that runs many independent
/// evaluations against it.
pub struct WeaverHost {
    registry: Arc<Registry>,
    sinks: Sinks,
    caps: Caps,
    pool: Arc<Mutex<StatePool>>,
    cmd: HashMap<String, WeaverValue>,
    actor: Option<Actor>,
    realm: Option<RealmView>,
    read_only: bool,
    limits: Limits,
    cancel: Option<CancellationToken>,
}

impl WeaverHost {
    pub fn builder() -> WeaverBuilder {
        WeaverBuilder::default()
    }

    #[cfg(test)] // Gated to mute clippy errors; Ungate if this ever becomes relevant.
    pub fn base() -> Self {
        Self::builder().build()
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// The bare registry, owned.
    ///
    /// For tests that register one more command by hand, and for callers that
    /// only need name resolution. Prefer a profile where one fits.
    #[cfg(test)] // Gated to mute clippy errors
    pub fn into_registry(self) -> Registry {
        // The only other handles are the short-lived clones `evaluate` hands
        // to its blocking thread, and this consumes the host.
        Arc::try_unwrap(self.registry).unwrap_or_else(|_| unreachable!("host owns its registry"))
    }

    /// The registry and sinks, for a caller that owns the evaluation itself.
    pub fn into_shared(self) -> (Arc<Registry>, Sinks, Arc<Mutex<StatePool>>) {
        (self.registry, self.sinks, self.pool)
    }

    /// Run one body to completion off the async runtime.
    ///
    /// Always evaluates on a blocking thread.
    pub async fn evaluate<E: Evaluable>(&self, body: E) -> Result<Outcome, WeaverError> {
        let baseline = self.pool.lock().unwrap().clone();
        let (stop, watcher) = cancellation_flag(self.cancel.clone());

        let mut ctx = self.context();
        let registry = Arc::clone(&self.registry);
        let limits = self.limits.clone();
        let evaluated =
            tokio::task::spawn_blocking(move || body.eval(&mut ctx, &registry, &limits, stop))
                .await;

        if let Some(watcher) = watcher {
            watcher.abort();
        }

        let value = match evaluated {
            Ok(result) => result?,
            Err(e) => return Err(WeaverError::DidNotFinish(e.to_string())),
        };
        Ok(self.finish(baseline, value))
    }

    /// Run one body on the current thread.
    ///
    /// For hosts small enough that a blocking hop is not worth it.
    pub fn evaluate_inline<E: Evaluable>(&self, body: E) -> Result<Outcome, WeaverError> {
        if self.caps.contains(Caps::GENERATION) {
            return Err(WeaverError::BlockingRequired);
        }
        let baseline = self.pool.lock().unwrap().clone();
        let mut ctx = self.context();
        let value = body.eval(&mut ctx, &self.registry, &self.limits, None)?;
        Ok(self.finish(baseline, value))
    }

    /// A fresh context over this host's pool, args and actor.
    fn context(&self) -> ToolContext {
        let ctx = ToolContext::new(self.cmd.clone(), Arc::clone(&self.pool))
            .with_actor(self.actor.clone())
            .with_realm(self.realm.clone());
        if self.read_only { ctx.read_only() } else { ctx }
    }

    /// Everything an evaluation leaves behind, once the body has run.
    fn finish(&self, baseline: StatePool, value: WeaverValue) -> Outcome {
        let pool = self.pool.lock().unwrap().clone();
        let delta = diff(&baseline, &pool);
        let Drained {
            effects,
            events,
            widgets,
        } = self.sinks.drain();

        Outcome {
            value,
            pool,
            delta,
            effects,
            events,
            widgets,
        }
    }
}

/// Configures one [`WeaverHost`].
///
/// Start from a [`WeaverProfile`] where one fits.
#[derive(Default)]
pub struct WeaverBuilder {
    caps: Caps,
    declarations_only: bool,
    events: Option<EventSink>,
    attachments: Vec<AttachmentSpec>,
    generation: Option<GenerateBridge>,
    pool: Option<Arc<Mutex<StatePool>>>,
    cmd: HashMap<String, WeaverValue>,
    actor: Option<Actor>,
    realm: Option<RealmView>,
    read_only: bool,
    limits: Limits,
    cancel: Option<CancellationToken>,
}

impl WeaverBuilder {
    /// Take capabilities and write access from a named host.
    pub fn profile(mut self, profile: WeaverProfile) -> Self {
        self.caps = profile.caps();
        self.read_only = profile.read_only();
        self
    }

    /// Register every capability, with no runtime machinery behind any of it.
    ///
    /// Commands resolve by name and carry their real signatures, but calling
    /// one is an error. This is an editor's view of the language: see
    /// [`editor_caps`](super::editor_caps).
    pub fn declarations_only(mut self) -> Self {
        self.caps = super::editor_caps();
        self.declarations_only = true;
        self.events.get_or_insert_with(Default::default);
        self
    }

    /// Collect emitted events into a sink the caller already holds.
    pub fn events(mut self, sink: EventSink) -> Self {
        self.events = Some(sink);
        self
    }

    /// The widgets `$[attach]` may choose from.
    pub fn attachments(mut self, catalog: Vec<AttachmentSpec>) -> Self {
        self.attachments = catalog;
        self
    }

    /// Wire `$[generate]` to a live chat handle.
    pub fn generation(mut self, bridge: GenerateBridge) -> Self {
        self.generation = Some(bridge);
        self
    }

    /// The state pool the body reads and writes. Defaults to an empty one.
    pub fn pool(mut self, pool: Arc<Mutex<StatePool>>) -> Self {
        self.pool = Some(pool);
        self
    }

    /// Bindings for the `cmd:` namespace. See
    /// [`bind_json_args`](super::bind_json_args).
    pub fn args(mut self, cmd: HashMap<String, WeaverValue>) -> Self {
        self.cmd = cmd;
        self
    }

    /// The realm agent the body runs on behalf of, mounting `self:` and
    /// `actor:`.
    pub fn actor(mut self, actor: Option<Actor>) -> Self {
        self.actor = actor;
        self
    }

    /// The realm the body runs inside, mounting `cast:` and `turn:`.
    pub fn realm(mut self, realm: RealmView) -> Self {
        self.realm = Some(realm);
        self
    }

    #[cfg(test)] // Gated to mute clippy errors
    pub fn limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// Stop evaluation when `token` fires.
    pub fn cancel(mut self, token: CancellationToken) -> Self {
        self.cancel = Some(token);
        self
    }

    pub fn build(self) -> WeaverHost {
        let mut registry = Registry::new();
        stdlib::register_all(&mut registry);

        let mut sinks = Sinks::default();

        if self.caps.contains(Caps::EFFECTS) {
            sinks.effects = register_effect_commands(&mut registry);
        }

        // Events need somewhere to go. A profile carrying the capability
        // describes what it *may* expose; whether `$[emit_event]` is actually
        // wired depends on the caller handing over a sink to read it back
        // from.
        if let (true, Some(sink)) = (self.caps.contains(Caps::EVENTS), self.events) {
            sinks.events = sink;
            registry.register_command(EmitEventCommand::new(Arc::clone(&sinks.events)));
        }

        if self.caps.contains(Caps::ATTACHMENTS) {
            sinks.widgets = register_attachment_commands(&mut registry, self.attachments);
        }

        if self.caps.contains(Caps::GENERATION) {
            match self.generation {
                Some(bridge) if !self.declarations_only => {
                    registry.register_command(GenerateRawCommand::new(bridge.clone()));
                    registry.register_command(GenerateCommand::new(bridge));
                }
                _ => {
                    registry.register_command(GenerateRawCommand::declaration());
                    registry.register_command(GenerateCommand::declaration());
                }
            }
        }

        WeaverHost {
            registry: Arc::new(registry),
            sinks,
            caps: self.caps,
            pool: self.pool.unwrap_or_default(),
            cmd: self.cmd,
            actor: self.actor,
            realm: self.realm,
            read_only: self.read_only,
            limits: self.limits,
            cancel: self.cancel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::weaver::{Limits, compile_expr, compile_template};

    fn names(host: &WeaverHost) -> Vec<String> {
        let mut names: Vec<String> = host
            .registry()
            .command_signatures()
            .into_iter()
            .map(|s| s.name)
            .collect();
        names.sort();
        names
    }

    #[test]
    fn the_editor_resolves_every_command_a_runtime_host_registers() {
        let editor = WeaverHost::builder().declarations_only().build();

        for profile in WeaverProfile::RUNTIME {
            let host = WeaverHost::builder()
                .profile(*profile)
                .events(Default::default())
                .build();

            for name in names(&host) {
                assert!(
                    editor.registry().command_signature(&name).is_some(),
                    "{profile:?} registers `{name}`, which the editor would underline as unknown"
                );
            }
        }
    }

    #[test]
    fn a_profile_registers_only_what_it_declares() {
        let seed = WeaverHost::builder()
            .profile(WeaverProfile::RealmSeed)
            .build();
        assert!(seed.registry().command_signature("toast").is_none());
        assert!(seed.registry().command_signature("emit_event").is_none());
        assert!(seed.registry().command_signature("set_var").is_some());
    }

    #[test]
    fn events_stay_unregistered_until_a_sink_can_read_them() {
        let without = WeaverHost::builder()
            .profile(WeaverProfile::ChatTurn)
            .build();
        assert!(without.registry().command_signature("emit_event").is_none());

        let with = WeaverHost::builder()
            .profile(WeaverProfile::ChatTurn)
            .events(Default::default())
            .build();
        assert!(with.registry().command_signature("emit_event").is_some());
    }

    #[tokio::test]
    async fn an_evaluation_reports_its_writes_as_a_delta() {
        let host = WeaverHost::builder()
            .profile(WeaverProfile::RealmSeed)
            .build();

        let outcome = host
            .evaluate(compile_template(r#"$[set_var("world:season", "winter")]"#).unwrap())
            .await
            .unwrap();

        assert!(outcome.wrote_state());
        assert_eq!(outcome.pool["world"]["season"], serde_json::json!("winter"));
        assert_eq!(
            outcome.delta.writes["world"]["season"],
            serde_json::json!("winter")
        );
    }

    #[tokio::test]
    async fn a_template_and_an_expression_both_come_back_as_one_value() {
        let host = WeaverHost::base();

        let rendered = host
            .evaluate(compile_template("two: @[math.add(a: 1, b: 1)]").unwrap())
            .await
            .unwrap();
        assert_eq!(rendered.to_output_string(), "two: 2");

        let condition = host.evaluate(compile_expr("1 < 2").unwrap()).await.unwrap();
        assert!(condition.is_truthy());
    }

    #[tokio::test]
    async fn effects_are_collected_for_a_host_that_can_read_them() {
        let host = WeaverHost::builder()
            .profile(WeaverProfile::Command)
            .build();

        let outcome = host
            .evaluate(compile_template(r#"$[sys("done")]"#).unwrap())
            .await
            .unwrap();

        assert_eq!(outcome.effects.len(), 1);
    }

    #[tokio::test]
    async fn a_runaway_loop_hits_the_ceiling_instead_of_hanging() {
        let host = WeaverHost::builder()
            .profile(WeaverProfile::RealmSeed)
            .limits(Limits {
                max_iterations: Some(10),
                ..Limits::default()
            })
            .build();

        let runaway = compile_template(
            "{# foreach n in @[array.range(start: 0, end: 1000)] #}{{n}}{# endforeach #}",
        )
        .unwrap();

        assert!(host.evaluate(runaway).await.is_err());
    }

    #[test]
    fn generation_cannot_be_evaluated_on_the_async_runtime() {
        // `$[generate]` blocks on a future, which panics on a worker thread.
        let host = WeaverHost::builder()
            .profile(WeaverProfile::Command)
            .build();

        let err = host
            .evaluate_inline(compile_template("nothing to run").unwrap())
            .unwrap_err();
        assert!(matches!(err, WeaverError::BlockingRequired));
    }
}

#[cfg(test)]
mod vocabulary {
    use super::*;

    /// Print the host tables in `docs/weaver-hosts.md`, the way
    /// `dump_error_table` prints the error contract:
    ///
    /// ```text
    /// cargo test -p lorealm_server --bin lorealm_server \\
    ///     dump_weaver_vocabulary -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "diagnostic, not an assertion"]
    fn dump_weaver_vocabulary() {
        // Commands a bare stdlib host already has, so the per-profile columns
        // list only what a capability adds.
        let stdlib: Vec<String> = command_names(&WeaverHost::base());

        println!("\n| Host | Capabilities | Commands beyond the stdlib |");
        println!("| --- | --- | --- |");
        for profile in WeaverProfile::RUNTIME {
            let host = WeaverHost::builder()
                .profile(*profile)
                .events(Default::default())
                .build();
            let added: Vec<String> = command_names(&host)
                .into_iter()
                .filter(|name| !stdlib.contains(name))
                .map(|name| format!("`{name}`"))
                .collect();

            let caps = format!("{:?}", profile.caps());
            let caps = caps
                .trim_start_matches("Caps(")
                .trim_end_matches(')')
                .to_string();
            // `|` is the bitflags separator and the markdown column divider.
            let caps = caps.replace(" | ", ", ");
            let caps = if caps == "0x0" {
                "none".into()
            } else {
                caps.split(", ")
                    .map(|c| format!("`{}`", c.to_lowercase()))
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            println!(
                "| `{profile:?}` | {caps} | {} |",
                if added.is_empty() {
                    "—".into()
                } else {
                    added.join(", ")
                }
            );
        }

        let editor = WeaverHost::builder().declarations_only().build();
        println!(
            "\nEditor vocabulary ({} commands):",
            command_names(&editor).len()
        );
        println!("{}", command_names(&editor).join(", "));

        let mut processors: Vec<String> = editor
            .registry()
            .processor_signatures()
            .into_iter()
            .map(|s| format!("{}.{}", s.namespace, s.name))
            .collect();
        processors.sort();
        println!("\nProcessors ({}):", processors.len());
        println!("{}", processors.join(", "));

        println!("\nStdlib commands: {}", stdlib.join(", "));
    }

    fn command_names(host: &WeaverHost) -> Vec<String> {
        let mut names: Vec<String> = host
            .registry()
            .command_signatures()
            .into_iter()
            .map(|s| s.name)
            .collect();
        names.sort();
        names
    }
}
