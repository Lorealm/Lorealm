//! Shared pieces of the load-testing harness.
//!
//! - `mock-llm`: an OpenAI-compatible endpoint with tunable latency
//! - `loadgen`: seeds users, characters and conversations, then drives chat

pub mod fdlimit;
pub mod rng;
pub mod stats;

/// Marker the mock puts in its first chunk
pub const TAG_TTFT: &str = "[[mock ttft_us=";
/// Marker the mock puts in its last content chunk
pub const TAG_STREAM: &str = "[[mock stream_us=";

/// Reads the number following `tag` in `text`, if the tag is present.
pub fn read_tag(text: &str, tag: &str) -> Option<u64> {
    let start = text.find(tag)? + tag.len();
    let rest = &text[start..];
    let end = rest.find("]]")?;
    rest[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_round_trip() {
        let text = format!("{TAG_TTFT}1234]] hello world {TAG_STREAM}99]]");
        assert_eq!(read_tag(&text, TAG_TTFT), Some(1234));
        assert_eq!(read_tag(&text, TAG_STREAM), Some(99));
        assert_eq!(read_tag("no tags here", TAG_TTFT), None);
    }
}
