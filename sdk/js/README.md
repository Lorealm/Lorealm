# @lorealm/plugin

Types for the frontend of a [Lorealm](https://github.com/Lorealm/Lorealm) plugin. For the backend, see [`lorealm-plugin`](../rust).

```sh
npm install --save-dev @lorealm/plugin
```

Types only: the client provides the API at runtime.

## Slots

List the slots your plugin fills in `manifest.toml`:

```toml
[[frontend.slots]]
slot = "inspector"
entry = "panel.html"  # relative to `frontend/`
title = "Reply count"
```

| `slot`      | Where                                                       | Context            |
| ----------- | ----------------------------------------------------------- | ------------------ |
| `inspector` | A panel in the sidebar                                      | `InspectorContext` |
| `composer`  | Above the input box                                         | `ComposerContext`  |
| `message`   | Under the latest message, or each one with `mode = "every"` | `MessageContext`   |

The context holds the open conversation and, for `message` slots, the message.

## Frames

By default, a slot is a sandboxed frame showing your HTML file, with `lorealm` defined globally:

```html
<p id="count">No replies yet</p>
<script>
	const label = document.getElementById('count');
	lorealm.call('count').then((n) => (label.textContent = `${n} replies`));
	lorealm.on('count', (n) => (label.textContent = `${n} replies`));
</script>
```

In TypeScript:

```ts
/// <reference types="@lorealm/plugin/frame" />
import type { LorealmApi, MessageContext } from '@lorealm/plugin';

// Optional: narrow the context to your slot's.
const api = lorealm as LorealmApi<MessageContext>;
```

A frame:

- Can't load anything from outside its HTML file, so inline everything. Fetch data through `lorealm.call`.
- Has no `localStorage` or cookies. Keep state in the backend.
- Gets the client's CSS variables (like `--color-text-primary`) and fonts, kept in sync with the theme.
- Is as tall as its `<body>`, except in the inspector.

To bundle a frame into one file with Vite, use [`vite-plugin-singlefile`](https://www.npmjs.com/package/vite-plugin-singlefile) and let the server build it:

```toml
[build]
frontend = ["npm install", "npx vite build"]

[frontend]
path = "dist"
```

## Unsafe slots

With `unsafe = true`, the entry is an ES module that runs in the client itself. It needs the `ui:unsafe` permission, and users have to allow unsafe plugins. Bundle it into one file.

```ts
import type { InspectorContext, Mount } from '@lorealm/plugin';

export const mount: Mount<InspectorContext> = (element, lorealm) => {
	const label = element.appendChild(document.createElement('p'));
	lorealm.on('count', (n) => (label.textContent = `${n} replies`));

	const timer = setInterval(() => lorealm.call('ping'), 60_000);
	return () => clearInterval(timer); // runs when the slot is removed
};
```

## The backend

`lorealm.call` reaches `Plugin::frontend`, and `host::send_frontend` reaches `lorealm.on`:

```rust
impl Plugin for Counter {
    fn frontend(&mut self, call: FrontendCall) -> Result<Value> {
        match call.name.as_str() {
            "count" => Ok(json!(self.replies)),
            other => Err(format!("unknown call `{other}`").into()),
        }
    }

    fn after_generate(&mut self, _reply: Generated) -> Result<()> {
        self.replies += 1;
        host::send_frontend("count", &self.replies)
    }
}
```

- A frame only gets events sent after it loads, so fetch the current state with `call` first.
- Pass `{ blocking: true }` when the user is waiting on the result. The chat stays locked until the call returns.

## Versions

The package version is the plugin API version, matching the Rust crate and the `api` in your manifest. Expect breaking changes before 1.0.
