use lorealm_storage::models::UserPersona;

use crate::services::chat::PromptMessage;

pub struct StageCx {
    pub persona: UserPersona,
}

/// One bidirectional stage. The outbound half runs once, may mutate the
/// outgoing prompt, and returns the inbound half carrying any captured state.
pub trait Stage: Send + Sync {
    fn id(&self) -> &str;
    fn outbound(&self, prompt: &mut Vec<PromptMessage>, cx: &StageCx) -> Box<dyn ResponseFilter>;
}

/// Inbound half. Fed model deltas one at a time; returns only text that is
/// safe to emit now. `finish` flushes whatever was held back at stream end.
pub trait ResponseFilter: Send {
    fn push(&mut self, delta: &str) -> String;
    fn finish(&mut self) -> String;
    /// One-shot transform of a detached string (e.g. tool-call output that
    /// bypasses the token stream). Must not touch streaming state.
    fn apply(&self, text: &str) -> String;
}

/// Inbound no-op. Passes deltas straight through. Returned by stages that
/// only touch the outbound prompt (or do nothing this turn).
pub struct Identity;

impl ResponseFilter for Identity {
    fn push(&mut self, delta: &str) -> String {
        delta.to_owned()
    }
    fn finish(&mut self) -> String {
        String::new()
    }
    fn apply(&self, text: &str) -> String {
        text.to_owned()
    }
}
