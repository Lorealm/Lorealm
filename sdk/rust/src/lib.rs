//! Write [Lorealm](https://github.com/Lorealm/Lorealm) plugins in Rust.
//!
//! A plugin is a type that implements [`Plugin`], handed to [`export!`]:
//!
//! ```ignore
//! use lorealm_plugin::{Generated, Hook, Plugin, Result, host};
//!
//! struct Counter {
//!     replies: u32,
//! }
//!
//! impl Plugin for Counter {
//!     fn init() -> Result<Self> {
//!         host::register_hook(Hook::AfterGenerate)?;
//!         Ok(Self { replies: 0 })
//!     }
//!
//!     fn after_generate(&mut self, _reply: Generated) -> Result<()> {
//!         self.replies += 1;
//!         host::send_frontend("count", &self.replies)
//!     }
//! }
//!
//! lorealm_plugin::export!(Counter);
//! ```
//!
//! Build it with `cargo build --release --target wasm32-wasip2`, as a
//! `cdylib`.
//!
//! This crate's version is the plugin API version it targets, which the
//! plugin's manifest names in `api`.

/// The plugin API this crate targets. Goes in the manifest's `api`.
pub const API_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The bindings generated from the `lorealm:plugin` WIT package.
pub mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "plugin",
        pub_export_macro: true,
        default_bindings_module: "lorealm_plugin::bindings",
    });
}

pub use bindings::lorealm::plugin::types::*;
pub use serde_json::{self, Value, json};

pub struct LorealmPluginError(String);

pub type Result<T> = core::result::Result<T, LorealmPluginError>;

impl LorealmPluginError {
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl<E: core::fmt::Display> From<E> for LorealmPluginError {
    fn from(error: E) -> Self {
        Self(error.to_string())
    }
}

impl From<LorealmPluginError> for String {
    fn from(error: LorealmPluginError) -> Self {
        error.0
    }
}

impl core::fmt::Debug for LorealmPluginError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Various functions provided by the host server. Many functions are gated
/// behind permissions.
///
/// Read more about host functions here: https://docs.lorealm.app/plugins/host-functions
pub mod host {
    use serde::{Serialize, de::DeserializeOwned};

    use crate::Result;
    use crate::bindings::lorealm::plugin::host as raw;

    pub use raw::{
        active_persona, delete_data, generate, http_fetch, load_data, log, messages, persist_data,
        register_command, register_hook, register_processor, register_source_node, subscribe,
        unsubscribe,
    };

    /// The user's settings, with defaults applied. `T` is a struct with a
    /// field per input of `settings.wtn`, or a [`Value`](crate::Value).
    pub fn settings<T: DeserializeOwned>() -> Result<T> {
        Ok(serde_json::from_str(&raw::settings()?)?)
    }

    /// Sends `payload` to the plugin's own frontend, where
    /// `lorealm.on(name, listener)` receives it.
    ///
    /// `payload` accepts any object implementing [`serde::Serialize`]
    pub fn send_frontend(name: &str, payload: &impl Serialize) -> Result<()> {
        Ok(raw::send_frontend(name, &serde_json::to_string(payload)?)?)
    }

    /// Emits `payload` under `topic`, which is `handle.name` with `name`
    /// listed in the manifest's `[provides] events`.
    pub fn emit(topic: &str, payload: &impl Serialize) -> Result<()> {
        Ok(raw::emit(topic, &serde_json::to_string(payload)?)?)
    }
}

/// A Lorealm Plugin.
///
/// `init()` is mandatory. It tells the server whether the plugin sucessfully
/// loaded. Any initial setup goes in here.
///
/// Hooks are optional and run in Load Order. This means any data you recieve
/// may already have gone through multiple plugins depending on the configured
/// load order.
pub trait Plugin: Sized + Send + 'static {
    /// Called once after the plugin is mounted. Register hooks, commands,
    /// processors and source nodes here, with [`host`].
    fn init() -> Result<Self>;

    /// Fired after all Lorealm's internal prompt building has resolved.
    ///
    /// The messages returned from this function will be the new prompt.
    /// This essentially means that if you return an empty array, the prompt
    /// will be empty when it reaches the model.
    fn prompt_build(&mut self, prompt: PromptAssembly) -> Result<Vec<Message>> {
        unhandled("prompt-build", prompt)
    }

    /// Fired whenever a Weaver Command you declared was executed through a
    /// Weaver script.
    ///
    /// Commands bundle an `EvalContext` object which you can use to access
    /// internal Weaver state.
    ///
    /// Returns an optional JSON value.
    fn command(&mut self, call: CommandCall<'_>) -> Result<Option<Value>> {
        unhandled("command", call)
    }

    /// Fired whenever a Weaver Processor you declared was executed through
    /// a Weaver script.
    ///
    /// Processors are functions that take pure data, and return pure data.
    /// If you need access to internal Weaver state, use a Command instead.
    ///
    /// Returns a JSON value
    fn processor(&mut self, call: ProcessorCall) -> Result<Value> {
        unhandled("processor", call)
    }

    /// Fired whenever a Source Node you declared was requested through
    /// Lorealm's internal prompt builder.
    ///
    /// This returns a list of context blocks which are later flattened
    /// top-to-bottom into the final plain-text prompt. Weaver evaluation
    /// happens later in the process, so any variables, processors, etc. you
    /// use will be evaluated.
    ///
    /// ## Performance Note
    /// This hook should resolve quickly. Refrain from making API calls
    /// or requesting LLM completions here.
    ///
    /// If you plan to use LLM completions
    /// for, say, a summary plugin, you should use another hook such as
    /// `after-generate` and run a non-blocking completion so you don't
    /// hinder the user's completion.
    fn source(&mut self, call: SourceCall) -> Result<Vec<Block>> {
        unhandled("source", call)
    }

    /// Fired when the frontend portion of this plugin sends a message.
    ///
    /// The value you return will be sent back to the frontend.
    fn frontend(&mut self, call: FrontendCall) -> Result<Value> {
        unhandled("frontend", call)
    }

    /// An event the plugin subscribed to with [`host::subscribe`].
    fn custom(&mut self, event: CustomEvent) -> Result<()> {
        unhandled("custom", event)
    }

    /// Fired after generation, just after a reply was saved.
    ///
    /// This is the optimal place to retrieve any prompt-analysis data.
    fn after_generate(&mut self, reply: Generated) -> Result<()> {
        unhandled("after-generate", reply)
    }

    /// Fired after a message was generated, just before it's about to be
    /// saved.
    ///
    /// The returned string will overwrite the message entirely.
    fn message_finalize(&mut self, reply: Finalizing) -> Result<String> {
        unhandled("message-finalize", reply)
    }

    /// Fires when the user sent a message.
    ///
    /// The plugin may allow, block or modify the message.
    ///
    /// Returns the verdict.
    fn user_message(&mut self, input: UserInput) -> Result<Verdict> {
        unhandled("user-message", input)
    }

    /// Fires when lorebooks are evaluated, before prompt construction.
    ///
    /// Useful for when you want to make debugging tools or run filters
    /// over the lorebook entries.
    ///
    /// Returns the new lorebook entries to be marked as active.
    fn lorebook_activate(&mut self, activation: LorebookActivation) -> Result<Vec<LorebookEntry>> {
        unhandled("lorebook-activate", activation)
    }

    /// Fires when an LLM model calls a tool.
    ///
    /// The plugin can allow, block or modify the tool call. Note that this
    /// function does *not* give you access to the tool's logic, only the
    /// input parameters. Modify parameters with care.
    ///
    /// Returns the verdict.
    fn tool_call(&mut self, call: ToolInvocation) -> Result<Verdict> {
        unhandled("tool-call", call)
    }

    /// Fires when a tool returned a result.
    ///
    /// Use this when you want to edit the result of certain tools
    /// or apply wide filters to result content.
    ///
    /// Returns the (new) result of the given tool.
    fn tool_result(&mut self, output: ToolOutput) -> Result<Value> {
        unhandled("tool-result", output)
    }

    /// Fired when a Realm turn is starting.
    ///
    /// Nothing waits for this function to complete.
    fn turn_start(&mut self, turn: TurnStarted) -> Result<()> {
        unhandled("turn-start", turn)
    }

    // Fired when a Realm turn ended.
    fn turn_end(&mut self, turn: TurnEnded) -> Result<()> {
        unhandled("turn-end", turn)
    }

    /// Ran after [`init`] when the plugin was last run with a different version.
    ///
    /// Useful when migrating user data or other migration-like operations.
    fn upgrade(&mut self, from: Option<String>) -> Result<()> {
        unhandled("upgrade", from)
    }

    /// Fired when the user changed a setting belonging to this plugin.
    fn settings_changed(&mut self) -> Result<()> {
        unhandled("settings-changed", ())
    }

    /// Fired when the plugin is about to be unloaded.
    ///
    /// NOTE: this does **not** always mean the server is shutting down,
    /// but rather, whenever the plugin is being unloaded (f.ex. after user
    /// inactivity)
    fn shutdown(&mut self) -> Result<()> {
        unhandled("shutdown", ())
    }
}

fn unhandled<I, T>(event: &str, _input: I) -> Result<T> {
    Err(format!("the plugin doesn't handle `{event}` events").into())
}

/// Makes `$plugin`, a type implementing [`Plugin`], what the server loads.
/// Used once, in the crate's root.
#[macro_export]
macro_rules! export {
    ($plugin:ty) => {
        const _: () = {
            static SLOT: $crate::__private::Slot<$plugin> = $crate::__private::Slot::new();

            struct Export;

            impl $crate::bindings::Guest for Export {
                fn init() -> ::core::result::Result<(), ::std::string::String> {
                    SLOT.init()
                }

                fn handle(
                    event: $crate::bindings::Event<'_>,
                ) -> ::core::result::Result<$crate::bindings::Response, ::std::string::String>
                {
                    SLOT.handle(event)
                }
            }

    $crate::bindings::export!(Export with_types_in $crate::bindings);
        };
    };
}

/// What [`export!`] expands to. Not part of the API.
#[doc(hidden)]
pub mod __private {
    use std::sync::Mutex;

    use super::*;

    /// Holds the plugin between calls.
    pub struct Slot<P>(Mutex<Option<P>>);

    impl<P: Plugin> Slot<P> {
        #[allow(clippy::new_without_default)]
        pub const fn new() -> Self {
            Self(Mutex::new(None))
        }

        pub fn init(&self) -> core::result::Result<(), String> {
            let plugin = P::init()?;
            *self.0.try_lock().map_err(|_| "the plugin is busy")? = Some(plugin);
            Ok(())
        }

        pub fn handle(&self, event: Event<'_>) -> core::result::Result<Response, String> {
            let mut slot = self.0.try_lock().map_err(|_| "the plugin is busy")?;
            let plugin = slot.as_mut().ok_or("the plugin wasn't initialised")?;
            Ok(dispatch(plugin, event)?)
        }
    }

    fn dispatch<P: Plugin>(plugin: &mut P, event: Event<'_>) -> Result<Response> {
        Ok(match event {
            Event::PromptBuild(prompt) => Response::PromptBuild(plugin.prompt_build(prompt)?),
            Event::Command(call) => {
                Response::Command(plugin.command(call)?.map(|value| value.to_string()))
            }
            Event::Processor(call) => Response::Processor(plugin.processor(call)?.to_string()),
            Event::Source(call) => Response::Source(plugin.source(call)?),
            Event::Frontend(call) => Response::Frontend(plugin.frontend(call)?.to_string()),
            Event::Custom(event) => {
                plugin.custom(event)?;
                Response::Custom
            }
            Event::AfterGenerate(reply) => {
                plugin.after_generate(reply)?;
                Response::AfterGenerate
            }
            Event::MessageFinalize(reply) => {
                Response::MessageFinalize(plugin.message_finalize(reply)?)
            }
            Event::UserMessage(input) => Response::UserMessage(plugin.user_message(input)?),
            Event::LorebookActivate(activation) => {
                Response::LorebookActivate(plugin.lorebook_activate(activation)?)
            }
            Event::ToolCall(call) => Response::ToolCall(plugin.tool_call(call)?),
            Event::ToolResult(output) => {
                Response::ToolResult(plugin.tool_result(output)?.to_string())
            }
            Event::TurnStart(turn) => {
                plugin.turn_start(turn)?;
                Response::TurnStart
            }
            Event::TurnEnd(turn) => {
                plugin.turn_end(turn)?;
                Response::TurnEnd
            }
            Event::Upgrade(from) => {
                plugin.upgrade(from)?;
                Response::Upgrade
            }
            Event::SettingsChanged => {
                plugin.settings_changed()?;
                Response::SettingsChanged
            }
            Event::Shutdown => {
                plugin.shutdown()?;
                Response::Shutdown
            }
        })
    }
}
