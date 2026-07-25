# Realms Revision — Design Decisions & Implementation Plan

> **Status 2026-07-22: features A–F are all IMPLEMENTED.** Each section below opens
> with a `> **DONE**` block recording what actually shipped and where it diverged
> from the plan — the prose after those blocks is the ORIGINAL 2026-07-21 plan,
> kept for its rationale, and is no longer an accurate description of the code.
> Where the two disagree, the DONE block wins (feature E diverged most: widgets
> became sandboxed HTML iframes, not server-rendered weaver templates).
>
> Verified after implementation: `cargo test` green (144 total / 113 server tests),
> svelte-check / eslint / vite build clean except the pre-existing
> `stores/tokenizer.svelte.ts:44` error. Line numbers cited below are from
> 2026-07-21 and have drifted.
>
> **Post-landing addition 2026-07-23: static widgets carry a `prompt` + `tools`
> (see the E DONE block).** A widget is now a self-contained feature — display +
> a model directive + the commands to maintain what it shows — not just a panel.
>
> Original preamble: planning document, written 2026-07-21 after a design session.
> Paths are relative to the repo root (`lorealm/`).
>
> **Purpose:** let a fresh session pick this work up without re-deriving the design.
> Read the whole file before touching anything; the features interlock.

## 0. The goal driving all of this

The end goal for Realms is a strategy game where LLM agents lead nations: per-agent
command suites, per-turn stamina, spies, diplomacy, a thin rules framework with the
LLM driving the world forward. The owner wants two modes of play from the same realm:
a serious managed campaign, and off-script playthroughs ("walk out of the kingdom").

A design review found the engine primitives (grants, per-agent observations, script
agents, weaver-bodied tools, branch-and-replay state) largely sufficient, but four
blockers: no scenario bootstrap, per-agent (not per-utterance) speech targeting,
tools imprisoned by caller grants, and no presentation layer for state.

## 1. Settled design decisions (from the 2026-07-21 session — do not re-litigate)

1. **Tools become global-by-default.** The original design (tool bodies see only the
   calling agent's grant slice, writes gated by ReadWrite grants) was a
   *misunderstanding*. Intended model: tool bodies read/write the whole pool; the
   AUTHOR writes weaver logic to gate access. Grants narrow to **perception**: what
   enters an agent's prompt slice, which events fan out to it, what the inspector
   shows. Binding a tool to an agent becomes the capability boundary.
2. **Agent identity + seedable agent variables.** Realm agents are neither `char` nor
   `user` (that pair is a dyadic-chat concept). Each agent gets a stable authored
   **handle**; authored key-value properties (e.g. `talkativeness`) seed into the
   state pool at instantiation under `agents:<handle>.<prop>`. Runtime projections:
   `self:*` (own subtree, at assembly time) and `actor:*` (caller identity+props,
   inside tool bodies). This is a deliberate, bounded retreat from the pure
   emergent-seed stance: *preset-shaped per-agent data may seed; a realm still never
   stores a world-state blob.*
3. **Scenarios become first-class realm items**, analogous to character greetings: a
   named opening picked at instantiation = seed script (weaver, committed as a
   turn-zero state delta) + opening narration (finally emits `EventKind::System`) +
   optional per-scenario overrides of agent variables.
4. **Widget rework**, three separable parts:
   a. (bug) PUA marker leakage into prompts — models see `\u{E000}widget:N\u{E001}`
      in history and imitate it. Sanitize at prompt-assembly time.
   b. Static widgets: always-rendered, author-defined, read weaver state (minimap,
      inventory, currency HUD). Never attached by an LLM. These are the player-side
      dual of an LLM's state context block.
   c. Interactive widgets ("widgets that return content", select menus → minigames):
      NOT a new input channel — model them as custom renderers over the existing
      grant-checked action pipeline (an `ActionSpec` renderer whose result submits
      as turn args).
5. **Builtin tools stop being opaque Rust**: marked `builtin`, bodies in weaver,
   powered by new stdlib commands/processors (e.g. an event-emitting command so
   `speak_to(channel, message)` can be a builtin tool — this also solves
   per-utterance speech targeting with no new engine surface).
6. **Agreed build order:** marker fix → agent identity/variables → global tools (+
   stdlib/builtins) → scenarios → static widgets → interactive widgets. Rationale:
   global tools are useless for gating until tool bodies know who is calling
   (`actor:*`), which is why identity/variables comes first.

Standing constraints that survive from earlier decisions (see auto-memory
`eldoria-realms-roadmap`): realm = PRESET, state binds to exactly one conversation;
Lorealm conforms to the weaver_lang standard, never the reverse (weaver_lang is the
owner's separate lane — propose stdlib additions in Lorealm's registry first, only
push into weaver_lang itself when general); strict import-first membrane; the realm
definition is versioned JSONB, so new serde-defaulted fields need **no migration**.

## 2. Current architecture map (verified 2026-07-21)

### Realm data model — `backend/storage/src/models/realms.rs`
- `RealmDefinition { name, description, imports: Vec<ImportRule>, agents: Vec<RealmAgent> }`
  — a versioned JSONB document (head row + `realm_versions`, editing sessions with
  600s TTL collapse rapid edits; see `RealmService::get_realm_for_edit`).
- `RealmAgent { id, name, kind: Llm|Player|Script, character_id/char_version_id,
  template_id, profile_id, user_id, persona_id, turn_order, grants, tools,
  speech_scope, lorebooks, script, run_condition }`. No handle, no properties map —
  that's feature B.
- `AgentGrant { scope: String, access: Read|ReadWrite }`; grants are weaver
  `scope[:name]` patterns (`ScopePattern::{parse,validate,matches_var,matches_channel}`,
  ~line 335). Scope half = bare identifier; name half = `*`, dotted path, or
  trailing `.*`. Channels (speech/event scopes) match on the scope half only.
- `RealmCast` — pinned into `conversation_casts` ONCE at instantiation
  (`put_cast` has a single call site, `start_conversation`); never mutated mid-game.
- `RealmEvent { kind: Spoke|Acted|StateChanged|System, scope, content, payload }` —
  **`System` is currently never emitted anywhere.**
- `AgentObservation` — one agent's private feed row; `ResolvedRealmConfig` holds the
  format strings (`speech_format`, `action_format`, `tool_result_format`, state
  change formats) with per-realm overrides.

### Turn loop — `backend/server/src/services/realm/`
- `service.rs` (~1490 lines): `start_conversation` (:475) creates convo + pins cast,
  **stores no state** ("emergent seed" comment); `run_turn_stream` (:520) plans and
  streams; `plan_turn` (:862) reconstructs world from the active path and picks the
  agent via `runnable_agent` (round-robin + run_condition gating; `turn_start`
  resumes after the last message's `agent_id` metadata); `submit_player_turn`
  (:621) runs player actions through the SAME weaver pipeline as LLM tool calls;
  `llm_production` (:926) turns an `AgentOutcome` into events (Spoke from spoken
  text, Acted per successful tool call w/ binding's `event_scope`, StateChanged per
  touched namespace) + private `actor_notes` (tool results, actor-only);
  `commit` (:1006) persists message → state → events → observations;
  `fan_out` (:1181) delivers each event only to agents whose grant scope-half
  matches the event's channel (own events always observed; own Spoke = Assistant
  role, everything else = User role).
- `agent.rs`: `Agent` trait with `PlayerAgent` (waits), `ScriptAgent` (runs inline
  `agent.script` via `weaver::run_scoped` against its granted slice), `LlmAgent`
  (slices baseline per grants, loads last `OBSERVATION_WINDOW`=200 observations as
  the prompt thread, builds `ToolPolicy { write_scopes, tool_ids }`).
  `MAIN_SCOPE = "main"` is the default broadcast channel (agents need a `main`
  grant to hear open speech — known UX trap).
- `weaver.rs`: **the choke point for feature C.** `run_scoped` (:60) slices the
  world by the CALLER's read grants before evaluating, and gates writes via
  ReadWrite grants (`ToolContext::with_write_scopes`). `run_condition_passes`
  (:114): read-only, errors close to false. `slice` (:42) filters per-variable.
- `validate.rs`: per-kind agent validation (`validate_agent`), `ensure_imported`
  membrane checks, `validate_grants`, `validate_channel` (bare identifier),
  run_condition compile check at edit time.
- `types.rs`: `TurnPolicy` trait, but `RoundRobin` is hardwired in
  `RealmService::new` (service.rs:104). `RealmStreamEvent` SSE enum
  (AwaitingPlayer/TurnStarted/Delta/Reasoning/Widget/Action/Committed).

### Tool execution — `backend/server/src/services/tools/`
- `types.rs`: `UserTool { name, display_name, description, params: Vec<Param>, body
  (weaver source), enabled }` — **no `builtin` flag** (feature C). `CompiledUserTool`
  compiles the body as a `CompiledTemplate` and evaluates against `ToolContext`;
  exposed to the LLM as a native tool def (JSON schema from `Param`s).
  `ToolPolicy { write_scopes: Option<..>, tool_ids: Option<..> }` — `None` means
  unrestricted (plain chat), `Some` = realm constraints.
- `context.rs`: `ToolContext` implements weaver's `EvalContext`. Reads: `cmd:` =
  declared params, `local:` = scratch, anything else falls through to the (already
  sliced, for realm turns) state pool. Writes: `local` free; **`user`/`char` writes
  are rejected as reserved** (:66); other scopes checked against `write_scopes`.
  `fire_trigger`/`resolve_document` are TODO stubs (lorebook tie-in later).
  **Tool bodies have NO caller identity today** — nothing tells a tool which agent
  invoked it. `build_tool_registry()` (:102) = `stdlib::register_all`.
- Inference loop: `chat/service.rs::infer` (:728) builds the registry per turn:
  widget tools (`search_widgets`, `start_widget`) + user tools (all enabled for
  plain chat; exactly `policy.tool_ids` for realm agents). `MAX_TOOL_ROUNDS = 8`
  hardcoded (:881).

### State — `backend/server/src/services/state/service.rs`
- Pool = `HashMap<namespace, HashMap<var_name, serde_json::Value>>`. Rides
  `message_states` rows as `StateDelta { writes, deletes }` per message;
  `reconstruct(chain)` (:157) replays root→leaf (skips message ids without a row);
  `diff` (:198); `persist` (:232). Activation (lorebook cooldown/sticky state) is a
  full snapshot carried on the same row — turn-zero seeding must not clobber it
  (`capture` in realm/agent.rs deliberately carries it forward).
- Emergent seed mechanism: `OpenAgentTurn::delta` (chat/types.rs:152) diffs against
  the PRE-assembly baseline, so lorebook weaver writes during assembly persist.
  That is currently the only way an opening world materializes.

### Weaver hosting — three hosts, one variable snapshot (assembly only)
- `services/variables/`: `VariableProvider` registry, `materialize` snapshots once
  per turn. Builtin providers: `char:name`, `user:name`, `realm:agents` (array of
  cast names). **Materialized only inside `assemble_context`**
  (chat/service.rs:442) — lorebook host + block-text `substitute`. Tool bodies and
  scripts never see these variables (their `user`/`char` reads fall through to the
  state pool and find nothing).
- Lorealm's weaver stdlib: `services/context/weaver/stdlib.rs` — commands `set_var`,
  `get_var`, `var_exists`, `inc_var`, `push_var`, `default_var` (set-if-absent —
  the idempotent-seeding primitive) + `text.*` processors. Registered into both the
  lorebook host and `ToolContext` evaluation.
- weaver_lang 0.4.0 (pinned in `backend/server/Cargo.toml`; context_weaver 0.4.0).
  Strict mode is the default: reading a missing scoped variable is
  `UndefinedVariable` (weaver_lang `src/eval/mod.rs`, `resolve_variable`) — which is
  why `run_condition` cannot express "world not yet initialized" (errors close to
  false). Editor intellisense: `GET /templates/weaver-vocabulary`
  (routes/templates/handlers.rs:111) serves command/processor signatures — new
  stdlib entries appear in the editor automatically.

### Widgets — `backend/server/src/services/chat/widgets.rs`
- Hardcoded `WIDGET_CATALOG` (sms bubbles, notes, signs) + `search_widgets` /
  `start_widget` tools. `start_widget` pushes a `WidgetInstance { widget_id, props }`
  into a per-turn sink; `TurnAccumulator::widget` (chat/types.rs:107) splices a PUA
  marker `\u{E000}widget:N\u{E001}` into the spoken text; widgets persist in message
  `metadata.widgets`, markers in message content.
- **Leak paths (the bug):** (1) plain chat — `chat_history` resolver renders stored
  message content verbatim into prompts, markers included; (2) realms —
  `outcome.spoken` (markers included) becomes the Spoke event's content →
  `fan_out` copies it into every hearing agent's observations →
  `PromptTurn::from_observation` (context/resolve/types.rs:111) feeds it back into
  prompts. Models have been observed imitating the markers. The only existing
  filter (`WidgetInstance::filtered`, applied chat/service.rs:624) is a
  schedule-transform for props, unrelated.
- Frontend rendering: `packages/ui/src/types/widgets.ts` (`splitWidgetMarkers`,
  `parseWidgets`), `MessageBubble.svelte` splices `<ChatWidget>` at marker
  positions; stream `widget` events accumulate in `chat.svelte.ts`.

### Frontend surfaces (Svelte 5 monorepo, `frontend/`)
- Realm editor: `packages/ui/src/widgets/realms/` — `RealmsPage` shell + 7-tab
  `RealmEditor` (Overview/Imports/Cast/AccessMap/Graph/Formatting/History) +
  right-side `RealmInspector` + `PickerDialog`. `realmEditor.svelte.ts` holds
  ephemeral view state; `AgentCard` accordions live-edit agents via
  `editAgent(id, mutate)` (optimistic + 500ms debounced lossless full-object PUT).
  New agent fields (handle, properties) belong in `AgentCard` + store types
  (`types/realm.ts`) + `CreateAgentRequest` mirror in `api/client.ts`.
- Play surface: Write tab. `RealmChatControls.svelte` (Step/Run/Pause/Stop +
  Player/Director view toggle), `PlayerTurnBar.svelte` (auto-generated action forms
  from `ActionSpec.params` — the human half of the tool/form symmetry).
  `chatStore.visiblePath` filters the transcript to the viewer's granted hearing.
- Routes: preset authoring under `/realms/*` (routes/realm/mod.rs); playthroughs
  under `/chat/*` (routes/chat/mod.rs): `POST /chat/realm`, `POST
  /chat/{id}/turn[/stream]`, GET `cast`/`events`/`agents/{id}/observations`/
  `agents/{id}/state`/`agents/{id}/actions`, `GET messages/{id}/state`.

## 3. Feature specs, in build order

### A. Widget marker sanitization (bug fix — do first, independent of everything)

> **DONE 2026-07-22.** `sanitize_widget_markers` in `chat/widgets.rs`; applied in
> `PromptTurn::from_message` (resolves `[widget: <id>]` via message
> `metadata.widgets`) and `PromptTurn::from_observation` (no metadata → generic
> `[widget]`) — the only two prompt-bound funnels. Stored rows/events stay raw.
> Non-widget PUA spans are stripped. Tests in both modules.

**Goal:** no model ever sees a PUA marker.

- Add a sanitizer (suggested home: `chat/widgets.rs`, next to `widget_marker`) that
  replaces `\u{E000}widget:N\u{E001}` with a plain-text stand-in derived from
  `metadata.widgets[N]` (e.g. `[widget: sms-outgoing]`) or strips it.
- Apply at every prompt-bound rendering: the `chat_history` resolver
  (context/resolve/builtin/chat_history.rs), and realm event/observation creation —
  decide whether to sanitize `outcome.spoken` before it becomes the Spoke event
  (`llm_production`, realm/service.rs:955) or at `fan_out`. Sanitizing the EVENT
  content is simpler but destroys marker positions for any future
  observer-side widget rendering; sanitizing at observation-creation keeps the
  event as source of truth. Recommended: keep raw content on the message row
  (client needs the markers), sanitize events AND history at prompt time.
- Regression test: a stored message containing a literal marker must reach the
  provider request with the marker gone (both plain chat and a realm turn).

### B. Agent handles + seedable agent variables + `self:`/`actor:` projections

> **DONE 2026-07-22.** `RealmAgent.handle` + `.properties` (serde-defaulted;
> `AGENTS_SCOPE` const + `RealmAgent::own_scope()` in storage models).
> Validation: handle = identifier, cast-unique (`validate_handle`), property
> keys = dotted identifier paths; legacy agents keep empty handles at runtime
> (no seeding/projections) but re-saving requires one. `Actor` on `ToolContext`
> (`with_actor`) resolves `self:`/`actor:` dynamically incl. write map-back;
> `ToolPolicy.actor` plumbs it into LLM tool calls. Implicit own-subtree
> read+write scope via `read_scopes_of`/`write_scopes_of` (slicing only — fan_out
> still matches explicit grants, so no implicit `agents` event subscription).
> `inject_self_alias` for the assembly baseline. Seed commit: `seed_commit()` in
> start_conversation — System-role root message, `metadata.realm_seed`,
> turn_start treats it silently. Frontend: handle input + properties editor in
> AgentCard, auto handle in CastTab, types + agentRequest mirror.
> Caveat: assembly-time lorebook WRITES to `self:` are not mapped back (only
> ToolContext maps back); reads work everywhere.

**Goal:** agents have stable weaver-addressable identity and authored properties
(the `talkativeness` use case); tool bodies know who called them.

- `RealmAgent` gains `handle: String` (weaver identifier; validate with
  `is_identifier`, enforce cast-wide uniqueness in `validate_agent`) and
  `properties: HashMap<String, serde_json::Value>` (flat KV; dotted keys allowed
  since pool var names are dotted paths). Serde-defaulted → no migration; old casts
  deserialize with empty values. Mirror in `CreateAgentRequest`
  (routes/realm/dto.rs), frontend `types/realm.ts`, `AgentCard` (add a properties
  editor — key + typed value rows), `api/client.ts`.
- Seeding: at `start_conversation`, materialize every agent's properties into a
  turn-zero commit as `agents:<handle>.<prop> = value` writes (one root message,
  probably role System with empty content, carrying the `MessageState`). NOTE:
  today the first real turn becomes the root message (`PersistPlan::Append(None)`);
  a seed commit changes that — `turn_start` handles it fine (unattributable message
  restarts rotation, logs a warn; consider stamping meta so it doesn't warn).
  This machinery is exactly what scenarios (D) need — build it as a shared helper.
- Projections:
  - `self:*` — when planning an LLM/script agent's turn, alias its own
    `agents:<handle>.*` subtree into scope `self` in the slice handed to
    assembly/scripts/run_conditions. Cheap: build in `weaver::slice` callers.
    Writes to `self:` should map back (or be rejected — decide; rejecting is
    simpler and authors can write `agents:<own-handle>.x`... except they'd need
    the handle literal; mapping back is friendlier. OPEN).
  - `actor:*` — inside `run_scoped`, mount `handle`, `name`, `kind` + the caller's
    properties under scope `actor` (read-only). This unlocks author-side gating in
    global tools ("spend `agents:{{actor:handle}}.stamina`" — note weaver has no
    dynamic scope interpolation in variable refs; gating logic will lean on
    commands like `get_var(scope:..., name:...)` taking computed names — verify
    stdlib `get_var` signature supports this, it takes name as a prop).
  - Grants: decide whether every agent implicitly reads `agents:<own-handle>.*`
    (recommended: yes, plus `self:` alias) while other agents' subtrees need
    explicit grants (`agents:rival.*` = you can see rival's public sheet).
- `char`/`user` in realm turns: leave the assembly-time variable providers as-is
  (they already resolve per-agent from its character/persona). The reserved-write
  guard in `ToolContext` stays. Do NOT overload `char:`/`user:` as pool scopes.
- Talkativeness itself: no engine hook yet. V1 = authors use it in run_conditions
  once a `chance(p)`/random stdlib command exists (nondeterminism is safe — who ran
  is committed as messages, replay is unaffected). Eventual home: a
  talkativeness-weighted `TurnPolicy`. Out of scope for this pass.

### C. Global-by-default tools + builtin weaver tools + stdlib growth

> **DONE 2026-07-22.** `run_scoped` runs against the whole pool (scripts, LLM
> tools, player actions); prompt slice + run_conditions stay grant-filtered
> (conditions via new `ToolContext::read_only()`). Write-scope machinery
> deleted (`ToolPolicy.write_scopes`, `ToolContext` enforcement,
> `CompiledUserTool` param); reserved `user`/`char` guard + new `actor` guard
> stay. `emit_event(scope, content, kind?)` command (`tools/events.rs`,
> `EmitEventCommand` holds an `EventSink` — registered per-turn, NOT in the
> static stdlib) buffers Spoke/System drafts; drained via `run_scoped`'s third
> return + `ToolPolicy.event_sink` for the LLM path; `push_emitted_events`
> stamps them into all three turn drivers. Builtins: global catalog
> `tools/builtin.rs` (`builtin_tool(id)`, well-known low ids, materialized as
> `UserTool` rows, owner id 0, NO migration) — resolved before storage in
> validate/action_scopes/action_specs/player-turn/infer; import-exempt
> (filtered from `agent_required_imports`). First builtin: `speak_to(channel,
> message)` (id 1). `GET /tools/builtin` + frontend: builtins optgroup +
> badge in AgentCard bind picker, capability warning copy. `chance(p)` skipped —
> `random.gen`/`random.pick` already exist. Docs-site copy still owed.

**Goal:** tool bodies see the whole pool; authors gate; capability = binding.

- `weaver::run_scoped` (realm/weaver.rs:60): stop slicing (`before = world.clone()`),
  stop passing write scopes (`with_write_scopes(None)`). Do the same for the LLM
  path: `LlmAgent::plan` builds `ToolPolicy.write_scopes` — drop it (keep
  `tool_ids`!). The PROMPT slice (`baseline`) stays grant-filtered — grants remain
  perception. `run_condition_passes` stays sliced (a condition models the agent's
  own perception).
- Script agents: go global too (they're author-written world machinery; the
  "spymaster intermediary" pattern becomes trivial or unnecessary).
- Player actions (`submit_player_turn`) run the same `run_scoped` — global as well;
  the author's weaver gating is the equalizer.
- Event fan-out is unchanged and is what keeps this sane: a global tool writing
  enemy namespaces emits StateChanged on those namespaces, observed only by
  grant-holders. Known accepted quirk: StateChanged events are per-namespace, so
  partial grants (`nation:a.*`) observe whole-`nation` change events — this gets
  MORE visible now; consider splitting state events per grant-visible subset later.
- **Document loudly** (docs site + editor Inspector copy): binding a tool = granting
  its full capability; tool output enters the actor's observations permanently.
  Knowledge isolation now lives in tool authorship review.
- Builtin tools: add `builtin: bool` (or an `origin` enum) to `UserTool` — this one
  IS a SQL migration (`user_tools` is relational). Builtins ship with the server
  (seeded per user, or a global registry consulted at bind time — OPEN), are
  marked in the editor, bodies visible but read-only. First builtins:
  - `speak_to(channel, message)` — needs a new stdlib command that emits an event
    draft from inside a weaver body (e.g. `emit_event(kind, scope, content)`
    buffered on `ToolContext`, drained by the caller into `EventDraft`s — mirrors
    how `StartWidget` uses a sink). This closes the per-utterance speech-targeting
    gap: agents whisper on any channel they're bound to speak on.
  - `chance(p)` / dice — for talkativeness-style gating.
- `ToolContext` needs the event sink + `actor:*` (from B). Keep `user`/`char`
  writes reserved.
- Tests to update: `weaver.rs` tests asserting slice-gating of tools
  (`dotted_names_read_through_scoped_grants` re: run_scoped), `tools/context.rs`
  write-scope tests (write gating now applies only where a policy passes scopes —
  which after this change is nowhere in the realm path; decide whether the
  mechanism stays for future use or is removed).

### D. Scenarios as first-class realm items

> **DONE 2026-07-22.** `Scenario { id, name, description, seed_script, opening,
> agent_overrides }` on `RealmDefinition.scenarios` (serde-defaulted, no
> migration). `POST /chat/realm` takes `scenario_id` (None + one scenario =
> use it; None + several = 400 INVALID_SCENARIO; None + zero = legacy).
> `seed_commit`: property seeds + overrides → `weaver::run_seed` (actorless
> whole-pool eval) → root System message (content = opening narration,
> `metadata.realm_seed` + `scenario_id`) + state row + sourceless
> `EventKind::System` on `main` fanned out to observations — System finally
> has a producer. Validation `validate_scenario` (name, seed compile,
> override handles + keys) at dedicated routes `POST/PUT/DELETE
> /realms/{id}/scenarios[/{sid}]` (same get_realm_for_edit versioning as
> agents — resolves open question 5 by convention). Frontend: Scenarios tab
> (8th, explicit-save drafts w/ override rows), scenario picker next to Play,
> store/api/type mirrors. Polish note: the opening bubble renders with no
> agent_name (undefined); empty seed commits are hidden in player view.

**Goal:** a realm carries N named openings; instantiation picks one.

- `RealmDefinition.scenarios: Vec<Scenario>` — serde-defaulted, no migration.
  `Scenario { id: Snowflake, name, description, seed_script: String (weaver),
  opening: Option<String>, agent_overrides: HashMap<handle, HashMap<String, Value>> }`.
- `POST /chat/realm` gains `scenario_id: Option<Snowflake>` (None + exactly one
  scenario = use it; None + zero scenarios = legacy emergent start).
- `start_conversation`: after pinning the cast, run the seed script through the
  (now global) tool pipeline against an empty pool, merge agent property seeds (B)
  with per-scenario overrides applied, commit as the turn-zero root message;
  emit `EventKind::System` with the opening narration on channel `main` (fan_out
  delivers it to every `main`-granted agent — finally a producer for System).
- Validation: seed script must compile at edit time (same pattern as
  run_condition in `validate_agent`).
- Editor: new Scenarios tab in `RealmEditor` (8th tab) — list + weaver editor
  (CodeMirror weaver mode + vocabulary autocomplete already exist for tools) +
  opening text + per-agent override grid. Store: `realmEditor.svelte.ts` +
  `realm.svelte.ts` CRUD via the ordinary realm-document save path
  (`update_realm`-style endpoint or a dedicated `PUT /realms/{id}/scenarios`).
- This subsumes the earlier "genesis script-agent with `default_var`" workaround —
  don't build that.

### E. Static widgets (state-projection HUD)

> **DONE 2026-07-22 — but note the owner AMENDED the design below.** Widgets are
> NOT server-rendered weaver templates. A widget is a first-class user-owned
> resource (`user_widgets` table, migration `0015`; `UserWidget { name,
> description, source, enabled }`) whose `source` is a self-contained HTML
> document rendered client-side in a `sandbox="allow-scripts"` iframe. The host
> posts the viewer's state slice in — `{ type: 'lorealm:state', state }` — on
> every committed turn; no weaver, no server rendering, no props schema.
> Attachment (both surfaces, like lorebooks): the user's `enabled` set applies
> everywhere, and realm agents carry `RealmAgent.widgets` gated by the new
> `ImportKind::Widget` membrane (director view unions the whole cast's).
> **Placement (owner correction, same day): widgets render PER MESSAGE, not as a
> HUD bar.** `StaticWidgets.svelte` mounts inside `MessageBubble` (alongside the
> LLM-attached `ChatWidget` trailing block) and hands each message the state as
> of THAT message — so scrolling back shows the world as it was. Iframes are
> `loading="lazy"` since a long transcript mounts many. The widget list loads
> once in `ChatBox`, not per bubble. `WidgetHud.svelte` was deleted.
> **Bugfix 2026-07-23: each iframe lives in its own `StateFrame.svelte` child
> with a plain single-variable `bind:this`.** The first cut kept iframe refs in
> a `SvelteMap` populated via `bind:this={getter, setter}` — that two-function
> form is for two-way PROPERTY bindings (`bind:value`), NOT `bind:this`; it
> compiled but never populated the map, so no `postMessage` ever reached the
> iframe (widgets stuck on their placeholder even though state was written).
> **ROOT CAUSE of the "waiting for state…" bug: `postMessage` of a Svelte 5
> `$state` proxy throws `DataCloneError` and silently fails.** The state slice
> is `$state(...)` in `StaticWidgets`; passed to `StateFrame` and posted
> straight through, structured-clone chokes on the proxy so the iframe never
> receives anything (the editor preview worked only because it posts a plain
> const `SAMPLE`). Fix: `StateFrame.push()` posts `$state.snapshot(slice)`.
> `StateFrame` posts on `onload` (initial), on every slice change (updates via
> an `$effect`), and on an optional `{type:'lorealm:ready'}` ping. It relays
> `lorealm:action` from its own frame. NOTE: a prop named `state` collides with
> the `$state` rune — the slice prop is `slice`. Anytime reactive state is
> handed to `postMessage`/`structuredClone`/an iframe, snapshot it first.
> **Auto-sizing 2026-07-24:** iframes can't self-size, so `StateFrame` appends a
> `HOST_SCRIPT` to every widget's srcdoc that `ResizeObserver`s the body,
> reports content height (body box + margins, so it grows AND shrinks) as
> `{type:'lorealm:height'}`, and pings ready. `StateFrame` sets `style:height`
> from it (clamped 0–4000). Authors write plain HTML; the STARTER no longer
> hand-writes the ready ping. Known limitation: a widget using
> `html,body{height:100%}` collapses to 0 (measured height follows the iframe
> we set) — use auto height / padding instead.
> Backend: `UserWidgetStorage` (postgres/memory/cache) + `UserWidgetService` +
> `/api/v1/widgets` CRUD. `agent_state_slice` now applies the same projection
> the agent's prompt sees (own-subtree grant + `self` alias) and takes an
> optional `?at=<message_id>` so a realm viewer's per-message widget still only
> sees granted state (the plain `/messages/{id}/state` route is unfiltered and
> is used only for non-realm chats). Frontend: `userWidgetStore`, a Studio →
> Widgets section with live preview, widget checkboxes in AgentCard, widget
> import kind everywhere.
> Character-attached widgets (the other half of the owner's "both") are NOT
> built — only the user's active set and realm bindings exist today.
>
> **Widget prompt + tools, 2026-07-23 (owner addition — a widget is now a
> self-contained feature, not just presentation).** `UserWidget` gained
> `prompt: String` and `tools: Vec<Snowflake>` (migration **0016**, the second
> SQL migration in the widget line; postgres needs a `PgUserWidget` row struct
> now that `tools` is JSONB). `prompt` = a model directive on how to maintain
> the state the widget shows; `tools` = the commands the model needs (user
> tools or builtins, validated owned/builtin at widget save). **Delivery
> (owner decisions): prompt via a placed ContextSource; tools bundled with the
> widget.**
> - Prompt: new builtin ContextSource **`widgets`** (`resolve/builtin/widgets.rs`,
>   node `main`, emits each active widget's non-empty prompt as a System block).
>   The author must place the "Widget Directives" node in the template — silent
>   no-op otherwise (accepted footgun, the owner picked this over auto-inject).
>   Fed via new `ResolveContext.widget_prompts`, populated by `assemble_context`'s
>   new `widget_prompts` param. Plain chat: `ChatService::enabled_widget_directives`
>   (enabled set). Realm: `RealmService::widget_directives(agent)` → threaded on
>   `LlmTurn.widget_prompts` → `open_agent_turn`.
> - Tools: `ToolPolicy.extra_tool_ids` (registered in `infer` IN ADDITION to
>   `tool_ids`, deduped) carries plain-chat widget tools. Realm folds them in via
>   `RealmService::effective_bindings(agent)` = `agent.tools` + one
>   `AgentToolBinding{event_scope:None}` per widget-bundled tool (explicit
>   binding wins). effective_bindings now backs tool_ids, `action_scopes`
>   (llm_production), `action_specs` (both action-space endpoints), AND the
>   player-turn lookup — so interactive-widget tools work for players too.
>   Bundling BYPASSES the tool-import membrane (the owner's deliberate exception,
>   scoped to widget-declared tools); ownership is gated at widget save instead.
> - Widget-tool state writes ride the shared turn pool like any tool → their
>   deltas fan out as StateChanged and the widget re-renders. Widget tools with
>   no declared event_scope emit private (actor-only) Acted events.
> - Frontend: `UserWidget`/create/update types + WidgetsEditor gained a prompt
>   textarea and a tools picker (user tools + builtins).

**Goal:** author-defined always-on panels (minimap, inventory, currency) bound to
reconstructed state at the active leaf; never LLM-attached.

- These do not attach to messages. Suggested model: `RealmDefinition.panels` (or
  per-agent, so the player's HUD differs from the director view): `{ id, title,
  template: String (weaver), placement }` rendered client-side from data the
  existing endpoints already serve (`GET /chat/{id}/agents/{agent_id}/state` =
  grant-filtered slice; `GET /chat/{id}/messages/{id}/state` for historical views).
  Rendering options: server renders the weaver template to structured props per
  committed turn (consistent with all other weaver evaluation being server-side —
  recommended), or the client gets raw slice + a TS weaver evaluator (does not
  exist — avoid).
- The same authored projection should eventually feed the LLM side as a pinned
  "your current state" context block (a new builtin ContextSource alongside
  character/chat_history/world_info in `context/resolve/builtin/`) — one authored
  template, two renderings (prompt text vs widget props). Design them together,
  even if the context-source half ships later.
- Frontend: a HUD strip/sidebar on the Write tab for realm conversations
  (`ChatBox.svelte` area), re-rendered on `Committed` SSE events.

### F. Interactive widgets (returning content)

> **DONE 2026-07-22 (realm half), and much smaller than planned** — because E
> landed as iframes, no per-tool renderer registry or props machinery was
> needed. A widget acts by posting `{ type: 'lorealm:action', tool_id, args }`
> back to the host; `StaticWidgets` verifies the message came from one of its
> own frames and submits it through `submitRealmPlayerTurn`, i.e. the existing
> grant-checked player-turn pipeline — so the server still rejects any tool the
> player's agent isn't bound to. The plain-chat half (LLM-attached interactive
> widgets producing user-role messages) remains deferred, as planned.

**Goal:** widgets as input surfaces (select → minigame) without a second pipeline.

- Frame: a widget interaction PRODUCES a player action (`PlayerAction { tool_id,
  args }`) submitted through the existing `POST /chat/{id}/turn` — i.e. a custom
  renderer replacing `PlayerTurnBar`'s auto-form for a given `ActionSpec`. Tool
  params define the result schema; the widget is presentation.
- Requires: a way to associate a widget definition with a tool (per-tool renderer
  id + props template — plausibly another use of the same weaver-template-to-props
  machinery as E), and client plumbing to mount custom renderers.
- For plain-chat (non-realm) LLM-attached interactive widgets, the result would
  have to enter the conversation as a user-role message — defer; realms are the
  driver.
- Build LAST; depends on C (tool model settled) and E (props machinery).

## 4. Known issues to keep in mind while implementing

- Cast is immutable post-instantiation — mid-game grant/channel changes and
  join/leave are still out of scope after this revision (deferred deliberately).
- `RoundRobin` hardwired; `TurnPolicy` trait exists for later (reactive slots,
  talkativeness weighting).
- `OBSERVATION_WINDOW = 200` raw observations per prompt; no compaction. Fine for
  short playthroughs; becomes load-bearing for real campaigns ("context minmax" on
  the deferred roadmap).
- Realm turns still derive the tokenizer from the user's actives (deliberate,
  scoped decision from 2026-07-16 — do not "fix" in passing).
- Tool/lorebook BODIES read live at turn time (not version-pinned in the cast) —
  known accepted gap.
- Branching a PLAYER message is unsupported (no alternate-player-turn endpoint).
- Cancelling a realm turn commits the partial message server-side.

## 5. Verification / dev workflow

- Backend: `cargo test` in `backend/` (unit tests inline per module — see
  realm/service.rs, realm/weaver.rs, storage models tests). Realm end-to-end has
  historically been verified via curl against the dev server with test user
  `realmtest` / `realmtest123` (dev DB).
- Frontend: `bun` workspace in `frontend/`; svelte-check + eslint + vite build were
  green after the editor rewrite (one pre-existing unrelated error at
  `stores/tokenizer.svelte.ts:44`).
- weaver_lang / context_weaver are separate repos (`../weaver_lang`,
  `../context_weaver`), published to crates.io (0.4.0 pinned). New stdlib
  commands/processors for Lorealm belong in
  `backend/server/src/services/context/weaver/stdlib.rs` unless they are truly
  general to the language.

## 6. Open questions (ask the owner before committing to an answer)

> Items 1, 2, 3, 6 RESOLVED by the owner 2026-07-22:
> 1. `self:` writes **map back** to `agents:<own-handle>.*`.
> 2. Builtin tools: **global registry** consulted at bind time; import-exempt;
>    no per-user rows.
> 3. Write-scope mechanism: **delete** (`ToolPolicy.write_scopes` + `ToolContext`
>    enforcement; the reserved `user`/`char` write guard stays).
> 6. Seed commit is **part of the message history** (an ordinary root message,
>    not specially hidden). Owner note: realms should eventually integrate into
>    a ContextTemplate node provider.

> Items 4 and 5 RESOLVED during implementation 2026-07-22:
> 4. Neither, in the end: widgets are a **user-owned importable resource**, bound
>    per-agent (`RealmAgent.widgets`) or via the user's enabled set, and rendered
>    **client-side in a sandboxed iframe** — the server renders nothing.
> 5. **Dedicated routes** (`POST/PUT/DELETE /realms/{id}/scenarios[/{sid}]`) that
>    go through `get_realm_for_edit`, so editing-session versioning still applies.
