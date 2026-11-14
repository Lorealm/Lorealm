# Lorealm
Lorealm is a self-hosted frontend for LLM chat and roleplay where the logic travels with the content. A character, lorebook or whole world can carry its own scripts, state and tools, and whoever imports it gets a working system with nothing to install or configure.

<img width="1990" height="1271" alt="Screen Shot 2026-11-14 at 03 08 08(4)" src="https://github.com/user-attachments/assets/e3a628ba-3d88-47e2-822b-6e091e1c01bd" />

Lorealm is built with security and extensibility in mind. It was built with a philosophy that users should be able to express their creativity freely without being limited by what the tool can do.

The result is a very powerful tool that lets you do just that, create anything your heart desires.

You can read more about our vision [here](https://docs.lorealm.app/)

## Features
- **The Weaver Language**: A small programming language designed to let you express yourself. It's what allows us to shoot the creative ceiling into the stratosphere, while keeping the floor right down under your feet.
- **Reimagined Lorebooks**: With the power of Weaver, we have created a new vision for what Lorebooks can be. Instead of being simple keyword-based retrieval, they support evolving state and and a variety of other features. This means that a Lorebook can evolve over the course of a conversation.
- **Realms**: The system that holds everything together. A Realm is essentially a stage, where every actor only sees what it's given. This allows us to express simple 1-on-1 conversations, group chats or even full game systems, within one single system. We highly encourage you to [read more](https://docs.lorealm.app/realms/what_are_realms/) so you can appreciate what this system offers.
- **Security & Privacy**: Lorealm offers a level of security and privacy that few alternatives can match. Your passwords and API keys are protected with modern, best-practice cryptography, and Lorealm is only reachable from your own machine unless you choose otherwise.
- **Extensibility**: A working, documented plugin system from day one. Plugins let you extend Lorealm to your heart's content without giving up security: every plugin runs inside a sandbox and has to ask for each permission it uses.

### Comparison
A quick overview on how Lorealm compares to alternatives. These fields are gauged on "vanilla" installations (except for the Extensibility part).

| Feature | Lorealm | SillyTavern | RisuAI | Agnai |
|---------|---------|-------------|--------|-------|
| Distribution | Everything, including complex scripts and Realm setups | Characters, lorebooks, presets, and regex embedded in cards. Scripts (STscript/Quick Replies) are not distributable. | Cards and Modules bundle lorebooks, regex, Lua/Python triggers, assets and custom HTML/CSS. Public hub (RisuRealm) | Characters with lorebook, sprites and JSON schema. Scenario books are separate. |
| Security | Modern, best-practice cryptography and Plugin sandboxing | Solid server security (scrypt, accounts, CSRF, whitelists, SSO). Extensions and plugins are not sandboxed. | Sandboxed plugins. Self-hosted server is single-user and stores its password in plaintext. | bcrypt/JWT auth, but some outdated crypto. No plugins. |
| Creative Ceiling | Virtually none | High. STscript and extensions are powerful but hard to ship with a character. | Very high. Lua/Python scripting, visual triggers, variables, custom UI. | Medium. Scenario state machines, JSON value tracking, template conditionals. |
| Extensibility | Very High level of extensibility with a public plugin marketplace and security features | Almost unlimited, though has little to no security features | High. Sandboxed JS plugin API plus Lua/Python scripting | None. No plugin system |
| Multi-user / Scale | Very High performance and low memory footprint. Can easily serve hundreds of users when configured for hosting. | Modest performance, higher memory footprint. Can not handle multiple users in parallel | Mostly client-side, single-user server | Multi-tenant, powers agnai.chat |

## Installing
Lorealm runs on Windows, Linux and macOS, or anywhere Docker does. Mobile/Termux is untested.

### Download and run
1. Download the server for your system from the [latest release](https://github.com/Lorealm/Lorealm/releases/latest).
2. Run it and follow the setup questions in the terminal.
3. Open http://127.0.0.1:1551 in your browser and log in.

### Docker
1. Download [`docker-compose.yaml`](docker-compose.yaml) and change the admin password in it.
2. Start it:
   ```sh
   docker compose up -d
   ```
3. Open http://127.0.0.1:1551 in your browser and log in.

The [installation guide](https://docs.lorealm.app/getting-started/installing/) covers both in more detail, along with building from source.

## Contributing
If you want to contribute, we're happy to have you!

- **Found a bug or have an idea?** Open an [issue](https://github.com/Lorealm/Lorealm/issues).
- **Have a question?** Ask on [Discord](https://discord.gg/HGucxzNMhq).
- **Want to write code?** For anything larger than a small fix, open an issue first so we can agree on the approach. The installation guide explains how to [build from source](https://docs.lorealm.app/getting-started/installing/#building-from-source).
- **Want to improve the docs?** They live in [lorealm-docs](https://github.com/Lorealm/lorealm-docs).

Before opening a pull request, run the checks:
```sh
cd backend && cargo ci
cd frontend && make check lint
```

We only have two rules:
- We don't accept vibe-coded contributions.
- All contributions should be in English (excluding translation files).

## License
MIT
