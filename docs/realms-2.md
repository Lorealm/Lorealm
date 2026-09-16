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
| 16/09/26      | Editor built; blueprint/realm split redefined         | ✅            |

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
| **Blueprint** | The authored WTN document. Everything you write, you write here. | No |
| **Realm** | One instantiation of a blueprint. Owns conversations and versions. | Yes |
| **Conversation** | A realm plus a picked scenario. One playthrough. | — |

`expand(blueprint, inputs) → realm`, then `start(realm, scenario) → conversation`.

**What separates them is authored versus instantiated, not whether anything is
still a hole.** A blueprint that declares no `#inputs` is still a blueprint,
and still gets expanded — with `{}` — before anything can be played. There is
no document you author that is already a realm.

> **This changed.** The first version of this document drew the line at holes:
> a document with nothing unbound *was* a realm. That is no longer true, and
> the paragraph that said so has been rewritten rather than annotated. If you
> find the old framing anywhere, it is stale.

**Why the line moved.** Three reasons, in descending order of weight:

- **Provenance stops being optional.** A realm records the blueprint it came
  from and the inputs it was built with. Under the old split, a
  directly-authored realm had no origin, so "update from blueprint" and
  "rebuild with a different cast" worked for some realms and not others,
  depending on whether the author happened to declare an input. Now every
  realm has an origin.
- **Authoring stops having a mode.** Under the old split the thing you were
  editing changed identity the moment you declared your first input — a
  different save target, a different list, a different endpoint. That is the
  same over-sequencing this document complains about in the old editor, moved
  up a level.
- **It is what the shape is for.** One blueprint, many realms: a group chat
  built once and instantiated with whichever characters you want this time.
  Realms become numerous and cheap; the blueprint is the durable thing.

The backend needed no new semantics for this. `expand` already accepted an
empty input set and already wrote the origin.

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

A **singular** `Ref<Character>` is projected identically — `Needs::One` and
`Needs::Many` run through the same `project()` — so a lone reference fills a
field on its own, without a loop around it.

**Reading one, in either position.** `{{input:lead.id}}` works in expression
position *and* in a string. `{{input:lead}}.id` works only in expression
position: inside a quoted string the interpolation stringifies the whole object
to JSON and the `.id` becomes literal text. Emit the dotted form everywhere and
the question never comes up.

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

### A realm's structure is frozen
A realm cannot be edited. Its cast, its scripts and its director are whatever
the blueprint produced. To change any of it, edit the blueprint and build
again.

**Why:** the moment a realm can be edited, its `origin.inputs` stop describing
what it contains, and there are two authoring surfaces that drift. Building
again is cheap — that is the entire point of the split — and it keeps
provenance true.

**The trade:** there is no way to tweak one agent in one realm without
rebuilding. If that becomes painful, the answer is "fork this realm into a new
blueprint", not "make realms editable".

**Note the seam.** `PATCH /realms/{id}` and the agent CRUD under
`/realms/{id}/agents` still exist and still work. This is a decision the editor
keeps, not one the backend enforces. If it should be enforced, that is a
separate change, and worth making deliberately rather than discovering.

### Weaver fields cannot read inputs
`to_wtn` writes every string through `raw_string`, as `r"..."`. That is what
stops a realm full of Weaver source from being *evaluated* on the way back in,
and it is load-bearing. The consequence is that a script body or a run
condition containing `{{input:x}}` survives expansion **verbatim** and is
evaluated later, at conversation time, in a host where the `input:` namespace
does not exist.

So: an input can be read from a handle, a name, a description — anything
expanded during instantiation. It cannot be read from a script or a run
condition. The editor does not offer it there.

**Unresolved:** getting an expansion-time value into a script needs an escaping
story that does not exist yet. Until it does, the seam is real and the editor
is honest about it rather than producing something that fails much later.

### A realm always exports to a document
Export is not a special path; it is the same format the editor will edit.
Two properties the emitter guarantees:

- **Strings are written raw.** A realm is full of Weaver source — scripts, run
  conditions, format strings. Written plain they would be *evaluated* on the
  way back in.
- **Output is deterministic.** Keys are sorted, so the same realm exports
  byte-identically every time. It is a format people diff and distribute.

`POST /realms/import` is the way back in for an exported realm, and nothing
else. It is not an authoring path: authoring always produces a blueprint. That
is what makes it sit beside blueprints rather than competing with them.

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

And the editor, under `frontend/packages/ui/src/widgets/realms/`:

| | |
|---|---|
| `model.ts` | The block model: cast entries, bindings, input declarations |
| `draft.svelte.ts` | The open document, in memory. Holds the cast and selection |
| `serialize.ts` | Block model → WTN |
| `RadioField.svelte` | The picker/ping/drop-target every resource field uses |
| `AssetBrowser.svelte` | Inputs and owned resources, both drag sources |
| `RealmInspector.svelte` | Selection-driven: agent, loop, input, resource |
| `panes/` | The tabbed shell, plus the Cast, Document and Script panes |
| `stores/blueprint.svelte.ts` | Blueprint CRUD, save status, expansion |

The endpoint the editor will lean on hardest is `GET /blueprints/{id}/inputs`.
It returns the declarations as a closed, tagged vocabulary — `string`,
`number`, `bool`, `enum`, `list`, `reference` — which is what lets a UI render
`[Ref<Character>]` as a character multi-picker and `enum(..)` as a dropdown
without guessing. It reads declarations without evaluating anything, so a
blueprint whose inputs cannot currently be satisfied still describes itself.

---

## The editor

The prototype is gone and the replacement is a pane-driven shell. What exists:

**The shell.** An asset browser down the left, a tabbed pane area in the
middle, the inspector on the right, a diagnostics strip along the bottom that
is still an empty div. Cast and Document are permanent tabs; a script opens in
its own, closeable one. Only the active pane is mounted — a CodeMirror
instance kept alive behind a hidden tab costs a linter poll and buys nothing.

**The block model.** The cast is an ordered list of entries, each either an
agent or a `foreach` wrapping a list of agents. Agent fields hold *bindings*,
not raw ids: a picked resource, a loop variable, or a singular entity input.
The model is one-way — it emits WTN; nothing parses WTN back into it.

**The radio field.** The thing the whole editor leans on. Click the dot to
pick, click the body to reveal what it points at, double-click to open it.
Drop a resource or an input onto it; incompatible targets grey out while a
drag is in flight, so where something can land is visible rather than guessed.

Every one of these slots is an id — `character_id`, `template_id` — so a
binding names *which entity* and the emitter writes `.id`. There is no field to
choose. An early version offered `c.name` alongside `c.id` and would happily
have bound a name where the backend parses a Snowflake.

**The serialiser.** `toWtn` turns the block model into a document, following
the two rules `wtn.rs` guarantees: strings raw unless they are meant to
expand, keys sorted. Verified by generating a document with a loop, a singular
ref, a defaulted enum, a script and a player, then expanding it through the
real `from_wtn`.

**Diagnostics with a profile.** The Document pane checks against
`dialect: document, profile: realm_template`; a script pane against
`realm_script`; a run condition against `realm_condition`. This closes the
open question below — the backend had accepted a profile all along and the
frontend simply never sent one.

### What is not built

- **Scenarios.** The block model has no scenario authoring, so every document
  emits `scenarios: []` and the play menu can only ever offer an open start.
  This is the largest remaining gap.
- **Reading a document back.** Reopening a blueprint shows its source and an
  empty cast. Closing this needs either a WTN parser in TypeScript or a
  narrower one that only accepts documents the editor itself emitted.
- **Editing resources in a pane.** The ping still leaves for Studio. The
  blocker below is unchanged and still gates it.
- **The diagnostics strip.** Reserved, empty. Inline underlining carries it
  for now.
- **Routing for blueprints.** Realms route; blueprints open from store state,
  so they are not in history and do not survive a refresh.
- **Simulations / dry-runs.** Author-defined tests that check state moves the
  way it should. Still worth doing last.

### Requirements, as they stand

**Build blueprints without touching WTN.** Met for the cast. Not met for
scenarios, which do not exist yet.

Note the constraint the format imposes: loops are position-typed. A loop in
array position yields elements, in object position yields entries, and mixing
them is a *parse* error. The block model only ever puts a loop in array
position, so the mistake is unrepresentable rather than caught.

**Pane-driven.** Met, with three pane kinds.

**Centralized.** Not met.

> **The hidden blocker.** The Studio resource editors bind to singleton stores
> with one draft slot each and call the global navigator directly. Two panes
> editing two characters is currently impossible, and a pane that navigates
> moves the whole app. Stores need to go from "one editing slot" to "documents
> keyed by id", and editors need an id prop with navigation via context.
> `ToolsEditor` already takes a prop, so there is precedent. Budget for it — it
> is invisible from any design doc and it gates the entire pane model.

### Keep, don't delete

`src/weaver/editor/` is ~1,500 lines of CodeMirror support — language,
completion, diagnostics, hover, references, theming. It is not realm-specific
and it *is* the "IDE functionality" the brief asks for. `WeaverEditor` is the
full one (line numbers, active line, indent-on-input); `WeaverArea` and
`WeaverField` are the field-shaped variants. All three take a `profile`.

---

## Open questions

**Panes and plugins.** [`plugin-system.md`](./plugin-system.md) puts frontend
plugins in an iframe at pre-defined locations, and says plugins may not
provide realms. If the editor is to "extend infinitely", a pane is the natural
extension point and panes are not on that list. Either panes become a
first-class plugin entry point, or "infinitely extensible" means "by us".

**Provenance is head-level.** A realm's `origin` is on the realm, not per
version, so rolling back a version keeps the current origin rather than the
one that produced that version. This matters more now that every realm has an
origin rather than only some; revisit alongside "update from blueprint".

**`blueprint_id` can dangle.** Deliberate — deleting a blueprint leaves the
realms it made fully working, because a realm is not a view over its
blueprint. But every realm now has one, so a deleted blueprint is a visible
hole in the Realms list rather than an edge case, and "update from blueprint"
will have to handle a missing source. The list currently says "a deleted
blueprint" and moves on.

**Two expressions of one rule.** "Entity inputs cannot have defaults" lives in
the checker (as a diagnostic) and in the resolver (as a hard error at
expansion). Both paths are needed — a document can reach expansion without a
blueprint save — but they could drift.

**Getting an expansion-time value into a script.** See "Weaver fields cannot
read inputs" above. The seam is understood and currently just refused; what it
needs is an escaping story, and there is no proposal yet.

**Reusable scripts.** Authoring a script agent's body in its own pane covers
most of the complaint. Sharing one body across realms does not fit as a
resource: a blueprint *is* its WTN source, so there is nowhere to put a pointer
that is not the document itself, and an id in the document breaks the
distribution the format exists for. If it lands, it should be an authoring-time
insert that copies the body in — a library, not a link.
