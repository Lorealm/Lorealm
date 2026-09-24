use std::sync::Arc;

use lorealm_encryption::{
    EncryptedData, EncryptionContext, decrypt_bytes, encrypt_bytes, traits::KeyManager,
};
use lorealm_providers::{
    LlmProvider, ModelInfo, OpenAiOptions, OpenAiProvider, OpenRouterProvider,
};
use lorealm_storage::{
    Storage,
    models::{ConnectionProfile, Secret, SecretMetadata, Snowflake},
};
use secrecy::zeroize::Zeroizing;

use crate::{
    errors::{ConnectionError, TargetType},
    services::{
        connection::{CommonParamsInput, ProfileInput, ResolvedConnection},
        snowflake::SnowflakeService,
    },
};

/// Providers the app can build a client for. Keep in sync with the match in
/// `validate_provider_params` and the dispatch in ChatService::infer.
const SUPPORTED_PROVIDERS: &[&str] = &["openrouter", "openai", "anthropic"];

/// How many trailing characters of a key are kept for display.
const KEY_HINT_LEN: usize = 4;

pub struct ConnectionService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
    key_manager: Arc<dyn KeyManager>,
}

impl ConnectionService {
    pub fn new(
        storage: Arc<dyn Storage>,
        snowflake: Arc<SnowflakeService>,
        key_manager: Arc<dyn KeyManager>,
    ) -> Self {
        Self {
            storage,
            snowflake,
            key_manager,
        }
    }

    /// Store a new provider API key, encrypted and bound to its own row id.
    /// The plaintext never leaves this method except inside the ciphertext.
    /// 
    /// `key_types` refer to custom API key types defined by a plugin.
    pub async fn create_secret(
        &self,
        user_id: Snowflake,
        provider: String,
        label: String,
        plaintext_key: String,
        key_types: &[String],
    ) -> Result<SecretMetadata, ConnectionError> {
        let plaintext_key = Zeroizing::new(plaintext_key);

        if !key_types.contains(&provider) {
            validate_provider(&provider)?;
        }
        if label.trim().is_empty() {
            return Err(ConnectionError::Validation(
                "label must not be empty".into(),
            ));
        }
        if plaintext_key.trim().is_empty() {
            return Err(ConnectionError::Validation("key must not be empty".into()));
        }

        let id = self.snowflake.generate();
        let binding = id.to_be_bytes();

        let encrypted = encrypt_bytes(
            plaintext_key.as_bytes(),
            &*self.key_manager,
            EncryptionContext::ApiKey,
            &binding,
        )
        .await?;

        let key_hint: String = {
            let chars: Vec<char> = plaintext_key.chars().collect();
            chars[chars.len().saturating_sub(KEY_HINT_LEN)..]
                .iter()
                .collect()
        };

        let secret = Secret {
            id,
            user_id,
            provider,
            label,
            key_hint,
            enc_version: encrypted.version as i32,
            context: API_KEY_CONTEXT.to_string(),
            salt: encrypted.salt.clone(),
            nonce: encrypted.nonce.to_vec(),
            ciphertext: encrypted.ciphertext.clone(),
            tag: encrypted.tag.to_vec(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.storage.save_secret(&secret).await?;

        Ok(SecretMetadata {
            id: secret.id,
            user_id: secret.user_id,
            provider: secret.provider,
            label: secret.label,
            key_hint: secret.key_hint,
            created_at: secret.created_at,
            updated_at: secret.updated_at,
        })
    }

    pub async fn list_secrets(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<SecretMetadata>, ConnectionError> {
        Ok(self.storage.list_secrets_by_owner(user_id).await?)
    }

    /// Rename a key. The payload is intentionally not updatable: replacing a
    /// key means creating a new secret and repointing profiles
    pub async fn rename_secret(
        &self,
        user_id: Snowflake,
        secret_id: Snowflake,
        label: String,
    ) -> Result<SecretMetadata, ConnectionError> {
        if label.trim().is_empty() {
            return Err(ConnectionError::Validation(
                "label must not be empty".into(),
            ));
        }
        let mut secret = self.owned_secret(secret_id, user_id).await?;
        secret.label = label;
        self.storage.save_secret(&secret).await?;

        Ok(SecretMetadata {
            id: secret.id,
            user_id: secret.user_id,
            provider: secret.provider,
            label: secret.label,
            key_hint: secret.key_hint,
            created_at: secret.created_at,
            updated_at: secret.updated_at,
        })
    }

    /// Delete a key..
    pub async fn delete_secret(
        &self,
        user_id: Snowflake,
        secret_id: Snowflake,
    ) -> Result<Vec<Snowflake>, ConnectionError> {
        self.owned_secret(secret_id, user_id).await?;

        let detached = self.storage.profiles_using_secret(secret_id).await?;
        self.storage.delete_secret(secret_id).await?;
        Ok(detached)
    }

    pub async fn create_profile(
        &self,
        user_id: Snowflake,
        input: ProfileInput,
    ) -> Result<ConnectionProfile, ConnectionError> {
        let (common, provider_specific) = self.validate_input(user_id, &input).await?;

        let profile = ConnectionProfile {
            id: self.snowflake.generate(),
            user_id,
            secret_id: input.secret_id,
            name: input.name,
            provider: input.provider,
            model: input.model,
            token_budget: input.token_budget,
            common_params: common,
            provider_params: provider_specific,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.storage.save_profile(&profile).await?;
        Ok(profile)
    }

    pub async fn update_profile(
        &self,
        user_id: Snowflake,
        profile_id: Snowflake,
        input: ProfileInput,
    ) -> Result<ConnectionProfile, ConnectionError> {
        let existing = self.owned_profile(profile_id, user_id).await?;
        let (common, provider_specific) = self.validate_input(user_id, &input).await?;

        let profile = ConnectionProfile {
            id: existing.id,
            user_id: existing.user_id,
            secret_id: input.secret_id,
            name: input.name,
            provider: input.provider,
            model: input.model,
            token_budget: input.token_budget,
            common_params: common,
            provider_params: provider_specific,
            created_at: existing.created_at,
            updated_at: chrono::Utc::now(),
        };

        self.storage.save_profile(&profile).await?;
        Ok(profile)
    }

    pub async fn get_profile(
        &self,
        user_id: Snowflake,
        profile_id: Snowflake,
    ) -> Result<ConnectionProfile, ConnectionError> {
        self.owned_profile(profile_id, user_id).await
    }

    pub async fn list_profiles(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<ConnectionProfile>, ConnectionError> {
        Ok(self.storage.list_profiles_by_owner(user_id).await?)
    }

    pub async fn delete_profile(
        &self,
        user_id: Snowflake,
        profile_id: Snowflake,
    ) -> Result<(), ConnectionError> {
        self.owned_profile(profile_id, user_id).await?;
        self.storage.delete_profile(profile_id).await?;
        Ok(())
    }

    /// Load a profile and decrypt its key (if any) for one generation. This
    /// is the only path that touches plaintext key material; it must never
    /// be exposed through an HTTP handler.
    pub async fn resolve_connection(
        &self,
        user_id: Snowflake,
        profile_id: Snowflake,
    ) -> Result<ResolvedConnection, ConnectionError> {
        let profile = self.owned_profile(profile_id, user_id).await?;

        let api_key = match profile.secret_id {
            None => None,
            Some(secret_id) => {
                let secret = self.owned_secret(secret_id, user_id).await?;
                let encrypted = to_encrypted_data(&secret)?;
                let binding = secret.id.to_be_bytes();

                let mut plaintext = decrypt_bytes(&encrypted, &*self.key_manager, &binding).await?;
                let bytes = std::mem::take(&mut *plaintext);
                match String::from_utf8(bytes) {
                    Ok(s) => Some(Zeroizing::new(s)),
                    Err(e) => {
                        drop(Zeroizing::new(e.into_bytes()));
                        return Err(ConnectionError::Corrupted(secret.id));
                    }
                }
            }
        };

        Ok(ResolvedConnection { profile, api_key })
    }

    /// Ask the profile's provider which models it offers. Providers that
    /// don't support listing yield an empty list rather than an error, so
    /// the caller can always treat this as "suggestions, possibly none".
    pub async fn list_models(
        &self,
        user_id: Snowflake,
        profile_id: Snowflake,
    ) -> Result<Vec<ModelInfo>, ConnectionError> {
        let connection = self.resolve_connection(user_id, profile_id).await?;
        let http_client = Arc::new(reqwest::Client::new());

        // A listing request carries no model, hence the empty model name.
        let models = match connection.profile.provider.as_str() {
            // OpenRouter's catalogue is public, so a profile with no key
            // attached yet can still populate the model field.
            "openrouter" => {
                let api_key = connection.api_key.map(|key| key.to_string());
                OpenRouterProvider::new(http_client, api_key.unwrap_or_default(), "")
                    .list_models()
                    .await?
            }
            "openai" => {
                let options: OpenAiOptions = serde_json::from_value(
                    connection.profile.provider_params.clone(),
                )
                .map_err(|e| ConnectionError::Validation(format!("invalid openai params: {e}")))?;
                // A profile with neither an endpoint nor a key is still being
                // filled in. Asking anyway would send an unauthenticated
                // request to the OpenAI default and report its 401 as if the
                // profile were broken.
                if options.base_url.is_none() && connection.api_key.is_none() {
                    return Ok(Vec::new());
                }
                OpenAiProvider::new(
                    http_client,
                    "",
                    options.base_url,
                    connection.api_key.map(|key| key.to_string()),
                )
                .list_models()
                .await?
            }
            other => {
                return Err(ConnectionError::Validation(format!(
                    "unsupported provider: {other}"
                )));
            }
        };

        Ok(models.unwrap_or_default())
    }

    /// Secret exists AND belongs to this user.
    async fn owned_secret(
        &self,
        secret_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Secret, ConnectionError> {
        let secret = self
            .storage
            .get_secret(secret_id)
            .await?
            .ok_or(ConnectionError::NotFound(TargetType::Secret, secret_id))?;
        if secret.user_id != user_id {
            return Err(ConnectionError::NotOwned(TargetType::Secret, secret_id));
        }
        Ok(secret)
    }

    /// Profile exists AND belongs to this user.
    async fn owned_profile(
        &self,
        profile_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<ConnectionProfile, ConnectionError> {
        let profile =
            self.storage
                .get_profile(profile_id)
                .await?
                .ok_or(ConnectionError::NotFound(
                    TargetType::ConnectionProfile,
                    profile_id,
                ))?;
        if profile.user_id != user_id {
            return Err(ConnectionError::NotOwned(
                TargetType::ConnectionProfile,
                profile_id,
            ));
        }
        Ok(profile)
    }

    /// Shared validation for create/update: ownership, provider coherence,
    /// and strict param round-tripping. Returns normalized param values.
    async fn validate_input(
        &self,
        user_id: Snowflake,
        input: &ProfileInput,
    ) -> Result<(serde_json::Value, serde_json::Value), ConnectionError> {
        if input.name.trim().is_empty() {
            return Err(ConnectionError::Validation("name must not be empty".into()));
        }
        validate_provider(&input.provider)?;

        // When a key is attached it must exist, be owned by this user, and
        // match the provider: an OpenAI key on an OpenRouter profile fails
        // at request time with a confusing 401, so catch it here instead.
        if let Some(secret_id) = input.secret_id {
            let secret = self.owned_secret(secret_id, user_id).await?;
            if secret.provider != input.provider {
                return Err(ConnectionError::Validation(format!(
                    "secret '{}' is a {} key, but the profile targets {}",
                    secret.label, secret.provider, input.provider
                )));
            }
        }

        let common = validate_common_params(&input.common_params)?;
        let provider_specific = validate_provider_params(&input.provider, &input.provider_params)?;
        Ok((common, provider_specific))
    }
}

/// Context name as stored in the secrets table
const API_KEY_CONTEXT: &str = "api_key";

fn validate_provider(provider: &str) -> Result<(), ConnectionError> {
    if SUPPORTED_PROVIDERS.contains(&provider) {
        Ok(())
    } else {
        Err(ConnectionError::Validation(format!(
            "unsupported provider: {provider}"
        )))
    }
}

/// An endpoint URL the server will POST to. Only the scheme and a non-empty
/// remainder are checked here; anything past that is the operator's problem.
fn validate_base_url(base_url: &str) -> Result<(), ConnectionError> {
    let rest = base_url
        .strip_prefix("http://")
        .or_else(|| base_url.strip_prefix("https://"));
    match rest {
        Some(rest) if !rest.trim().is_empty() => Ok(()),
        _ => Err(ConnectionError::Validation(
            "base URL must start with http:// or https://, e.g. http://localhost:5001/v1".into(),
        )),
    }
}

/// Strict on write: unknown fields and out-of-range values are rejected,
/// and what's stored is the normalized re-serialization
fn validate_common_params(value: &serde_json::Value) -> Result<serde_json::Value, ConnectionError> {
    let typed: CommonParamsInput = serde_json::from_value(value.clone())
        .map_err(|e| ConnectionError::Validation(format!("invalid common params: {e}")))?;

    if let Some(t) = typed.temperature {
        if !(0.0..=2.0).contains(&t) {
            return Err(ConnectionError::Validation(
                "temperature must be between 0 and 2".into(),
            ));
        }
    }
    if let Some(p) = typed.top_p {
        if !(0.0..=1.0).contains(&p) {
            return Err(ConnectionError::Validation(
                "top_p must be between 0 and 1".into(),
            ));
        }
    }
    if let Some(p) = typed.min_p {
        if !(0.0..=1.0).contains(&p) {
            return Err(ConnectionError::Validation(
                "min_p must be between 0 and 1".into(),
            ));
        }
    }
    if let Some(p) = typed.frequency_penalty {
        if !(-2.0..=2.0).contains(&p) {
            return Err(ConnectionError::Validation(
                "frequency_penalty must be between -2 and 2".into(),
            ));
        }
    }
    if let Some(p) = typed.presence_penalty {
        if !(-2.0..=2.0).contains(&p) {
            return Err(ConnectionError::Validation(
                "presence_penalty must be between -2 and 2".into(),
            ));
        }
    }
    if let Some(p) = typed.repetition_penalty {
        if p <= 0.0 || p > 2.0 {
            return Err(ConnectionError::Validation(
                "repetition_penalty must be greater than 0 and at most 2".into(),
            ));
        }
    }
    if typed.max_tokens == Some(0) {
        return Err(ConnectionError::Validation(
            "max_tokens must be greater than 0".into(),
        ));
    }
    if let Some(stop) = &typed.stop {
        if stop.iter().any(|s| s.is_empty()) {
            return Err(ConnectionError::Validation(
                "stop sequences must not be empty strings".into(),
            ));
        }
    }

    Ok(serde_json::to_value(&typed).expect("DTO serialization cannot fail"))
}

fn validate_provider_params(
    provider: &str,
    value: &serde_json::Value,
) -> Result<serde_json::Value, ConnectionError> {
    match provider {
        "openrouter" => {
            let typed: lorealm_providers::OpenRouterOptions = serde_json::from_value(value.clone())
                .map_err(|e| {
                    ConnectionError::Validation(format!("invalid openrouter params: {e}"))
                })?;
            Ok(serde_json::to_value(&typed).expect("options serialization cannot fail"))
        }
        "openai" => {
            let typed: lorealm_providers::OpenAiOptions = serde_json::from_value(value.clone())
                .map_err(|e| ConnectionError::Validation(format!("invalid openai params: {e}")))?;
            if let Some(base_url) = &typed.base_url {
                validate_base_url(base_url)?;
            }
            Ok(serde_json::to_value(&typed).expect("options serialization cannot fail"))
        }
        other => Err(ConnectionError::Validation(format!(
            "unsupported provider: {other}"
        ))),
    }
}

/// Reconstruct `EncryptedData` from the flat storage columns.
fn to_encrypted_data(secret: &Secret) -> Result<EncryptedData, ConnectionError> {
    if secret.context != API_KEY_CONTEXT {
        return Err(ConnectionError::Corrupted(secret.id));
    }
    Ok(EncryptedData {
        version: secret.enc_version as u32,
        context: EncryptionContext::ApiKey,
        salt: secret.salt.clone(),
        nonce: secret
            .nonce
            .as_slice()
            .try_into()
            .map_err(|_| ConnectionError::Corrupted(secret.id))?,
        ciphertext: secret.ciphertext.clone(),
        tag: secret
            .tag
            .as_slice()
            .try_into()
            .map_err(|_| ConnectionError::Corrupted(secret.id))?,
    })
}
