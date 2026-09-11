# Plugin System

A living document to help us keep track of the plugin system implementation.

## Updates
| Date DD/MM/YY | Summary                             | AI-Assisted? |
|---------------|-------------------------------------|--------------|
| 09/09/26      | Created the initial ground plan     | ❌            |
| 10/09/26      | Extended the plan with more details | ❌            |

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

### Plugin Development
Day one support includes *at least* Rust and Python, the native language and the most popular language.

The enviornment is undecided. We'll have to see if extism's PDK generation is sufficient, if not, we'll be maintaing a seperate repository that allows users to scaffold the plugin environment via a simple CLI tool.

### Manifest
Every plugin must ship with a manifest.toml file. A manifest file ships the plugin definition, and will contain *at least* these fields:
#### Plugin object
- name (human readable name)
- handle (used for weaver namespaces, must be unique, FCFS)
- description
- tags
- author
- version (semver)
- permissions

#### Compatibility (optional)
- loaded_before (array of handle (or author+handle), this plugin must be loaded before these plugins)
- loaded_after (same as above, must be loaded after these plugins)
- requires
- recommends (for the marketplace)
- conflicts

#### Structure (optional)
- backend_path (the relative path to the backend code, defaults to "backend")
- frontend_path (same, but for frontend, defaults to "frontend")

Here's an example:
```toml
[plugin]
name = "Markdown Preview"
handle = "janedoe"
description = "Live-renders Markdown files in a side panel as you type."
tags = ["markdown", "preview", "editor"]
author = "Jane Doe <jane@example.com>"
version = "1.2.0"
permissions = ["fs:read", "ui:modal"]

[compatibility]
loaded_before = ["mermaid_render"]
loaded_after = ["markdown_extensions"]
requires = { markdown_extensions = "^1.4" }
recommends = ["syntax_highlight"]
conflicts = ["other_md_preview"]

[structure]
backend_path = "src/backend"
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

### Plugin Hooks
On the server, we don't care what goes on inside the WASM sandbox. The main way a plugin triggers based on activity is through hooks.
Here is a non-exhaustive list of hooks that will (probably) exist:
- `on_prompt_build`
- `before_request`
- `on_stream_chunk`
- `after_generate`
- `on_generate_error`
- `on_generate_abort`
- `on_message_edit`
- `on_message_delete`
- `on_regenerate`
- `on_continue`
- [More...]

### Host Commands
Apart from hook registering, a plugin will need to ship with a variety of commands to interact with the server. Most of these are permission-gated:
- `persist_data(key: String, value: Any)`
- `generate_raw(prompt: String)`
- `generate()`
- `get_character`, `get_lorebook`, `get_context_template`, `etc.`
- `update_character`, `update_lorebook`, `update_context_template`, `etc.`

### Proposed Plugin Structure
```
plugin
├── .gitignore
├── manifest.toml
├── backend
│   ├── pkg (extism/custom PDK types and wrappers)
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
3. When plugins want to register a capability, we have options. The first option is to be completely manifest driven. This keeps everything: permissions and hooks, in one place. It is a lot more finnicky to get around, though.
The other option involves providing an interface through the PDK that lets authors just do: `register_processor([...])`. This is a lot more predictable, but creates a small split between permissions and hooks/capabilities. This
gap can realistically be narrowed with expressive errors.
4. After plugins are loaded, what order should plugins be run in? Either they run in the configured load order, or we add some sort of machinery to allow plugins to optionally schedule themselves. This is important when a plugin
5. mutates data, such as for the outbound/inbound request filters.