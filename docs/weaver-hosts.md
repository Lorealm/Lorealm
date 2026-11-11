# Weaver hosts

> AI Disclosure: This document was authored with the help of Claude.

The [Weaver Language](https://github.com/crux-03/weaver_lang) runs in several
very different places in Lorealm, and each one offers a different set of
commands. A `/slash` command can ask the model a question; a realm seed script
cannot. A tool the model calls mid-turn can attach a widget; an agent's run
condition cannot even write state.

A **host** is one of those places. Every host is built the same way, from
`backend/server/src/services/weaver/`:

```rust
let host = WeaverHost::builder()
    .profile(WeaverProfile::RealmScript)
    .events(sink)
    .actor(Some(actor_of(agent)))
    .pool(Arc::clone(&pool))
    .build();

let outcome = host.evaluate(compile_template(source)?).await?;
```

## Rules

- **A profile decides what exists.** Nothing else assembles a registry by hand.
  If a body can call `$[toast]` somewhere, it is because that host's profile
  carries `effects`.
- **The editor is the union, derived.** `editor_caps()` folds every entry in
  `WeaverProfile::RUNTIME` together. Completion and diagnostics are built from
  it, so neither can offer a command no host has, or underline one that every
  host accepts.
- **Capabilities are potential, not wiring.** A profile says what a host *may*
  expose. `$[emit_event]` still needs a sink to report into and `$[generate]`
  still needs a live chat handle. Without them the command is left
  unregistered, or registered as a declaration that errors when called — a body
  naming it fails loudly rather than emitting into a void.
- **Unavailable is an error, never a silent no-op.** A command that would
  discard its own output is worse than one that refuses. This is why
  `RealmScript` has no `effects`: realm has nowhere to deliver `$[sys]`, so
  granting it would swallow the call instead of reporting it.
- **Every host has ceilings.** `Limits` applies node and iteration caps by
  default. They are template-only: `weaver_lang` 0.6 has no options variant for
  expressions, which is safe because the expression grammar has no loop.
- **`generation` may not run on the async runtime.** `$[generate]` blocks on a
  future, which panics on a runtime worker. `WeaverHost::evaluate` always hops
  to a blocking thread; `evaluate_inline` refuses a host that carries the
  capability.

## Hosts

Generated from the source — regenerate with:

```
cargo test -p lorealm_server --bin lorealm_server dump_weaver_vocabulary -- --ignored --nocapture
```

| Host | Capabilities | Commands beyond the stdlib |
| --- | --- | --- |
| `ChatTurn` | `effects`, `events`, `attachments`, `generation` | `attach`, `compose`, `emit_event`, `generate`, `generate_raw`, `search_attachments`, `sys`, `toast` |
| `Command` | `effects`, `generation` | `compose`, `generate`, `generate_raw`, `sys`, `toast` |
| `RealmScript` | `events` | `emit_event` |
| `RealmCondition` | none | — |
| `RealmDirector` | none | — |
| `WidgetRender` | none | — |
| `RealmTemplate` | none | — |

Where each one is built:

| Host | Built at | Runs |
| --- | --- | --- |
| `ChatTurn` | `services/chat/service.rs` | A user tool body the model invokes mid-turn |
| `Command` | `services/commands/service.rs` | A `/slash` command body |
| `RealmScript` | `services/realm/weaver.rs` | A script agent's body, and realm actions |
| `RealmCondition` | `services/realm/weaver.rs` | An agent's `run_condition` |
| `RealmDirector` | `services/realm/weaver.rs` | A realm's `Director::Script`, rendering the handle of whoever acts next |
| `WidgetRender` | `services/chat/widgets.rs` | A widget's `render`, turning it into prompt text during assembly |
| `RealmTemplate` | `services/realm/wtn.rs` | A WTN document, expanded once into a realm definition |

The editor's two endpoints — `check` in `services/context/weaver/check.rs` and
`list_weaver_vocabulary` in `routes/templates/handlers.rs` — build
`WeaverHost::builder().declarations_only()`, which is every capability above
with no machinery behind any of it.

`check` also takes an optional `WeaverProfile`. Given one it builds
`declarations_for(profile)` instead — that host's vocabulary rather than the
union — so a body is checked against the host that will actually run it.
Without one it falls back to the union, which accepts commands the real host
will refuse. A caller that knows where the source runs should say so.

### Dialects

`check(source, dialect, profile)` picks the compiler from `dialect`:

| Dialect | Compiler | What it is |
| --- | --- | --- |
| `template` | `CompiledTemplate` | Text with weaver embedded in it |
| `expression` | `CompiledExpr` | A single value, the way a run condition is |
| `document` | `parse_value_doc` | A WTN document: one value with holes, plus its `#inputs` |

The dialect is not cosmetic. A WTN document fed to the `template` compiler
parses *as prose* — `{ name: "x" }` is literal text and `{# foreach #}` is a
valid template block — so it comes back clean with none of its real errors
reported. Silence, not diagnostics.

`document` additionally checks the `#inputs` block, which is the only place
the other two dialects never reach:

- a `Ref<Kind>` naming a kind the registry has no entry for
- an entity input carrying a default, which cannot work: entity inputs are
  resolved before the document runs, and a default has not been evaluated by
  then

`BlueprintService` saves through this same function rather than a validator of
its own, so the editor cannot show a document clean that saving then refuses.

### Always available

The stdlib is present in every host and is not a capability:

`default_var`, `del_var`, `get_var`, `inc_var`, `push_var`, `set_var`,
`var_exists`

Along with 36 processors: `array.*` (9), `math.*` (12), `object.*` (1),
`random.*` (2), `text.*` (12).

All but `del_var` come from `context_weaver::stdlib`, which the lorebook
engine loads too. `del_var` is Lorealm's own and is registered in
`WeaverBuilder::build` instead: `EvalContext` has no delete, so the command
reaches the pool by downcasting to `ToolContext`. Every host above evaluates
against one; the lorebook engine does not, which is why the command is not in
`register_all` alongside the rest. A read-only profile refuses it exactly as
it refuses `set_var`.

### Entity kinds

Registered alongside the stdlib, for the `Ref<...>` declarations a WTN
document's `#inputs` block may name:

`Character`, `Lorebook`, `Template`, `Tool`, `Persona`, `ConnectionProfile`,
`Widget`

The list is `RealmResourceKind::ALL`, so it is the same vocabulary a realm's
inferred resource set uses and cannot drift from it. A kind is a type rather
than something callable, so it is not a capability and is inert in the hosts
that never expand a document.

Values arrive as ids and are bound as small objects — `id`, `name`,
`description`, `slug` — resolved by `services/realm/project.rs` *before*
expansion, because `validate_input` is synchronous and every lookup behind it
is not. `slug` is the name reduced to an identifier, so a loop can mint
handles from it.

### Modules

A user's enabled modules are registered in every host, after their plugins,
through the same `PluginVocabulary`. A module named `combat` is reached two ways:

| | |
| --- | --- |
| `@[combat.damage(3)]` | An `export proc`. Its body runs inside the caller's evaluation, so it has exactly the caller's capabilities: `$[toast]` works from a `Command` and fails from a `RealmCondition`. A declared return type (`-> number`) is checked each time it returns. |
| `{{combat:hit_table}}` | An `export name = value`. Values are constants, evaluated against the stdlib under the default `Limits` when the vocabulary is built; a value that reads state, or runs past the limits, is refused on save. |

Each module's scope is read-only, in every host and in the lorebook engine. A
module may not be named after a reserved scope (`RESERVED_SCOPES`: `cmd`,
`local`, `self`, `cast`, `user`, `state` and the rest), a stdlib or plugin
namespace, or another of the user's modules; a module that collides with a
plugin installed later is skipped at registration, because a module proc
shadows a registered processor of the same name.

State namespaces are created at runtime by whatever writes to them, so no save
can rule out a collision with one. If a scope is both a module and a namespace
holding state, reading it is an error rather than a silent answer from the
module.

A realm's `modules` are loaded for its turns on top of the globally enabled
ones, whether or not they are enabled themselves.

The editor reads modules like everything else. `processor_signatures` lists
module procs with their doc comment, parameter types and return type, and the
vocabulary carries one namespace per module whose members are its values,
described by their doc comments.

### Inside a realm

The realm hosts also mount three read-only namespaces describing the realm
itself, so a body never has to restate what the engine already knows. They are
derived per evaluation from the pinned cast, the active path, and the pool as
it stands — deliberately not *kept* in the state pool, which is persisted as
per-message deltas and would freeze a stale copy into history.

| | |
| --- | --- |
| `cast:count`, `cast:handles`, `cast:names` | The whole roster, in turn order |
| `cast:members` | One object per agent, in turn order. What a `foreach` walks |
| `cast:llm.*`, `cast:player.*`, `cast:script.*` | The same four, per kind |
| `cast:agents.<handle>` | One cast member, in the same shape as a `members` element |
| `turn:index` | Turns already committed on this path |
| `turn:last.handle` / `.name` / `.kind` / `.said` | Who acted last; absent on the opening turn |
| `scenario:slug`, `scenario:name` | The opening this playthrough started from; absent when the realm has none |

A member object carries `handle`, `name`, `kind` and `turn_order` over
whatever that agent holds in `agents:<handle>` at the moment of evaluation, so
a director weighting its cast reads `{{c.talkativeness}}` off the loop binding
rather than rebuilding the pool key by hand. The engine's four fields are
written last: a property named `name` cannot displace the display name.

`count`/`handles`/`names` stay because they answer the common questions without
a loop. Note that only `members` is iterable — `{{cast:llm}}` is the group
object itself, and `foreach` refuses it.

Only agents with a handle appear: an unhandled agent has no `agents:` subtree
and no `self:`, so listing it would name an identity nothing can address.

`RealmTemplate` is the exception among the realm hosts: a document is expanded
before there is a playthrough, so it mounts none of these — there is no roster
or opening to describe yet. It is the document that *produces* them.

`scenario:` is how an opening sets itself up now that a scenario carries no
script of its own. A script agent gated on `{{turn:index}} == 0` branches on
`{{scenario:name}}` and writes whatever state that opening needs — the same
effect as a seed script, in a place the cast can see. The one thing it gives
up is ordering: a seed ran before anything else, while a script agent runs
when the director says so, so put it first and keep turn zero deterministic.

`cast:`, `turn:` and `scenario:` are reserved in every host, the way `actor:` is — a state
namespace cannot shadow them, and writing to one is an error rather than a
write that goes nowhere.

## Running a body

Both compilers produce a `Value` — a template's rendered text is that value
through `to_output_string()`, not a separate evaluation mode. `Outcome` carries
the value plus everything the body left behind, and each host takes the
projection it wants:

| Field | |
| --- | --- |
| `value` | What the body evaluated to |
| `pool` | The state pool as the body left it |
| `delta` | What it changed, against the pool it started from |
| `effects` | `$[sys]`, `$[toast]`, `$[compose]` output |
| `events` | `$[emit_event]` output |
| `widgets` | `$[attach]` output |

```rust
outcome.to_output_string()  // a realm script's spoken output
outcome.to_json()?          // a command's or tool's return value
outcome.is_truthy()         // a run condition
```

Sink fields are empty rather than absent when the host lacked the capability,
so no caller branches on whether a group was wired.

### Two terminals

`evaluate()` runs one body and hands back an `Outcome`. It is what every
one-shot host uses.

`into_shared()` hands back the registry and sinks instead, for a caller that
owns its own evaluations. Chat takes this: its registry is shared by every tool
the model may call across as many rounds as the loop runs, and its sinks are
drained per call while the turn is still streaming. Lifetime is the call site's
business, not the host's.

## Adding a capability

1. Add the flag to `Caps` in `services/weaver/profile.rs`, documenting what it
   needs at runtime.
2. Add it to the profiles that carry it in `WeaverProfile::caps`.
3. Register its commands in `WeaverBuilder::build`, behind the capability check.
   Leave the command *implementations* in the module that owns them —
   `$[attach]` with widgets, `$[emit_event]` with tools, `$[sys]` with commands.
   This module only decides which of them exist where.
4. If the group needs runtime plumbing a caller must supply, make the
   registration conditional on receiving it, and give `declarations_only` a
   throwaway so the editor still resolves the name.
5. Regenerate the tables above.

## Adding a host

1. Add the variant to `WeaverProfile` **and** to `WeaverProfile::RUNTIME`. The
   editor's vocabulary is derived from that list; a host missing from it will
   have its commands underlined as unknown.
2. Give it an arm in `caps`, and in `read_only` if it may not write state.
3. Build it at the call site with `WeaverHost::builder().profile(..)`.
4. Regenerate the tables above.

`cargo test -p lorealm_server --bin lorealm_server weaver` enforces that the
editor resolves every command some runtime host registers, that a profile
registers only what it declares, that limits are applied, and that a host
carrying `generation` refuses to run inline.

## Known gap

The frontend still sends no profile, so everything it checks is validated
against the union. The backend takes one now — `weaverCheck` needs to pass
which context is being edited.
