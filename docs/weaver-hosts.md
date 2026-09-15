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
| `RealmSeed` | none | — |
| `RealmCondition` | none | — |
| `RealmDirector` | none | — |
| `WidgetRender` | none | — |

Where each one is built:

| Host | Built at | Runs |
| --- | --- | --- |
| `ChatTurn` | `services/chat/service.rs` | A user tool body the model invokes mid-turn |
| `Command` | `services/commands/service.rs` | A `/slash` command body |
| `RealmScript` | `services/realm/weaver.rs` | A script agent's body, and realm actions |
| `RealmSeed` | `services/realm/weaver.rs` | A scenario's seed script, once at instantiation |
| `RealmCondition` | `services/realm/weaver.rs` | An agent's `run_condition` |
| `RealmDirector` | `services/realm/weaver.rs` | A realm's `Director::Script`, rendering the handle of whoever acts next |
| `WidgetRender` | `services/chat/widgets.rs` | A widget's `render`, turning it into prompt text during assembly |

The editor's two endpoints — `check` in `services/context/weaver/check.rs` and
`list_weaver_vocabulary` in `routes/templates/handlers.rs` — both build
`WeaverHost::builder().declarations_only()`, which is every capability above
with no machinery behind any of it.

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

### Inside a realm

The four realm hosts also mount two read-only namespaces describing the realm
itself, so a body never has to restate what the engine already knows. They are
derived per evaluation from the pinned cast and the active path — deliberately
not kept in the state pool, which is persisted as per-message deltas and would
freeze a stale copy into history.

| | |
| --- | --- |
| `cast:count`, `cast:handles`, `cast:names` | The whole roster, in turn order |
| `cast:llm.*`, `cast:player.*`, `cast:script.*` | The same three, per kind |
| `cast:agents.<handle>.name` / `.kind` / `.turn_order` | One cast member |
| `turn:index` | Turns already committed on this path |
| `turn:last.handle` / `.name` / `.kind` / `.said` | Who acted last; absent on the opening turn |

Only agents with a handle appear: an unhandled agent has no `agents:` subtree
and no `self:`, so listing it would name an identity nothing can address.

`cast:` and `turn:` are reserved in every host, the way `actor:` is — a state
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

`check` validates against the union of every host rather than the one being
edited, so `$[toast]` in a realm script is accepted by the editor and rejected
at runtime. The fix is `check(source, dialect, Option<WeaverProfile>)`, which
needs the frontend to send which context it is editing.
