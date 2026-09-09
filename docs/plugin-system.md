# Plugin System

A living document to help us keep track of the plugin system implementation.

## Updates
| Date DD/MM/YY | Summary                         | AI-Assisted? |
|---------------|---------------------------------|--------------|
| 09/09/26      | Created the initial ground plan | ❌            |
|               |                                 |              |

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

## Open Questions
- The frontend in this project is the official one, but is only one of potentially many. Clients will naturally have different ways of handling plugins. How do we want to handle these cases semantically? The two main options are:
  1. We rely on authors documenting their plugins targeting non-official clients
  2. We implement an optional `client_id` field in the manifest which accepts an arbitrary string. Clients can use these to broadcast support capabilites.
- How do we want to handle plugin installation? For max "user-friendliness", the ideal way is to paste in a GitHub link and let the server build the .wasm file from source. The other option is only accepting pre-built .wasm files,
but this creates a significant technical overhead on plugin authors.
- When plugins want to register a capability, we have options. The first option is to be completely manifest driven. This keeps everything: permissions and hooks, in one place. It is a lot more finnicky to get around, though.
The other option involves providing an interface through the PDK that lets authors just do: `register_processor([...])`. This is a lot more predictable, but creates a small split between permissions and hooks/capabilities. This
gap can realistically be narrowed with expressive errors.