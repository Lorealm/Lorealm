use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Duration, Utc};
use kitoken::Kitoken;
use lorealm_storage::{
    BlobStore, Storage,
    models::{BlobId, Snowflake, Tokenizer},
};
use tokio::{sync::RwLock, time::Instant};

use crate::{
    config::TokenizerConfig, core::CleanableService, errors::TokenizerError,
    services::snowflake::SnowflakeService,
};

struct TokenizerSession {
    encoder: Arc<kitoken::Kitoken>,
    expires_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct TokenizerService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
    blob: Arc<dyn BlobStore>,
    http: reqwest::Client,
    config: TokenizerConfig,
    sessions: Arc<RwLock<HashMap<Snowflake, TokenizerSession>>>,
}

impl TokenizerService {
    pub fn new(
        storage: Arc<dyn Storage>,
        snowflake: Arc<SnowflakeService>,
        blob: Arc<dyn BlobStore>,
        http: reqwest::Client,
        config: TokenizerConfig,
    ) -> Self {
        Self {
            storage,
            snowflake,
            blob,
            http,
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Convert raw tokenizer bytes to kitoken's canonical format, store them as a
    /// blob, and register the row. This is the single ingest path both the upload
    /// and network sources funnel through.
    pub async fn ingest_tokenizer(
        &self,
        owner_id: Snowflake,
        name: String,
        raw: Bytes,
        is_public: bool,
    ) -> Result<Tokenizer, TokenizerError> {
        if raw.len() as u64 > self.config.max_tokenizer_size_mb * 1024 * 1024 {
            return Err(TokenizerError::InvalidData(
                "tokenizer exceeds size limit".into(),
            ));
        }

        let canonical = Self::to_canonical(raw).await?;
        let blob_id = self.blob.put(&canonical).await?;
        self.create_tokenizer(owner_id, name, blob_id.to_uri(), is_public)
            .await
    }

    /// Fetch a tokenizer from a supported network source and ingest it. Only the
    /// `hf:owner/model` coordinate form is accepted; the host is locked to
    /// huggingface.co, and bare URLs are rejected to avoid SSRF.
    pub async fn ingest_from_network(
        &self,
        owner_id: Snowflake,
        source: &str,
        name: Option<String>,
        is_public: bool,
    ) -> Result<Tokenizer, TokenizerError> {
        let repo = source
            .strip_prefix("hf:")
            .filter(|r| !r.is_empty() && !r.contains("://"))
            .ok_or_else(|| TokenizerError::InvalidSource(source.to_string()))?;

        let name = name.unwrap_or_else(|| repo.to_string());
        let raw = self.fetch_hf(repo).await?;
        self.ingest_tokenizer(owner_id, name, raw, is_public).await
    }

    /// Register a tokenizer whose blob is already stored.
    pub async fn create_tokenizer(
        &self,
        owner_id: Snowflake,
        name: String,
        blob_uri: String,
        is_public: bool,
    ) -> Result<Tokenizer, TokenizerError> {
        let tokenizer = Tokenizer {
            id: self.snowflake.generate(),
            name,
            blob_uri,
            owner_id,
            is_public,
        };

        self.storage.save_tokenizer(&tokenizer).await?;
        Ok(tokenizer)
    }

    /// Fetch a tokenizer the caller is allowed to use (owner, public, or shared).
    pub async fn get_tokenizer(
        &self,
        requester_id: Snowflake,
        id: Snowflake,
    ) -> Result<Tokenizer, TokenizerError> {
        if !self.storage.can_access_tokenizer(id, requester_id).await? {
            return Err(TokenizerError::NotFound(id));
        }

        self.storage
            .get_tokenizer(id)
            .await?
            .ok_or(TokenizerError::NotFound(id))
    }

    /// Every tokenizer the caller can use: owned, public, or explicitly shared.
    pub async fn list_accessible(
        &self,
        requester_id: Snowflake,
    ) -> Result<Vec<Tokenizer>, TokenizerError> {
        Ok(self
            .storage
            .list_accessible_tokenizers(requester_id)
            .await?)
    }

    /// Delete a tokenizer the caller owns. Share rows cascade in storage.
    pub async fn delete_tokenizer(
        &self,
        requester_id: Snowflake,
        id: Snowflake,
    ) -> Result<(), TokenizerError> {
        self.get_owned(requester_id, id).await?;
        self.storage.delete_tokenizer(id).await?;
        Ok(())
    }

    /// Grant another user access. Owner only.
    pub async fn share_tokenizer(
        &self,
        requester_id: Snowflake,
        tok_id: Snowflake,
        target_user_id: Snowflake,
    ) -> Result<(), TokenizerError> {
        self.get_owned(requester_id, tok_id).await?;
        self.storage.share_tokenizer(tok_id, target_user_id).await?;
        Ok(())
    }

    /// Revoke another user's access. Owner only.
    pub async fn unshare_tokenizer(
        &self,
        requester_id: Snowflake,
        tok_id: Snowflake,
        target_user_id: Snowflake,
    ) -> Result<(), TokenizerError> {
        self.get_owned(requester_id, tok_id).await?;
        self.storage
            .unshare_tokenizer(tok_id, target_user_id)
            .await?;
        Ok(())
    }

    /// List the users a tokenizer is shared with. Owner only.
    pub async fn list_shares(
        &self,
        requester_id: Snowflake,
        tok_id: Snowflake,
    ) -> Result<Vec<Snowflake>, TokenizerError> {
        self.get_owned(requester_id, tok_id).await?;
        Ok(self.storage.list_tokenizer_shares(tok_id).await?)
    }

    /// Fetch a tokenizer and assert the caller owns it.
    async fn get_owned(
        &self,
        requester_id: Snowflake,
        id: Snowflake,
    ) -> Result<Tokenizer, TokenizerError> {
        let tokenizer = self
            .storage
            .get_tokenizer(id)
            .await?
            .ok_or(TokenizerError::NotFound(id))?;

        if tokenizer.owner_id != requester_id {
            return Err(TokenizerError::Forbidden(id));
        }

        Ok(tokenizer)
    }

    /// Resolve an `hf:` coordinate to its `tokenizer.json` on huggingface.co and
    /// download it, bounding the response size.
    async fn fetch_hf(&self, repo: &str) -> Result<Bytes, TokenizerError> {
        let url = format!("https://huggingface.co/{repo}/resolve/main/tokenizer.json");

        let max_bytes = self.config.max_tokenizer_size_mb * 1024 * 1024;

        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| TokenizerError::Fetch(e.to_string()))?;

        if let Some(len) = resp.content_length()
            && len > max_bytes
        {
            return Err(TokenizerError::Fetch(format!(
                "tokenizer too large ({len} bytes)"
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| TokenizerError::Fetch(e.to_string()))?;

        if bytes.len() as u64 > max_bytes {
            return Err(TokenizerError::Fetch("tokenizer too large".into()));
        }

        Ok(bytes)
    }

    pub async fn load_kitoken_tokenizer(
        &self,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Arc<Kitoken>, TokenizerError> {
        // Authorize on every call, including cache hits.
        let tokenizer_def = self.get_owned(user_id, tok_id).await?;

        // Fast path: short critical section, no .await held.
        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(&tok_id) {
                session.expires_at = Utc::now() + Duration::seconds(self.config.tokenizer_ttl);
                return Ok(Arc::clone(&session.encoder));
            }
        }

        tracing::debug!(
            "Tokenizer '{}' was not found in cache; proceeding to build from blob",
            tokenizer_def.name
        );
        let start = Instant::now();
        // Slow path: fetch + build with no lock held.
        let blob_id =
            BlobId::from_uri(&tokenizer_def.blob_uri).ok_or(TokenizerError::NotFound(tok_id))?;
        let blob = self
            .blob
            .get(&blob_id)
            .await?
            .ok_or(TokenizerError::NotFound(tok_id))?;
        let tokenizer = Arc::new(
            Kitoken::from_slice(blob.as_ref())
                .map_err(|e| TokenizerError::InvalidData(e.to_string()))?,
        );

        let end = Instant::now();
        tracing::trace!(
            "Built tokenizer '{}' in {:?}",
            tokenizer_def.name,
            end - start
        );

        // Re-acquire; another task may have inserted while we fetched.
        let mut sessions = self.sessions.write().await;
        let session = sessions.entry(tok_id).or_insert_with(|| TokenizerSession {
            encoder: Arc::clone(&tokenizer),
            expires_at: Utc::now() + Duration::seconds(self.config.tokenizer_ttl),
        });
        Ok(Arc::clone(&session.encoder))
    }

    pub async fn tokenize(
        &self,
        content: String,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<u32>, TokenizerError> {
        let tokenizer = self.load_kitoken_tokenizer(tok_id, user_id).await?;

        let tokens = tokenizer
            .encode(content, true)
            .map_err(|e| TokenizerError::Tokenize(e.to_string()))?;

        Ok(tokens)
    }

    pub async fn batch_tokenize(
        &self,
        batch: HashMap<String, String>,
        tok_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<HashMap<String, Vec<u32>>, TokenizerError> {
        let tokenizer = self.load_kitoken_tokenizer(tok_id, user_id).await?;
        let mut output = HashMap::new();
        let mut total = 0;

        let start = Instant::now();

        for (key, value) in batch.iter() {
            let tokens = tokenizer
                .encode(value, true)
                .map_err(|e| TokenizerError::Tokenize(e.to_string()))?;

            total += tokens.len();
            output.insert(key.to_owned(), tokens);
        }

        let end = Instant::now();
        tracing::debug!(
            "Tokenized {} entries ({total} tokens) in {:?}",
            output.len(),
            end - start
        );
        Ok(output)
    }

    /// Validate + convert to kitoken's canonical binary on a blocking thread,
    /// since `from_slice` is CPU-bound. Invalid bytes surface as `InvalidData`.
    async fn to_canonical(raw: Bytes) -> Result<Vec<u8>, TokenizerError> {
        tokio::task::spawn_blocking(move || -> Result<Vec<u8>, TokenizerError> {
            let tokenizer = Kitoken::from_slice(&raw)
                .map_err(|e| TokenizerError::InvalidData(e.to_string()))?;
            Ok(tokenizer.to_vec())
        })
        .await
        .map_err(|e| TokenizerError::InvalidData(format!("conversion task panicked: {e}")))?
    }
}

#[async_trait]
impl CleanableService for TokenizerService {
    fn name(&self) -> &'static str {
        "TokenizerService"
    }
    async fn clean(&self) -> usize {
        let now = Utc::now();
        let mut sessions = self.sessions.write().await;
        let initial = sessions.len();
        sessions.retain(|_, v| v.expires_at > now);
        initial - sessions.len()
    }
}
