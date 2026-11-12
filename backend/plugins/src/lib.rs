mod host;
mod log;
mod resolve;
mod stage;

pub use host::{
    API_VERSION, CommandContext, HostServices, LoadedPlugin, PluginError, PluginHost,
    PluginPackage, Registrations, check_api,
    lorealm::plugin::types::{
        Block, ChatMessage, CommandSignature, Finalizing, Generated, Hook, HttpRequest,
        HttpResponse, Level, LorebookActivation, LorebookEntry, Message, MessageRange, Param,
        Persona, ProcessorSignature, PromptAssembly, Role, SourceNode, ToolInvocation, ToolOutput,
        TurnEnded, TurnStarted, UserInput, ValueType, Verdict,
    },
};
pub use log::{LogEntry, LogSource};
pub use lorealm_plugin_manifest::{
    Build, Compatibility, Definitions, Frontend, FrontendSlot, MANIFEST_FILE, Manifest,
    ManifestError, Network, PluginInfo, Provides, SlotMode, Structure,
};
pub use resolve::{Resolution, resolve};
pub use semver::Version;
pub use stage::{StageError, stage};
