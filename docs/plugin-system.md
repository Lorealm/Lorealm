# Plugin System

A living document to help us keep track of the plugin system implementation.

## Updates
| Date DD/MM/YY | Summary                             | AI-Assisted? |
|---------------|-------------------------------------|--------------|
| 09/09/26      | Created the initial ground plan     | ❌            |
| 10/09/26      | Extended the plan with more details | ❌            |
| 23/09/26      | Switched to the component model     | ✅            |
| 23/09/26      | Resolved capability registration    | ✅            |
| 23/09/26      | Added plugin logging                | ✅            |
| 23/09/26      | Added load order and compatibility  | ✅            |
| 23/09/26      | Added Weaver and template support   | ✅            |
| 23/09/26      | Added secure frontend slots         | ✅            |
| 23/09/26      | Added message slots and unsafe mode | ✅            |
| 23/09/26      | Added installation from source      | ✅            |
| 24/09/26      | Validated and surfaced permissions  | ✅            |
| 24/09/26      | Added plugin settings               | ✅            |
| 24/09/26      | Added API key types                 | ✅            |
| 24/09/26      | Added the settings host function    | ✅            |
| 24/09/26      | Added http-fetch                    | ✅            |
| 24/09/26      | Gated fetching and autoplay         | ✅            |
| 24/09/26      | Added the TTS example plugin        | ✅            |
| 24/09/26      | Planned hook kinds and custom hooks | ✅            |
| 24/09/26      | Added plugin events                 | ✅            |
| 24/09/26      | Added the after-generate hook       | ✅            |
| 24/09/26      | Added the message-finalize hook     | ✅            |
| 24/09/26      | Added the user-message gate         | ✅            |
| 24/09/26      | Added the lorebook-activate hook    | ✅            |
| 24/09/26      | Added the tool-call gate            | ✅            |
| 24/09/26      | Added the tool-result hook          | ✅            |
| 24/09/26      | Added the realm turn hooks          | ✅            |

## Ambitions
The plugin system is the main way we provide extensibility to users. It is important we get this right before Lorealm goes public.

Our plugin system should allow users to:
- Author plugins in an ergonomic environment with helpful IDE support.
- Extend both the backend server and the (probably official) frontend client within a single code base.
- Implement new Weaver logic (commands/processors).
- Persist arbitrary data to our configured storage backend.
- Should be permission gated.
- Should provide near-unlimited extensibility, at the cost of security.

## The Plan
A plugin is inherently per-user. They will be loaded on demand when a user logs in, then cached for either the duration of the session, or a smaller time window, then simply reloaded on demand.

A plugin is defined as two parts: the backend code: compiled to wasm, and the client bundle: requested by the client and handled accordingly.

When a plugin wants to define frontend components, they have two routes:
**Secure** (default) - Limits plugin authoring to within an iframe. This is limited in the sense that a plugin can only exist at certain pre-defined locations, can't reliably control exterior styling and is limited to what the client provides.
We will provide paths for common things, such as inspector modules, custom chat widgets, etc.
**Unsafe** - Gated both by a plugin-level permission and a user-level permission in `ALLOW_UNSAFE_PLUGINS`. Allows (near) direct DOM access. Realistically speaking, this is going to be neccessary for plugins that want to add more advanced behavior.

#### Frontend
The manifest declares where the plugin appears; the client decides which slots exist and ignores ones it doesn't know. The official client has `inspector` (a widget in the inspector grid) and `message` (under assistant messages, like static widgets).
```toml
[frontend]
path = "frontend"          # defaults to "frontend"

[[frontend.slots]]
slot = "inspector"
entry = "inspector.html"   # a single self-contained HTML file
title = "Greeter"

[[frontend.slots]]
slot = "message"
entry = "message.html"
title = "Word count"
mode = "every"             # message slots: "latest" (default) or "every"

[[frontend.slots]]
slot = "inspector"
entry = "panel.js"         # an unsafe slot is an ES module
title = "Greeter"
unsafe = true
```
Both modes get the same API: `call(name, args)` reaches only the plugin's own backend (`POST /plugins/{handle}/call`, delivered to `handle` as a `frontend` event), `context` is what the slot provides (a `message` slot gets `{ message: { id, role, content, updated_at } }`; `inspector` gets `{}`), and `onContext(listener)` reports changes. Anything touching user data goes through the backend half, where permissions apply. A secure frame may only play sound after the user clicks in it, unless the plugin has `ui:autoplay`, which lets its frames play sound on their own (`allow="autoplay"`).

**Secure** (the default): the client fetches the entry through the API (`GET /plugins/{handle}/frontend/{entry}`, only for declared entries of enabled plugins) and renders it in an `iframe sandbox="allow-scripts"` as `srcdoc`, so it runs in an opaque origin with no session or access to the app. A CSP is put in front of it: no network (`connect-src` falls back to `default-src 'none'`), and no scripts, styles or images from outside the file, so entries inline everything. The API is `window.lorealm`. The app's style variables (everything declared on `:root`) are applied ahead of the entry's own styles, and the font faces the app has loaded are handed over as bytes, since the CSP refuses font URLs. Both are kept up to date, and user widgets get them the same way.

**Unsafe**: needs the plugin's `ui:unsafe` permission and the user's `ALLOW_UNSAFE_PLUGINS`; otherwise the slot isn't listed or served. The entry is an ES module, imported into the client, exporting `mount(element, api)` and optionally returning an unmount function. It runs with the client's full privileges, and inherits the app's styles directly.

Known gaps: a secure frame can still navigate itself to an outside URL, which CSP can't prevent; unsafe code in the desktop app shares the webview with Tauri's IPC.

### Plugin Development
Day one support includes *at least* Rust and Python, the native language and the most popular language.

Plugins are WebAssembly components (wasmtime + WIT), not Extism modules. The WIT world lives in `backend/plugins/wit` and is the single source of truth for bindings in every language (`wit-bindgen` for Rust, `componentize-py` for Python). Both are exercised by the same test fixture in `backend/plugins/tests/fixtures` (`hello`, `hello_py`). Python components bundle CPython (~18 MB) and take roughly a second to compile in release builds; compiled plugins are cached, so this is paid once per plugin.

WASI is linked so standard toolchains work, but the sandbox grants nothing by default: no filesystem, environment, arguments or network.

Every call into a plugin (`init` or a hook) has a wall-clock limit, currently 5 seconds, that includes time spent in host functions, except `http-fetch`. A plugin that exceeds it is stopped and unmounted until the user's plugins are next loaded.

### Manifest
Every plugin must ship with a manifest.toml file. A manifest file ships the plugin definition, and will contain *at least* these fields:
#### Plugin object
- name (human readable name)
- handle (used for weaver namespaces, must be unique, FCFS)
- description
- tags
- author
- version (semver)
- permissions (one of `chat:read`, `chat:write`, `data:persist`, `net:fetch`, `persona:read`, `prompt:write`, `state:read`, `state:write`, `tools:write`, `ui:autoplay`, `ui:unsafe`; any other is refused when the plugin is installed or loaded, and the Plugins page lists them)

#### Compatibility (optional)
- loaded_before (array of handle (or author+handle), this plugin must be loaded before these plugins)
- loaded_after (same as above, must be loaded after these plugins)
- requires
- recommends (for the marketplace)
- conflicts

#### Structure (optional)
- backend (the relative path to the built backend component, defaults to "plugin.wasm")
- readme (the relative path to a Markdown README, defaults to "README.md"; optional). Shown on the plugin's page in the client, and later in the marketplace.
- settings (the relative path to the settings file, defaults to "settings.wtn"; optional). See Settings.

#### Network (optional)
- hosts (where the plugin may send requests, e.g. `["api.elevenlabs.io", "localhost:8880"]`). A host alone matches the scheme's default port; `host:port` matches that port. Listed on the plugin's page.

#### Provides (optional)
- events (the events the plugin may emit, as names under its handle, e.g. `["ping"]` for `handle.ping`). See Custom hooks.

#### Definitions (optional)
- api_keys (API key types, e.g. `["elevenlabs"]`; identifiers that don't match a built-in type). See Settings.

#### Build (optional)
- backend (a command that builds the backend component, as a list of arguments)
- frontend (the same, for the frontend)

Each runs in the plugin's source directory when it's installed from source; see Installation.

Here's an example:
```toml
[plugin]
name = "Markdown Preview"
handle = "janedoe"
description = "Live-renders Markdown files in a side panel as you type."
tags = ["markdown", "preview", "editor"]
author = "Jane Doe <jane@example.com>"
version = "1.2.0"
permissions = ["prompt:write", "persona:read"]

[compatibility]
loaded_before = ["mermaid_render"]
loaded_after = ["markdown_extensions"]
requires = { markdown_extensions = "^1.4" }
recommends = ["syntax_highlight"]
conflicts = ["other_md_preview"]

[structure]
backend = "build/plugin.wasm"
frontend_path = "src/frontend"
```

### Marketplace
For the time being, the marketplace will be a simple GitHub repository, similar to how [A111's plugin marketplace](https://github.com/AUTOMATIC1111/stable-diffusion-webui-extensions) works.
If Lorealm sees enough adoption, time may be spent to create an official marketplace.

Optionally, we could also inherit Jellyfin's model where you can add custom plugin indices created by plugin authors.

### The Plugin Loader
The plugin loader is simple in theory. Simply put, these are the steps:
1. Starts init; triggered by user activity (axum middleware?)
2. Retrieves plugins from database table, filtered by user
3. Loads plugins based on load orders and constraints. Calls each plugin's `init` function. Collects any potential errors and temporarily persists them for client-side debugging. A simple, less specific error is sent to the client
to inform them something went wrong.
4. The plugin `init` functions in turn registers what it uses through `register_*` functions (permissions errors propagated to the last step)
5. Plugins are cached in-memory and are now active.

### Installation
Plugins are installed server-wide, one directory per version: `data/plugins/{handle}@{version}`. Users pin a version: enabling picks the newest, and a newer install doesn't move anyone. A user moves with `POST /plugins/{handle}/update` (to the newest, or a given `version`), which is refused if their other plugins wouldn't load together on it. A version is removed from disk once no user is on it, unless it's the newest. Plugin data and logs are keyed by handle, so they carry across versions.

**From source** (`POST /plugins/install {"path": "..."}`, needs `ALLOW_PLUGIN_INSTALL_FROM_SOURCE`): reads the manifest in that directory, runs its `[build]` commands there, and copies only the result (manifest, backend, frontend) into place, with the manifest's paths rewritten to fixed ones. The build output comes back in the response, and a failure's tail goes to the installer's log for that plugin. The build runs with the server's rights, so this permission amounts to running code on the host; it's not granted by default. Installing a version that's already installed is refused.

**Uninstalling** (`DELETE /plugins/{handle}`, same permission) stops every version from being enabled; each is removed once no user is on it.

**Planned:** installing from a git URL; installing pre-built plugins under a separate, less powerful permission.

### Load Order and Compatibility
A user's plugins load in the order they set (`PUT /plugins` with `{ "order": [...] }`, listing every enabled plugin; enabling appends to the end). Manifests override that order where they need to:
- `requires`: the plugin must be enabled at a matching version, and loads after it.
- `conflicts`: the two can't both be enabled. Either side declaring it is enough.
- `loaded_before` / `loaded_after`: ordering only, and only between plugins that are both enabled.
- `recommends`: marketplace only; ignored by the server.

Enabling a plugin fails if it conflicts with an enabled plugin, needs one that isn't enabled, or would create a load-order cycle. Disabling one fails while other enabled plugins require it. The same rules run again whenever a user's plugins load, since installed files can change: a plugin that no longer fits is left out, with the reason in its log.

### Entry Points
Lorealm will only serve one type of plugin which covers everything. This plugin will be able to:
- Add custom ContextTemplate nodes
- Add custom Weaver commands and/or processors
- Attach metadata to messages (needed for frontend rendering)
- Register new entries in the filter chain
- Register new tools and commands

Plugins will not be able to directly provide content such as (directly as in concrete entries; we don't know what goes on during prompt injection or in an unsafe frontend plugin):
- Characters
- Lorebooks
- Realms

### Weaver and Context Templates
During `init`, a plugin can register:
- **commands** with `register-command`, called as `$[handle.name(args)]`.
- **processors** with `register-processor`, called as `@[handle.name(properties)]`.
- **context template nodes** with `register-source-node`, listed under a source with id `plugin:{handle}`.

Each call reaches the plugin through `handle(event)`, which answers with the matching `response` case. Weaver values cross the boundary as JSON text, since WIT has no recursive types.

A command is lent an `eval-context` for the length of its call, to `get` and `set` the evaluation's variables. Reading needs `state:read`; writing needs `state:write` and a command that declared `mutates-context`. The host context still decides what exists and what can be written, so a read-only host (a run condition, say) refuses writes. The context can't be kept: it's a borrowed resource, which the component model invalidates when the call returns. Triggers and documents aren't reachable through it yet, since they re-enter Weaver and could call back into the same plugin.

Plugin callables are available wherever built-in vocabulary is: every Weaver host (tool bodies, slash commands, realm scripts, conditions and directors, widget renders, WTN documents), lorebooks, and the editor's vocabulary and checker. A plugin can't add processors to a namespace a built-in already uses. A template node whose plugin isn't loaded is skipped (the plugin's log says why) instead of failing the generation.

Registering needs no permission: a registered callable only runs when the user's own script or template calls it.

### Settings
A plugin's settings are the `#inputs` block of a WTN document, and nothing else; the user fills them in under Settings → You → Plugins, which the plugin list and the plugin's page link to.
```
#inputs
voice: enum("rachel", "adam") = "rachel"
speed: range(0.5, 2) = 1
autoplay: bool
```
The file must parse when the plugin is installed. Values are checked by Weaver the same way a blueprint's inputs are, when saved (`PUT /plugins/{handle}/settings`, refused with the reason) and when read (`GET`, with defaults applied). Settings are kept per user and plugin, and outlive disabling it.

An API key setting is `Ref<type>`, where `type` is declared in `[definitions] api_keys` by any of the user's enabled plugins, so one plugin can declare a type that others use through `requires`. Any number of plugins may declare the same type. The user adds keys of these types in the key manager, and a setting holds the id of one of their own keys of that type, or `null` for none (so `key: Ref<type> = null` is optional); the plugin never sees the key itself. A `Ref` whose type no enabled plugin declares, and built-in types like `Ref<Character>`, are refused for now.

A plugin reads its settings with `settings()`: a JSON object with every input, set to the user's value or its default, with API key settings as key ids. It needs no permission. Key types are those declared by the user's loaded plugins. While the saved values don't fit (a required setting not filled in yet, or values an update made invalid), the call fails with the reason, and the plugin's log gets a warning telling the user where to fix them.

### Plugin Hooks
On the server, we don't care what goes on inside the WASM sandbox. The main way a plugin triggers based on activity is through hooks.

#### Hook kinds (FOG)
Every hook is one of three kinds, which decides how the host runs it:
- **Filter (F)**: takes a value and returns it, possibly changed. Runs in load order, each plugin seeing the previous one's output, and blocks whatever triggered it. Used sparingly.
- **Observe (O)**: fire-and-forget. The host doesn't wait for it and may deliver it later or to several plugins at once. It can't change anything. Most hooks are this kind.
- **Gate (G)**: a filter that can also refuse: it answers allow, deny with a reason, or a changed input.

The kind is part of the WIT, so an observer can never hold up a generation. Variations share one hook with a reason field rather than getting their own (`before-generate` has a `kind` of `new`, `regenerate`, `continue` or `swipe`), and `register-hook` should take a filter (only realms, only a given tool, ...) so plugins aren't called for everything.

#### Planned hooks
`user-message` (needs `chat:write`), `lorebook-activate` and `prompt-build` (both need `prompt:write`), `message-finalize` (needs `chat:write`), `after-generate` (needs `chat:read`), `tool-call` and `tool-result` (both need `tools:write`), and `turn-start` and `turn-end` (both need `chat:read`) exist so far. `user-message` is a gate: it gets the conversation and the text the user sent for a reply (a new message or a branch, not an edit, which starts no generation) before it's saved, and answers a `verdict` of `allow`, `change(text)` or `deny(reason)`. The first denial stops the rest; nothing is saved, and the request fails with `chat.refused`, naming the plugin and its reason. `message-finalize` gets a chat reply's conversation and content just before it's saved, and returns the content to save; the client shows the saved version once streaming ends. `after-generate` then gets the saved reply's conversation, message id and content. Realm turns fire none of these except `lorebook-activate`, which runs wherever lorebooks are assembled (chat, realms, slash commands): once per world info node in the template, with that node's slot (`prelude`, `setting`, ...) and its activated entries as `{id, content, priority}` in insertion order. It returns the entries to insert, and may drop, rewrite, reorder or add them; the node's `max_entries` applies afterwards. `tool-call` is a gate that runs wherever tools do (chat, realm agents), before each call the model makes: it gets the tool's name and arguments as JSON, and answers `allow`, `change(arguments)` or `deny(reason)`. A change that isn't JSON is ignored and logged. A denial stops the rest, and the tool doesn't run; the model gets "refused by plugin `handle`: reason" as the call's error result, and the reply carries on. `tool-result` then gets every call's name, arguments, result (what the tool returned, or its error message as a JSON string, refusals included) and whether it's an error, and returns the result the model sees, as JSON. It can't turn an error into a success or back; a change that isn't JSON is ignored and logged. `turn-start` and `turn-end` observe every realm turn (an agent's, whether or not it calls the model, or a player's): `turn-start` gets the conversation and the acting agent's handle once the agent is picked, before its prompt is built, and `turn-end` also gets the saved message's id and content. Observers aren't ordered against each other, so a quick turn's `turn-end` may arrive before its `turn-start`.

| Area          | Hook                                                  | Kind  |
|---------------|-------------------------------------------------------|-------|
| Generation    | `before-generate`                                     | G     |
|               | `prompt-build`                                        | F     |
|               | `before-request` (model, sampling, stop, max tokens)  | F     |
|               | `stream-chunk` (batched, not per token)               | O     |
|               | `message-finalize` (the reply before it's saved)      | F     |
|               | `after-generate`                                      | O     |
|               | `generate-error`, `generate-abort`                    | O     |
| Messages      | `user-message` (the input before it's saved)          | F/G   |
|               | `message-edited`, `message-deleted`, `branch-activated` | O   |
|               | `conversation-created`, `conversation-deleted`        | O     |
| Context       | `lorebook-activate` (entries before insertion)        | F     |
|               | `variable-changed`                                    | O     |
| Tools         | `tool-call`                                           | G     |
|               | `tool-result`                                         | F     |
| Realms        | `turn-start`, `turn-end`                              | O     |
|               | `select-speaker`                                      | F     |
|               | `player-turn`                                         | F/G   |
|               | `action`                                              | G     |
|               | `realm-event`, `observation`                          | O     |
| Lifecycle     | `upgrade(from-version)` (migrate persisted data)      | —     |
|               | `settings-changed`                                    | O     |
|               | `shutdown` (flush before unloading)                   | —     |

Rough order: `after-generate`, `message-finalize`, `user-message`, `lorebook-activate`, `tool-call`, then realms. A scheduled `tick` hook is undecided, since plugins only load while their user is active.

#### Custom hooks
Plugins should be able to talk to each other, so one plugin can act as a library or "script extender" that others build on. This comes in two forms:
- **Events** (observe-only): `emit("handle.topic", json)`, which other plugins subscribe to. A plugin can only emit under its own handle. Events are delivered after the emitting call returns, never during it: a plugin's instance can't be re-entered, so A emitting to B, which emits back to A, would otherwise deadlock. Chains of events have a depth limit.
- **Services** (request/response): a plugin registers callable functions, which plugins that `require` it call with `call("handle", "name", json)`. `requires` then versions that API and orders loading. A call into a plugin already on the call stack is refused. The callee runs with its own permissions, since the user granted them to it, and is told who's calling so it can decide what to allow. Time spent in the callee counts against the callee.

Both are declared in the manifest (`[provides] events` and `exports`, with JSON schemas for their payloads), so the Plugins page can show what depends on what and an SDK can generate typed bindings. Payloads are JSON, so custom hooks never change the WIT.

**Events exist so far.** A plugin calls `subscribe(topic)` and `unsubscribe(topic)` whenever it likes, and `emit(topic, json)` outside `init`. Subscriptions last until the plugin is unloaded (idle, or after a crash), and `init` runs again when it next loads, so a plugin that subscribes later keeps its own record, with `persist-data`, and subscribes again from `init`. An emitted `topic` is `handle.name` under the plugin's own handle, with `name` listed in `[provides] events`; anything else is refused and logged. Subscribers get a `custom` event with the topic and payload, in load order, once the emitting call has returned (still before the request that led to it finishes). Each delivery is its own call, with its own time limit. A failed call's events are dropped. Events emitted while handling an event are delivered in turn, up to 8 deep; past that they're dropped and the plugin's log says so. Payloads aren't checked, and `[provides]` has no schemas yet.

**Planned:** record what each plugin's `prompt-build` hook changed, diffing the prompt before and after every plugin, so the context inspector can show which plugin changed what.

### Logging
Every plugin has a log per user, readable by that user at `GET /plugins/{handle}/logs` (newest first). Each entry records its `source`:
- **host**: written by the server: permission denials, failed calls (errors, crashes, timeouts), a plugin stopped after a crash, and plugins that failed to load.
- **guest**: written by the plugin, through the `log(level, message)` import or by printing to stdout (`info`) or stderr (`warn`). Uncaught Python exceptions and Rust panics land here.

Only the host sets `source`, so a plugin cannot pass its lines off as the server's. Limits: messages are truncated at 4 KiB, a single call keeps at most 100 entries (the rest are counted in a host entry), and each user keeps the newest `plugins.log_cap` entries per plugin (default 500). Host entries are mirrored to the server log at their own level; guest entries only at `debug`.

**Planned:** admins can read any user's plugin logs, so a malicious plugin can't hide from server owners. A separate audit log comes later.

### Host Commands
Apart from hook registering, a plugin will need to ship with a variety of commands to interact with the server. Most of these are permission-gated:
- `settings()`: the user's settings for the plugin; see Settings.
- `http-fetch(request)`, gated by `net:fetch`: sends an http(s) request to a host in `[network] hosts` and returns the whole response (at most 16 MiB). Redirects aren't followed, so a request can't leave those hosts. A header value may contain `{secret:name}`, where `name` is a setting declared as `Ref<type>` of an API key type: the server puts the user's key there, only over https or to this machine, and the plugin never sees it. Time spent waiting doesn't count toward the call's limit. Instead, the request fails if connecting takes over 10 seconds or the response goes 30 seconds without sending anything; a steady stream, like audio, can take as long as it needs. Requests carry `User-Agent: Lorealm/{version} (plugin {handle}@{version})` unless the plugin sets its own.
- `persist-data(key, value)`, `load-data(key)`, `delete-data(key)`: bytes stored per user and plugin (keys up to 256 bytes, values up to 1 MiB), gated by `data:persist`. Data outlives disabling the plugin.
- `generate_raw(prompt: String)`
- `generate()`
- `get_character`, `get_lorebook`, `get_context_template`, `etc.`
- `update_character`, `update_lorebook`, `update_context_template`, `etc.`

### Example
`plugins/tts` is a complete plugin in Rust: it reads replies aloud with OpenRouter or ElevenLabs. It shows settings with API keys (declaring `openrouter` to reuse the user's existing keys, and `elevenlabs` as a new type), `http-fetch` with `{secret:name}`, a `message` slot, and `ui:autoplay`. Install it with "Add from source" and its folder. A server test installs it and exercises it short of calling a provider.

### Proposed Plugin Structure
```
plugin
├── .gitignore
├── manifest.toml
├── backend
│   ├── wit (the Lorealm WIT world)
│   └── src
└── frontend
    └── src
```

## Open Questions
1. The frontend in this project is the official one, but is only one of potentially many. Clients will naturally have different ways of handling plugins. How do we want to handle these cases semantically? The two main options are:
  a. We rely on authors documenting their plugins targeting non-official clients
  b. We implement an optional `client_id` field in the manifest which accepts an arbitrary string. Clients can use these to broadcast support capabilites.
2. How do we want to handle plugin installation? For max "user-friendliness", the ideal way is to paste in a GitHub link and let the server build the .wasm file from source. The other option is only accepting pre-built .wasm files,
but this creates a significant technical overhead on plugin authors.
3. ~~How do plugins register capabilities?~~ **Resolved:** through the interface. A WIT world can't have optional exports, so a plugin exports a single `handle(event)` and subscribes from `init` with `register-hook(hook)`. Permissions are declared in the manifest and checked at the host-function boundary, so `register-hook` fails with a clear error if the permission is missing.
4. ~~What order should plugins run in?~~ **Resolved:** the user's order, overridden by manifest constraints. See Load Order and Compatibility.
