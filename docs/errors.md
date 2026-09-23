# Error contract

> AI Disclosure: This document was authored with the help of Claude.

Every failing API response — HTTP or SSE — carries the same JSON body:

```json
{
  "error_code": "state.no_active_connection_profile",
  "message": "No connection profile is selected.",
  "details": { "resource": "connection_profile" }
}
```

| Field | Notes |
| --- | --- |
| `error_code` | Stable `domain.reason` identifier. **This is the API.** Clients match on it. |
| `message` | A sentence to show a user. Never contains ids or internal error text. |
| `details` | Optional. Points at the resource the error concerns, so clients can act without parsing `message`. |

## Rules

- **The code is stable.** Once released, a code's meaning does not change. New
  failure modes get new codes.
- **`message` is for humans, `error_code` is for programs.** Never parse
  `message`; it is free to be reworded at any time.
- **`internal` is the only 500 code.** Every server-side failure collapses to
  `error_code: "internal"` with a fixed message; the real detail stays in the
  server log. Other 5xx (`502`) are upstream failures and keep their own code.
- **`404` means gone *or* not yours.** Ownership failures deliberately return
  the same code and status as a missing resource, so the API never reveals that
  a resource exists under another account.
- **The server never names a route.** `state.no_active_connection_profile` is a
  fact about server state. Turning that into "open the Connections tab" is the
  client's decision — a CLI would print something else from the same code.

## Handling codes

Match exactly on the codes you care about, and fall back on the namespace:

```ts
import { extractError, errorDomain } from '@lorealm/ui';

const err = extractError(e);
switch (err.code) {
  case 'state.no_active_connection_profile':
    return promptToPickConnection();
  case 'connection.secret_corrupted':
    return openSecret(err.details?.id);
}
if (errorDomain(err.code) === 'realm') return showRealmToast(err.message);
```

## Resource kinds

`details.resource` is one of: `character`, `character_version`, `greeting`,
`conversation`, `message`, `example_message`, `context_template`, `persona`,
`connection_profile`, `secret`, `user_tool`, `user_widget`, `lorebook`, `realm`,
`realm_agent`, `realm_version`, `scenario`, `tokenizer`, `lorebook_entry`,
`template_node`.

`details.id` is a snowflake string, present when the error concerns one specific
instance.

## Codes

Generated from the source — regenerate with:

```
cargo test -p lorealm_server --bin lorealm_server dump_error_table -- --ignored --nocapture
```

Messages containing quoted values are examples; the interpolated part varies.

| Code | Status | Message |
| --- | --- | --- |
| `auth.csrf_invalid` | 401 | Invalid or missing CSRF token. |
| `auth.email_taken` | 409 | That email is already registered. |
| `auth.invalid_credentials` | 401 | Invalid credentials. |
| `auth.missing_token` | 401 | You are not signed in. |
| `auth.session_invalid` | 401 | Your session has expired. Please sign in again. |
| `auth.username_taken` | 409 | That username is already taken. |
| `character.not_found` | 404 | That character no longer exists. |
| `character.unsupported_format` | 400 | Unsupported card format 'chara_card_v1' — expected chara_card_v2. |
| `character.unsupported_media_type` | 415 | That image format isn't supported. |
| `chat.not_found` | 404 | That conversation no longer exists. |
| `chat.provider_failed` | 502 | Could not reach the model provider: timed out. |
| `chat.realm_conversation` | 409 | Realm conversations advance through their turn endpoints. |
| `config.unknown_key` | 400 | Unknown configuration key. |
| `connection.invalid_profile` | 400 | Model is required. |
| `connection.not_found` | 404 | That connection profile no longer exists. |
| `connection.secret_corrupted` | 409 | This API key could not be decrypted and needs to be entered again. |
| `context.no_active_character` | 409 | No character is selected. |
| `context.not_found` | 404 | That lorebook no longer exists. |
| `context.resolver_not_found` | 400 | Unknown resolver 'char' in this template. |
| `context.unknown_source_node` | 400 | This template references an unknown node 'core:persona'. |
| `conversation.message_not_found` | 404 | That message is no longer in this conversation. |
| `conversation.not_found` | 404 | That conversation no longer exists. |
| `lorebook.not_found` | 404 | That lorebook no longer exists. |
| `permission.denied` | 403 | You are not allowed to perform this action. |
| `persona.not_found` | 404 | That persona no longer exists. |
| `persona.unsupported_media_type` | 415 | That image format isn't supported. |
| `plugin.call_failed` | 422 | The plugin couldn't complete that request. Its log says why. |
| `plugin.cannot_enable` | 409 | This plugin can't be enabled: it requires `lib` ^1.4, which is not loaded. |
| `plugin.invalid` | 400 | That plugin could not be loaded. |
| `plugin.invalid_order` | 400 | The order must list each enabled plugin exactly once. |
| `plugin.not_found` | 404 | That plugin is not installed. |
| `plugin.required` | 409 | Disable the plugins that require this one first: `hello`, `other`. |
| `realm.action_failed` | 400 | Action failed: roll failed. |
| `realm.director_failed` | 400 | The realm could not decide who acts next: unknown handle. |
| `realm.import_in_use` | 409 | This lorebook is still used by agent "Narrator" — unbind it there first. |
| `realm.invalid_agent` | 400 | Invalid agent: no name. |
| `realm.invalid_scenario` | 400 | Invalid scenario: no opening. |
| `realm.invalid_turn` | 400 | Invalid turn: already ended. |
| `realm.malformed_agent` | 400 | This agent is misconfigured: no model. |
| `realm.no_agents` | 400 | This realm has no agents yet. |
| `realm.no_runnable_agent` | 409 | No agent could run this turn — every run condition was false. |
| `realm.not_found` | 404 | That realm no longer exists. |
| `realm.not_imported` | 409 | This lorebook isn't imported into the realm — import it first. |
| `realm.not_realm_conversation` | 400 | This conversation was not started from a realm. |
| `realm.not_your_turn` | 409 | It isn't this agent's turn. |
| `realm.tool_not_bound` | 400 | This action isn't available to the agent. |
| `state.no_active_connection_profile` | 409 | No connection profile is selected. |
| `state.no_active_persona` | 409 | No persona is selected. |
| `state.no_active_template` | 409 | No context template is selected. |
| `state.no_active_tokenizer` | 409 | No tokenizer is selected. |
| `state.not_found` | 404 | That persona no longer exists. |
| `template.circular_reference` | 400 | That move would create a circular reference. |
| `template.duplicate_source_node` | 409 | Node 'core:persona' is already used in this template. |
| `template.index_out_of_bounds` | 400 | Position 9 is out of range (max 3). |
| `template.invalid_parent` | 400 | Nodes can only be nested inside a group. |
| `template.invalid_path` | 400 | That template path is not valid. |
| `template.node_not_found` | 404 | That template node no longer exists. |
| `tokenizer.fetch_failed` | 502 | Failed to download tokenizer: connection reset. |
| `tokenizer.forbidden` | 403 | This tokenizer can't be modified. |
| `tokenizer.invalid_data` | 400 | The imported tokenizer was invalid. |
| `tokenizer.invalid_source` | 400 | 'ftp://x' is not a valid tokenizer source. |
| `tokenizer.not_found` | 404 | That tokenizer no longer exists. |
| `tokenizer.tokenize_failed` | 400 | Failed to tokenize: overflow. |
| `tool.invalid_body` | 400 | Tool failed to compile: unexpected token. |
| `tool.invalid_name` | 400 | Invalid tool name: must be snake_case. |
| `tool.not_found` | 404 | That tool no longer exists. |
| `widget.invalid` | 400 | Invalid widget: missing layout. |
| `widget.not_found` | 404 | That widget no longer exists. |

### Not covered by the generator

These wrap foreign error types or are raised at the route layer, so they are not
in `contract_surface()`:

| Code | Status | Meaning |
| --- | --- | --- |
| `internal` | 500 | Any server-side failure. Detail is in the server log only. |
| `auth.registration_disabled` | 403 | Registration is turned off in server config. |
| `character.invalid_card` | 400 | Character card failed to parse or deserialize. |
| `character.malformed_upload` | 400 | Multipart upload could not be read. |
| `character.no_files` | 400 | Import request contained no files. |
| `context.evaluation_failed` | 400 | ContextWeaver could not build the context. |
| `context.plugin_failed` | 400 | A ContextWeaver plugin failed. |
| `lorebook.evaluation_failed` | 400 | ContextWeaver could not evaluate the lorebook. |
| `lorebook.invalid_yaml` | 400 | Uploaded lorebook YAML failed to parse. |
| `tokenizer.empty_file` | 400 | Tokenizer upload was empty. |

## Adding a code

1. Add the variant to the domain enum in `backend/server/src/errors/`.
2. Give it a code, status, message and (if it concerns a resource) details in
   that file's `impl DomainError`.
3. Add it to `contract_surface()` in `errors/contract.rs` if it is client-facing.
4. Add it to `ErrorCode` in `frontend/packages/ui/src/types/error.ts`.
5. Regenerate the table above.

`cargo test -p lorealm_server --bin lorealm_server errors::` enforces code shape,
namespacing, message hygiene and that every 404 carries a resource pointer.
