# Realms 2.0

> AI Disclosure: This document was authored with the help of Claude.

A living document for the Realms rewrite. It supersedes the original design
brief: the backend work that brief called for is done, and the decisions taken
along the way changed enough of the model that the brief is now misleading in
places. Everything still true has been carried over.

## Updates
| Date DD/MM/YY | Summary                                              | AI-Assisted? |
|---------------|------------------------------------------------------|--------------|
| 16/09/26      | Written after the backend rework; frontend not begun  | ✅            |

---

## Terminology

### Lorealm
The platform everything takes place within. An LLM RP platform and a spiritual
successor to SillyTavern — "SillyTavern on steroids".

### Weaver
A custom DSL built for Lorealm. It powers every part of the application and is
the main reason Lorealm is as powerful as it is: it lets LLMs author world
state directly, through tools written in the language.

```
{# if {{self:gold}} < 150 #}
    {# return "A shinobi costs 150 koku; the treasury holds " + {{self:gold}} + "." #}
{# endif #}
$[inc_var("self:gold", -150)]
$[set_var("orders:" + {{actor:handle}}, {kind: "spy", target: {{cmd:target}}})]
{# return "A shinobi slips toward " + {{cmd:target}} + " tonight." #}
```

Where a given body may run, and what it may call there, is decided by its
**host**. See [`weaver-hosts.md`](./weaver-hosts.md) — that document is the
reference for hosts, profiles, dialects and the namespaces a realm mounts.

### Weaver Template Notation (WTN)
A declarative format added in Weaver 0.7: one value with holes, plus a block
declaring what fills them.

```
#inputs
regents: [Ref<Character>]
difficulty: enum("gentle", "brutal") = "gentle"

{
  name: "Sengoku"
  agents: [
    {# foreach c in {{input:regents}} #}
    { handle: "regent_{{c.slug}}", name: "{{c.name}}", character_id: c.id }
    {# endforeach #}
  ]
}
```

The point is distribution. An author builds a framework for a niche — a
comprehensive strategy game, say — and instead of editing half a dozen fields
every time someone wants a new character, they describe the *pipeline* that
recreates the environment from a few inputs.

### Blueprint, Realm, Conversation
Three stages, not two. This is the most important thing to internalise:

| | What it is | Playable? |
|---|---|---|
| **Blueprint** | A WTN document **with** unbound `#inputs`. A constructor. | No |
| **Realm** | A WTN document with **nothing** unbound. Owns conversations and versions. | Yes |
| **Conversation** | A realm plus a picked scenario. One playthrough. | — |

`expand(blueprint, inputs) → realm`, then `start(realm, scenario) → conversation`.

One document format across both entities; what separates them is whether
anything is still a hole. A realm is a blueprint with no inputs — which is why
export and import are the same two functions for both.

**Naming note:** "Template" was already taken in this codebase, thoroughly —
a *context template* is prompt assembly, it has its own Studio section, and an
agent's `template_id` means that. Hence Blueprint. Do not reintroduce
"template" for this concept; you will end up writing "the template's
template".

### Agent
An actor inside a realm. Three kinds:

- **Player** — the human, driven from the client
- **LLM** — a model
- **Script** — a slice of Weaver that runs over the state pool

Configuration differs per kind, but every agent converges on **handle**, name
and run condition.

### Scenario
A named opening, exactly analogous to a greeting on a character: name,
description, and the narration a playthrough starts on. It carries no state of
its own.

### Realm (what it's for)
A stage for multiple agents where everything is configurable — the underlying
system for everything that is not a simple user-to-character pipeline. Group
chats, larger worlds, fully enforceable game systems, worlds where every NPC
is an autonomous agent.

Abstractly: a realm maintains a Weaver state pool per conversation. Agents
evolve it. Everything else is gating access to that pool and building a
surface to interact with it.

---

## The decisions

These are the values the backend now embodies. Each exists for a reason; the
reason matters more than the rule.

### Handles are identity
A definition agent has no Snowflake. Its `handle` is its identity everywhere —
the `agents:<handle>` state subtree, the `cast:` roster, the source of its
events, the key of its observation feed.

**Why:** a template generates its cast in a loop, and a loop can mint a handle
from its input. It cannot mint a Snowflake.

### Array order is turn order
No `turn_order` field. The cast array is the order.

**Why:** same reason. A `foreach` produces members in order for free; a
per-agent integer is another thing a loop would have to invent. Where order
genuinely matters, use a custom director.

### Nothing is declared before it is used
There is no import manifest. The resources a realm references are **inferred**
from its cast. Ownership is the gate — every reference is checked against the
caller, nothing else.

**Why:** the old manifest bought nothing the ownership check didn't, and cost
an entire authoring step. This was the single largest source of the
over-sequencing the original brief complained about, and it lived in the
backend, so no amount of frontend rewriting would have fixed it.

### Entity inputs are ids in, objects inside
`Ref<Character>` is supplied as an **id** — that is what a picker produces and
what a saved set of inputs stores — and bound as a small object:

```
{ id, name, description, slug }
```

Resolved before expansion, in one pass, ownership-checked. Inside the document
the author writes `{{c.name}}` for prose and `c.id` where a reference belongs.
`slug` is the name reduced to a Weaver identifier, so a loop can mint handles
from it.

**Why lean:** every field here is a promise to templates already published.
Expand it when something needs it, not before.

**Why before expansion:** the language hands a `Ref` to a synchronous
callback, and every lookup behind it is async. Resolving up front also batches
the queries and reports a dead reference before any of the document runs.

**Consequence:** an entity input cannot carry a default. There would be
nothing to project at the moment resolution happens, so it would bind as a
bare id and every `{{c.name}}` reading it would come back empty. Refused at
save, underlined in the editor.

> The original brief's example, `"You are {{c}}."`, does not do what it looks
> like. `c` is an object; that renders the whole thing. It is `{{c.name}}`.

### Scenarios hold no state
Seed scripts and per-agent overrides are gone. Whatever an opening needs is
set up by a script agent gated on the first turn, branching on which scenario
was picked:

```
{# if {{turn:index}} == 0 && {{scenario:name}} == "Spring" #}
    $[set_var("world:season", "spring")]
{# endif #}
```

**Why:** overrides were keyed by handle, which stops meaning anything the
moment handles come out of a loop, and a seed script was a second mechanism
for what properties already do. This also puts the setup *in the cast*, where
it is visible, instead of hidden in a field.

**The trade:** a seed ran before anything else, atomically. A script agent
runs when the director says so. Put the seeder first and keep turn zero
deterministic; a random director that depends on turn-zero seeding is a bug,
and eventually a diagnostic.

### Expansion is a one-time materialization
A realm stores its expanded document, not a pointer to the blueprint it came
from. It also stores **provenance** — the blueprint id and the inputs as
supplied.

**Why:** a pointer would mean a blueprint edit retroactively rewrites a realm
that already has live playthroughs. Keeping the inputs means "update from
blueprint" can later land as a new realm *version*, which conversations
already pin against.

### A realm always exports to a document
Export is not a special path; it is the same format the editor will edit.
Two properties the emitter guarantees:

- **Strings are written raw.** A realm is full of Weaver source — scripts, run
  conditions, format strings. Written plain they would be *evaluated* on the
  way back in.
- **Output is deterministic.** Keys are sorted, so the same realm exports
  byte-identically every time. It is a format people diff and distribute.

### One checker, and it knows what it is looking at
`check(source, dialect, profile)`:

- **dialect** — is this text, an expression, or a WTN document? A document fed
  to the text compiler parses *as prose* and reports nothing, which is worse
  than reporting nothing on purpose.
- **profile** — which host will run this? Without it, checking happens against
  the union of every host, which accepts commands the real one refuses.

Blueprint saving goes through this same function, so the editor cannot show a
document clean that saving then refuses. There is a test pinning exactly that.

---

## Where things live

The backend side of this rewrite is finished and covered by tests:

| | |
|---|---|
| `services/realm/wtn.rs` | Document ↔ definition, both directions |
| `services/realm/project.rs` | `Ref` resolution and the projection shape |
| `services/realm/validate.rs` | Per-field and whole-definition validation |
| `services/blueprint/` | Blueprint CRUD, input declarations, expansion |
| `services/context/weaver/check.rs` | The checker — dialects, profiles, diagnostics |
| `server/src/tests/` | Service-level tests on an in-memory harness |

The endpoint the editor will lean on hardest is `GET /blueprints/{id}/inputs`.
It returns the declarations as a closed, tagged vocabulary — `string`,
`number`, `bool`, `enum`, `list`, `reference` — which is what lets a UI render
`[Ref<Character>]` as a character multi-picker and `enum(..)` as a dropdown
without guessing. It reads declarations without evaluating anything, so a
blueprint whose inputs cannot currently be satisfied still describes itself.

---

## The editor

Nothing has been built. The prototype still stands and still has its two
original problems:

**Over-sequencing.** Partly fixed — the manifest is gone, so any resource the
owner has is bindable immediately. The rest is UI.

**No overview.** Untouched. Seven isolated full-view tabs (Imports went with
the manifest), and editing a character still means leaving the editor entirely
for Studio.

The plan is unchanged: delete the prototype, rebuild as something closer to a
game engine — a pane-driven shell with an inspector, an asset browser, a
diagnostics pane, and room to grow.

### Requirements

**Build blueprints without touching WTN.** The hard one. The instantiation
form is already free — parse the document, read its declarations, render the
fields. Authoring is the open problem. What helps: a document is one value
with holes, and a realm is the same value with none, so a graphical editor
that always emits WTN gets the non-templated case for free. There is no
"realm mode" and "blueprint mode" to build.

Note the constraint the format imposes: loops are position-typed. A loop in
array position yields elements, in object position yields entries, and mixing
them is a *parse* error. Bake that into the block model rather than
discovering it.

**Pane-driven.** Inspector, asset browser, diagnostics, and more over time.

**Centralized.** No leaving the editor to edit a character. Cheaper than it
sounds — the Studio resource editors are already standalone components, so
this is mounting, not rewriting.

> **The hidden blocker.** Those editors bind to singleton stores with one
> draft slot each and call the global navigator directly. Two panes editing
> two characters is currently impossible, and a pane that navigates moves the
> whole app. Stores need to go from "one editing slot" to "documents keyed by
> id", and editors need an id prop with navigation via context. `ToolsEditor`
> already takes a prop, so there is precedent. Budget for it — it is invisible
> from any design doc and it gates the entire pane model.

**Simulations / dry-runs.** Author-defined test scenarios that check state
moves the way it should, so live-LLM testing is the exception. Nothing exists
yet. Worth doing last — what you would want to simulate keeps changing until
the rest settles.

### Keep, don't delete

`src/weaver/editor/` is ~1,500 lines of CodeMirror support — language,
completion, diagnostics, hover, references, theming. It is not realm-specific
and it *is* the "IDE functionality" the brief asks for.

---

## Open questions

**The frontend does not send a profile.** The backend takes one; `weaverCheck`
still checks everything against the union. First thing to fix when the editor
work starts, because a diagnostics pane is the part of an editor users decide
whether to trust.

**Panes and plugins.** [`plugin-system.md`](./plugin-system.md) puts frontend
plugins in an iframe at pre-defined locations, and says plugins may not
provide realms. If the editor is to "extend infinitely", a pane is the natural
extension point and panes are not on that list. Either panes become a
first-class plugin entry point, or "infinitely extensible" means "by us".

**Provenance is head-level.** A realm's `origin` is on the realm, not per
version, so rolling back a version keeps the current origin rather than the
one that produced that version. Fine for now; revisit alongside "update from
blueprint".

**`blueprint_id` can dangle.** Deliberate — deleting a blueprint leaves the
realms it made fully working, because a realm is not a view over its
blueprint. But "update from blueprint" will have to handle a missing source.

**Two expressions of one rule.** "Entity inputs cannot have defaults" lives in
the checker (as a diagnostic) and in the resolver (as a hard error at
expansion). Both paths are needed — a document can reach expansion without a
blueprint save — but they could drift.

**`POST /realms/import` sits beside blueprints.** It takes a document with
nothing unbound, which is how an exported realm comes back in. Worth
confirming that is still the split you want once the editor exists.
