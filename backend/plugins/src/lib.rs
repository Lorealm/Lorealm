mod host;
mod log;
mod manifest;
mod resolve;
mod stage;

pub use host::{
    CommandContext, HostServices, LoadedPlugin, PluginError, PluginHost, PluginPackage,
    Registrations,
    lorealm::plugin::types::{
        Block, CommandSignature, Hook, Level, Message, Param, Persona, ProcessorSignature, Role,
        SourceNode, ValueType,
    },
};
pub use log::{LogEntry, LogSource};
pub use manifest::{
    Build, Compatibility, Frontend, FrontendSlot, Manifest, ManifestError, PluginInfo, SlotMode,
    Structure,
};
pub use resolve::{Resolution, resolve};
pub use semver::Version;
pub use stage::{StageError, stage};
