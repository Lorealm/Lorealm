use std::time::Duration;

use lorealm_plugins::{HttpRequest, HttpResponse};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long a response may go without sending anything. There's no limit on
/// its total length, since audio and the like can stream in for a while.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

const MAX_BODY: usize = 16 * 1024 * 1024;

/// Doesn't follow redirects, so a request can't leave the plugin's hosts.
pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .build()
        .expect("plugin HTTP client should build")
}

/// The settings named by `{secret:name}` in `value`.
pub fn placeholders(value: &str) -> Vec<String> {
    value
        .split("{secret:")
        .skip(1)
        .filter_map(|rest| rest.split_once('}'))
        .map(|(name, _)| name.to_owned())
        .collect()
}

/// Whether keys may be sent to `url`: over https, or to this machine.
pub fn may_carry_keys(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|url| {
        url.scheme() == "https"
            || matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
    })
}

/// Names the plugin, such as `Lorealm/0.1.0 (plugin hello@0.1.0)`.
pub fn user_agent(handle: &str, version: &impl std::fmt::Display) -> String {
    format!(
        "Lorealm/{} (plugin {handle}@{version})",
        env!("CARGO_PKG_VERSION")
    )
}

/// Sends `request`, with `user_agent` unless it sets its own.
pub async fn send(
    client: &reqwest::Client,
    request: HttpRequest,
    user_agent: &str,
) -> Result<HttpResponse, String> {
    let method = reqwest::Method::from_bytes(request.method.as_bytes())
        .map_err(|_| format!("`{}` isn't an HTTP method", request.method))?;
    let mut builder = client.request(method, &request.url);
    if !request
        .headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("user-agent"))
    {
        builder = builder.header(reqwest::header::USER_AGENT, user_agent);
    }
    for (name, value) in request.headers {
        builder = builder.header(name, value);
    }
    if let Some(body) = request.body {
        builder = builder.body(body);
    }

    let mut response = builder.send().await.map_err(describe)?;
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            let value = String::from_utf8_lossy(value.as_bytes()).into_owned();
            (name.to_string(), value)
        })
        .collect();
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(describe)? {
        if body.len() + chunk.len() > MAX_BODY {
            return Err(format!("the response is over {} MiB", MAX_BODY >> 20));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}

/// reqwest's message with its causes; its own names only the step that
/// failed, such as "error decoding response body".
fn describe(err: reqwest::Error) -> String {
    let mut message = err.to_string();
    let mut cause = std::error::Error::source(&err);
    while let Some(err) = cause {
        message.push_str(": ");
        message.push_str(&err.to_string());
        cause = err.source();
    }
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_placeholders() {
        assert_eq!(placeholders("Bearer {secret:key}"), ["key"]);
        assert_eq!(placeholders("{secret:a}:{secret:b}"), ["a", "b"]);
        assert!(placeholders("Bearer {secret:unclosed").is_empty());
        assert!(placeholders("plain").is_empty());
    }

    #[test]
    fn keys_need_https_or_this_machine() {
        assert!(may_carry_keys("https://api.example.com/v1"));
        assert!(may_carry_keys("http://localhost:8880/tts"));
        assert!(may_carry_keys("http://127.0.0.1:8880/tts"));
        assert!(!may_carry_keys("http://api.example.com/v1"));
    }
}
