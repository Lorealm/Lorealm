use base64::{Engine, engine::general_purpose::STANDARD};
use lorealm_plugin::host::{http_fetch, settings};
use lorealm_plugin::{FrontendCall, HttpRequest, Plugin, Result, Value, json, serde_json};

#[derive(Default)]
struct Tts {
    /// The last reply read aloud on its own, so it isn't read twice.
    autoplayed: Option<String>,
}

impl Plugin for Tts {
    fn init() -> Result<Self> {
        Ok(Self::default())
    }

    fn frontend(&mut self, call: FrontendCall) -> Result<Value> {
        let args: Value = serde_json::from_str(&call.args)?;
        Ok(match call.name.as_str() {
            "autoplay" => json!(self.autoplay(args["id"].as_str().unwrap_or_default())?),
            "speak" => speak(args["text"].as_str().unwrap_or_default())?,
            other => return Err(format!("unknown call `{other}`").into()),
        })
    }
}

impl Tts {
    /// Whether to read the reply `id` aloud on its own
    fn autoplay(&mut self, id: &str) -> Result<bool> {
        let settings: Value = settings()?;
        if settings["autoplay"] != true || self.autoplayed.as_deref() == Some(id) {
            return Ok(false);
        }
        self.autoplayed = Some(id.to_owned());
        Ok(true)
    }
}

/// `{ "audio": <data URL> }`, or `{ "error": <what the user can do> }`.
fn speak(text: &str) -> Result<Value> {
    let settings: Value = settings()?;
    let text = plain(text);
    if text.is_empty() {
        return Ok(json!({ "error": "There's nothing to read." }));
    }

    let (name, key, request) = match settings["provider"].as_str() {
        Some("elevenlabs") => (
            "ElevenLabs",
            "elevenlabs_key",
            HttpRequest {
                method: "POST".into(),
                url: format!(
                    "https://api.elevenlabs.io/v1/text-to-speech/{}?output_format=mp3_44100_128",
                    settings["elevenlabs_voice"].as_str().unwrap_or_default()
                ),
                headers: vec![
                    ("xi-api-key".into(), "{secret:elevenlabs_key}".into()),
                    ("content-type".into(), "application/json".into()),
                ],
                body: Some(
                    json!({ "text": text, "model_id": settings["elevenlabs_model"] })
                        .to_string()
                        .into_bytes(),
                ),
            },
        ),
        _ => (
            "OpenRouter",
            "openrouter_key",
            HttpRequest {
                method: "POST".into(),
                url: "https://openrouter.ai/api/v1/audio/speech".into(),
                headers: vec![
                    (
                        "authorization".into(),
                        "Bearer {secret:openrouter_key}".into(),
                    ),
                    ("content-type".into(), "application/json".into()),
                ],
                body: Some(
                    json!({
                        "model": settings["openrouter_model"],
                        "voice": settings["openrouter_voice"],
                        "input": text,
                        "response_format": "mp3",
                    })
                    .to_string()
                    .into_bytes(),
                ),
            },
        ),
    };
    if settings[key].is_null() {
        return Ok(json!({
            "error": format!("Pick your {name} key under Settings → You → Plugins.")
        }));
    }

    let response = http_fetch(&request)?;
    if response.status != 200 {
        let reason = String::from_utf8_lossy(&response.body);
        return Ok(json!({ "error": format!("{name} answered {}: {reason}", response.status) }));
    }
    let audio = STANDARD.encode(&response.body);
    Ok(json!({ "audio": format!("data:audio/mpeg;base64,{audio}") }))
}

/// `text` without the markdown that would be read out: emphasis, headings,
/// code marks, and link addresses.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    // `[label](address)` reads as `label`.
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let link = after
            .split_once("](")
            .and_then(|(label, tail)| Some((label, tail.split_once(')')?.1)));
        match link {
            Some((label, tail)) if !label.contains(['\n', '[', ']']) => {
                out.push_str(label);
                rest = tail;
            }
            _ => {
                out.push('[');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out.chars()
        .filter(|c| !"*_#`~>".contains(*c))
        .collect::<String>()
        .trim()
        .to_owned()
}

lorealm_plugin::export!(Tts);

#[cfg(test)]
mod tests {
    use super::plain;

    #[test]
    fn drops_markdown() {
        assert_eq!(plain("*She waves.* **Hi!**"), "She waves. Hi!");
        assert_eq!(
            plain("# Title\nSee [the docs](https://x.y/z)."),
            "Title\nSee the docs."
        );
        assert_eq!(plain("An [aside] stays."), "An [aside] stays.");
        assert_eq!(plain("An [aside], a [link](x)."), "An [aside], a link.");
        assert_eq!(plain("`code` and ~struck~"), "code and struck");
    }
}
