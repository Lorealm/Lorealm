# lorealm-plugin

Write [Lorealm](https://github.com/Lorealm/Lorealm) plugins in Rust.

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
lorealm-plugin = "0.1.0"
```

```rust
use lorealm_plugin::{Generated, Hook, Plugin, Result, host};

struct Counter {
    replies: u32,
}

impl Plugin for Counter {
    fn init() -> Result<Self> {
        host::register_hook(Hook::AfterGenerate)?;
        Ok(Self { replies: 0 })
    }

    fn after_generate(&mut self, _reply: Generated) -> Result<()> {
        self.replies += 1;
        host::send_frontend("count", &self.replies)
    }
}

lorealm_plugin::export!(Counter);
```

Build it with `cargo build --release --target wasm32-wasip2` (`rustup target add wasm32-wasip2` once).

## Versions

This crate's version is the plugin API version it targets. A server only runs plugins built for its own API version, so the plugin's `manifest.toml` names the same one:

```toml
[plugin]
api = "0.1.0"
```

The API isn't stable before 1.0: each new version needs a rebuild against the matching release of this crate, and may need changes.

## What's in it

- `Plugin`: one method per hook and per kind of call. Implement the ones `init` registers.
- `host`: what the server offers, such as `register_hook`, `settings`, `messages`, `generate`, `http_fetch` and `persist_data`.
- `Result` and `Error`: `?` works on any error that can be displayed, and the message reaches the user or the plugin's log.
- `bindings`: the generated bindings as they are, for anything the rest doesn't cover.

`plugins/hypebot` and `plugins/tts` in the Lorealm repository are complete examples.

For the plugin's frontend, [`@lorealm/plugin`](../js) has the types and docs for `lorealm.call` and `lorealm.on`, the other end of `Plugin::frontend` and `host::send_frontend`.
