use bitflags::bitflags;

bitflags! {
    /// The optional command groups a host may expose on top of the stdlib.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct Caps: u8 {
        /// `$[sys]`, `$[toast]`, `$[compose]`: messages and UI effects
        /// drained by the caller after evaluation.
        const EFFECTS = 1 << 0;
        /// `$[emit_event]`: realm events on a granted channel.
        const EVENTS = 1 << 1;
        /// `$[attach]`, `$[search_attachments]`: widget attachment.
        const ATTACHMENTS = 1 << 2;
        /// `$[generate]`, `$[generate_raw]`: a nested completion.
        ///
        /// These block the calling thread on a future.
        const GENERATION = 1 << 3;
    }
}

/// A named weaver host.
///
/// A profile's [`Caps`] are what that host *may* expose, not what any one
/// instance of it does. `$[emit_event]` needs a sink to report into and
/// `$[generate]` needs a live chat handle; without them the builder leaves the
/// command unregistered or registers a declaration that errors on call
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaverProfile {
    /// A user tool body invoked by the model mid-turn.
    ChatTurn,
    /// A `/slash` command body.
    Command,
    /// A realm script agent's body, or a realm action.
    RealmScript,
    /// A scenario's seed script, run once at instantiation.
    RealmSeed,
    /// An agent's `run_condition`, evaluated to decide whether it acts.
    RealmCondition,
    /// A realm's `Director::Script`, evaluated to decide who acts.
    RealmDirector,
    /// A widget's `render`, evaluated during prompt assembly.
    WidgetRender,
}

impl WeaverProfile {
    /// Every profile that runs for real.
    pub const RUNTIME: &'static [Self] = &[
        Self::ChatTurn,
        Self::Command,
        Self::RealmScript,
        Self::RealmSeed,
        Self::RealmCondition,
        Self::RealmDirector,
        Self::WidgetRender,
    ];

    pub const fn caps(self) -> Caps {
        match self {
            // A tool body may attach widgets, speak, emit realm events when
            // the turn belongs to a realm agent, and nest a completion.
            Self::ChatTurn => Caps::EFFECTS
                .union(Caps::EVENTS)
                .union(Caps::ATTACHMENTS)
                .union(Caps::GENERATION),
            // A command speaks to its conversation and may ask the model.
            Self::Command => Caps::EFFECTS.union(Caps::GENERATION),
            // A script agent speaks through events only.
            Self::RealmScript => Caps::EVENTS,
            // A seed script only writes the pool it is handed.
            Self::RealmSeed => Caps::empty(),
            // A condition only reads.
            Self::RealmCondition => Caps::empty(),
            // A director only reads, and answers with a handle.
            Self::RealmDirector => Caps::empty(),
            // A render only reads, and answers with text.
            Self::WidgetRender => Caps::empty(),
        }
    }

    /// Whether this profile writes state.
    pub const fn read_only(self) -> bool {
        matches!(
            self,
            Self::RealmCondition | Self::RealmDirector | Self::WidgetRender
        )
    }
}

/// Everything some runtime profile can call.
pub fn editor_caps() -> Caps {
    WeaverProfile::RUNTIME
        .iter()
        .fold(Caps::empty(), |acc, profile| acc.union(profile.caps()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_editor_is_the_union_of_every_runtime_host() {
        let editor = editor_caps();
        for profile in WeaverProfile::RUNTIME {
            assert!(
                editor.contains(profile.caps()),
                "{profile:?} can call something the editor does not know about"
            );
        }
    }

    #[test]
    fn every_capability_belongs_to_some_host() {
        assert_eq!(
            editor_caps(),
            Caps::all(),
            "a capability exists that no runtime profile uses"
        );
    }
}
