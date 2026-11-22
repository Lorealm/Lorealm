//! The slice of the Lorealm HTTP API the load generator needs, plus seeding.

use std::{
    ops::Range,
    sync::{Arc, Mutex},
};

use anyhow::{Context, bail};
use futures_util::{StreamExt, stream};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

pub const PROFILE_NAME: &str = "Bench mock";
pub const CHARACTER_NAME: &str = "Bench Character";
const PASSWORD: &str = "bench-password-1234";

#[derive(Clone)]
pub struct Api {
    pub http: reqwest::Client,
    pub base: String,
}

#[derive(Clone)]
pub struct Session {
    token: Arc<Mutex<String>>,
}

impl Session {
    pub fn bearer(&self) -> String {
        format!("Bearer {}", self.token.lock().unwrap())
    }

    /// Adopts a rotated token, if the response carries one.
    pub fn absorb(&self, response: &reqwest::Response) {
        if let Some(token) = response
            .headers()
            .get("X-New-Session-Token")
            .and_then(|v| v.to_str().ok())
        {
            *self.token.lock().unwrap() = token.to_string();
        }
    }
}

/// A conversation ready for turns, and whose it is.
#[derive(Clone)]
pub struct Conversation {
    pub id: String,
    pub session: Session,
}

impl Api {
    pub fn new(base: &str, timeout_secs: u64) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .tcp_nodelay(true)
            .pool_max_idle_per_host(usize::MAX)
            .connect_timeout(std::time::Duration::from_secs(10))
            .read_timeout(std::time::Duration::from_secs(timeout_secs))
            .build()?;
        Ok(Self {
            http,
            base: format!("{}/api/v1", base.trim_end_matches('/')),
        })
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    async fn call(
        &self,
        session: Option<&Session>,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> anyhow::Result<(StatusCode, Value)> {
        let mut request = self.http.request(method.clone(), self.url(path));
        if let Some(session) = session {
            request = request.header("Authorization", session.bearer());
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .with_context(|| format!("{method} {path}"))?;
        if let Some(session) = session {
            session.absorb(&response);
        }
        let status = response.status();
        let text = response.text().await?;
        let value = if text.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(Value::String(text))
        };
        Ok((status, value))
    }

    /// Like [`Self::call`], but a non-2xx status is an error.
    async fn ok(
        &self,
        session: &Session,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> anyhow::Result<Value> {
        let (status, value) = self.call(Some(session), method.clone(), path, body).await?;
        if !status.is_success() {
            bail!("{method} {path}: {status}: {value}");
        }
        Ok(value)
    }

    pub async fn health(&self) -> anyhow::Result<()> {
        let (status, _) = self.call(None, Method::GET, "/health", None).await?;
        if !status.is_success() {
            bail!("health check returned {status}");
        }
        Ok(())
    }

    pub async fn login(&self, username: &str, password: &str) -> anyhow::Result<Session> {
        let (status, value) = self
            .call(
                None,
                Method::POST,
                "/users/login",
                Some(json!({ "username": username, "password": password })),
            )
            .await?;
        if !status.is_success() {
            bail!("logging in as {username}: {status}: {value}");
        }
        let token = value["session_token"]
            .as_str()
            .context("login response has no session_token")?;
        Ok(Session {
            token: Arc::new(Mutex::new(token.to_string())),
        })
    }

    /// Logs in as `username`, creating the account first if it doesn't exist.
    async fn ensure_user(&self, admin: &Session, username: &str) -> anyhow::Result<Session> {
        if let Ok(session) = self.login(username, PASSWORD).await {
            return Ok(session);
        }
        let (status, value) = self
            .call(
                Some(admin),
                Method::POST,
                "/users",
                Some(json!({ "username": username, "password": PASSWORD })),
            )
            .await?;
        if !status.is_success() {
            bail!("creating user {username}: {status}: {value}");
        }
        self.login(username, PASSWORD).await
    }

    /// Points the user's active connection at the mock.
    async fn ensure_profile(
        &self,
        session: &Session,
        model: &str,
        mock_url: &str,
    ) -> anyhow::Result<()> {
        let body = json!({
            "secret_id": null,
            "name": PROFILE_NAME,
            "provider": "openai",
            "model": model,
            "token_budget": 8192,
            "common_params": {},
            "provider_params": { "base_url": mock_url },
        });
        let existing = self
            .ok(session, Method::GET, "/connections/profiles", None)
            .await?;
        let found = existing
            .as_array()
            .into_iter()
            .flatten()
            .find(|p| p["name"] == PROFILE_NAME)
            .and_then(|p| p["id"].as_str().map(str::to_string));
        let profile = match found {
            Some(id) => {
                self.ok(
                    session,
                    Method::PUT,
                    &format!("/connections/profiles/{id}"),
                    Some(body),
                )
                .await?
            }
            None => {
                self.ok(session, Method::POST, "/connections/profiles", Some(body))
                    .await?
            }
        };
        let id = profile["id"].as_str().context("profile has no id")?;
        self.ok(
            session,
            Method::POST,
            "/connections/profiles/active",
            Some(json!({ "id": id })),
        )
        .await?;
        Ok(())
    }

    /// Finds or installs a tokenizer from a network source such as
    /// `hf:owner/model`
    async fn ensure_tokenizer(
        &self,
        session: &Session,
        source: &str,
        public: bool,
    ) -> anyhow::Result<String> {
        let name = source.strip_prefix("hf:").unwrap_or(source);
        let list = self.ok(session, Method::GET, "/tokenizers", None).await?;
        let found = list
            .as_array()
            .into_iter()
            .flatten()
            .find(|t| t["name"] == name && t["is_public"] == public)
            .and_then(|t| t["id"].as_str().map(str::to_string));
        if let Some(id) = found {
            return Ok(id);
        }
        let created = self
            .ok(
                session,
                Method::POST,
                "/tokenizers/network",
                Some(json!({ "source": source, "is_public": public })),
            )
            .await
            .with_context(|| format!("installing tokenizer {source}"))?;
        Ok(created["id"]
            .as_str()
            .context("tokenizer has no id")?
            .to_string())
    }

    async fn activate_tokenizer(&self, session: &Session, id: &str) -> anyhow::Result<()> {
        self.ok(
            session,
            Method::PUT,
            "/tokenizers/active",
            Some(json!({ "id": id })),
        )
        .await?;
        Ok(())
    }

    /// Finds or makes the bench character. Returns its id and a greeting id.
    async fn ensure_character(
        &self,
        session: &Session,
        description: &str,
    ) -> anyhow::Result<(String, String)> {
        let list = self.ok(session, Method::GET, "/characters", None).await?;
        let found = list
            .as_array()
            .into_iter()
            .flatten()
            .find(|c| c["name"] == CHARACTER_NAME)
            .and_then(|c| c["id"].as_str().map(str::to_string));
        let id = match found {
            Some(id) => id,
            None => {
                let created = self
                    .ok(
                        session,
                        Method::POST,
                        "/characters",
                        Some(json!({ "name": CHARACTER_NAME })),
                    )
                    .await?;
                created["id"]
                    .as_str()
                    .context("character has no id")?
                    .to_string()
            }
        };

        // Always rewrite the description so `--character-bytes` takes effect
        // on reruns too.
        let mut edits = vec![json!({ "op": "definition", "description": description })];
        let current = self
            .ok(session, Method::GET, &format!("/characters/{id}"), None)
            .await?;
        if greeting_id(&current).is_none() {
            edits.push(json!({
                "op": "add_greeting",
                "text": "The tavern is warm and loud. A stranger waves you over.",
            }));
        }
        let character = self
            .ok(
                session,
                Method::PUT,
                &format!("/characters/{id}"),
                Some(Value::Array(edits)),
            )
            .await?;
        let greeting = greeting_id(&character).context("character has no greeting")?;
        Ok((id, greeting))
    }

    async fn start_chat(
        &self,
        session: &Session,
        character_id: &str,
        greeting_id: &str,
    ) -> anyhow::Result<String> {
        let chat = self
            .ok(
                session,
                Method::POST,
                "/chat",
                Some(json!({ "character_id": character_id, "greeting_id": greeting_id })),
            )
            .await?;
        Ok(chat["metadata"]["id"]
            .as_str()
            .context("conversation has no id")?
            .to_string())
    }
}

fn greeting_id(character: &Value) -> Option<String> {
    character["definition"]["greetings"].as_array()?.first()?["id"]
        .as_str()
        .map(str::to_string)
}

pub struct SeedPlan<'a> {
    pub admin_user: &'a str,
    pub admin_password: &'a str,
    pub user_prefix: &'a str,
    pub convs_per_user: usize,
    pub model: &'a str,
    pub mock_urls: &'a [String],
    pub character_bytes: usize,
    pub tokenizer: &'a str,
    /// Give every user a private copy instead of sharing one public tokenizer.
    pub tokenizer_per_user: bool,
    pub concurrency: usize,
}

/// A seeded user, ready to start conversations.
#[derive(Clone)]
pub struct BenchUser {
    session: Session,
    character: String,
    greeting: String,
}

pub async fn seed_users(
    api: &Api,
    plan: &SeedPlan<'_>,
    users: Range<usize>,
) -> anyhow::Result<Vec<BenchUser>> {
    let admin = api
        .login(plan.admin_user, plan.admin_password)
        .await
        .context("logging in as the admin (see --admin-user / --admin-password)")?;
    let description = filler(plan.character_bytes);
    let shared_tokenizer = if plan.tokenizer_per_user {
        None
    } else {
        Some(api.ensure_tokenizer(&admin, plan.tokenizer, true).await?)
    };

    let seeded: Vec<anyhow::Result<BenchUser>> = stream::iter(users.clone())
        .map(|i| {
            let (admin, description, shared_tokenizer) =
                (admin.clone(), description.clone(), shared_tokenizer.clone());
            async move {
                let username = format!("{}_{i}", plan.user_prefix);
                let session = api.ensure_user(&admin, &username).await?;
                let tokenizer = match shared_tokenizer {
                    Some(id) => id,
                    None => {
                        api.ensure_tokenizer(&session, plan.tokenizer, false)
                            .await?
                    }
                };
                api.activate_tokenizer(&session, &tokenizer).await?;
                let mock_url = &plan.mock_urls[i % plan.mock_urls.len()];
                api.ensure_profile(&session, plan.model, mock_url).await?;
                let (character, greeting) = api.ensure_character(&session, &description).await?;
                anyhow::Ok(BenchUser {
                    session,
                    character,
                    greeting,
                })
            }
        })
        .buffered(plan.concurrency.max(1))
        .collect()
        .await;

    users
        .zip(seeded)
        .map(|(i, result)| result.with_context(|| format!("seeding user {i}")))
        .collect()
}

pub async fn start_conversations(
    api: &Api,
    users: &[BenchUser],
    count: usize,
    concurrency: usize,
) -> anyhow::Result<Vec<Conversation>> {
    if users.is_empty() {
        bail!("no users to start conversations for");
    }
    stream::iter(0..count)
        .map(|i| {
            let user = &users[i % users.len()];
            async move {
                let id = api
                    .start_chat(&user.session, &user.character, &user.greeting)
                    .await?;
                anyhow::Ok(Conversation {
                    id,
                    session: user.session.clone(),
                })
            }
        })
        .buffered(concurrency.max(1))
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect()
}

/// Roughly `bytes` of prose, to make the prompt as large as needed.
fn filler(bytes: usize) -> String {
    const SENTENCE: &str = "A weathered innkeeper who remembers every traveller's name and \
        trades rumours for coin, wary of the guild and fond of the old songs. ";
    let mut out = String::with_capacity(bytes + SENTENCE.len());
    while out.len() < bytes.max(1) {
        out.push_str(SENTENCE);
    }
    out
}
