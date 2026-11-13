use crate::services::{
    chat::PromptMessage,
    schedule::{Identity, ResponseFilter, Stage, StageCx, filters::builtin::Replace},
};

/// Aliases the user's name on the wire: storage truth -> alias outbound,
/// alias -> storage truth inbound. The model only ever sees the alias;
/// everything persisted stays in the real namespace.
pub struct PrivacyStage;

impl Stage for PrivacyStage {
    fn _id(&self) -> &str {
        "privacy"
    }

    fn outbound(&self, prompt: &mut Vec<PromptMessage>, cx: &StageCx) -> Box<dyn ResponseFilter> {
        let Some(alias) = cx.persona.pseudonym.as_deref() else {
            return Box::new(Identity);
        };

        let display = cx
            .persona
            .display_name
            .as_deref()
            .unwrap_or(&cx.persona.name);

        if alias == display {
            return Box::new(Identity);
        }

        // Outbound: whole prompt in hand, so a plain full-string swap.
        for msg in prompt.iter_mut() {
            msg.content = replace_word(&msg.content, display, alias);
        }

        // Inbound: undo it on the streamed response, boundary-safe across deltas.
        Box::new(Replace::new(alias, display.to_owned()))
    }
}

/// One-shot boundary-aware replace. Same matcher as the streaming filter,
/// so outbound and inbound can never disagree on what counts as a match.
pub fn replace_word(haystack: &str, from: &str, to: &str) -> String {
    let mut r = Replace::new(from, to);
    let mut out = r.push(haystack);
    out.push_str(&r.drain());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(from: &str, to: &str, deltas: &[&str]) -> String {
        let mut r = Replace::new(from, to);
        let mut out: String = deltas.iter().map(|d| r.push(d)).collect();
        out.push_str(&r.drain());
        out
    }

    #[test]
    fn split_match() {
        assert_eq!(run("Alex", "Bob", &["Al", "ex"]), "Bob");
    }
    #[test]
    fn standalone() {
        assert_eq!(run("Alex", "Bob", &["Hi Alex!"]), "Hi Bob!");
    }
    #[test]
    fn split_boundaries() {
        assert_eq!(run("Alex", "Bob", &["Hi Al", "ex!"]), "Hi Bob!");
    }
    #[test]
    fn trailing_on_finish() {
        assert_eq!(run("Alex", "Bob", &["Alex"]), "Bob");
    }
    #[test]
    fn multiple() {
        assert_eq!(run("Alex", "Bob", &["Alex and Alex"]), "Bob and Bob");
    }
    #[test]
    fn not_inside_word() {
        assert_eq!(run("Alex", "Bob", &["Alexander here"]), "Alexander here");
    }
    #[test]
    fn not_inside_word_split() {
        assert_eq!(run("Alex", "Bob", &["Al", "ex", "ander"]), "Alexander");
    }
    #[test]
    fn outbound_symmetry() {
        assert_eq!(
            replace_word("Bobby met Bob", "Bob", "Alex"),
            "Bobby met Alex"
        );
    }
}
