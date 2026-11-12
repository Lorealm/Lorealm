use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use lorealm_encryption::LocalKeyManager;
use lorealm_storage::{
    BlobStore, CharacterStorage, ConnectionStorage, LorebookStorage, PersonaStorage, StorageError,
    TemplateStorage, UserToolStorage, UserWidgetStorage,
    backends::MemoryStorage,
    models::{
        BlobId, Character, CharacterDefinition, CharacterMetadata, ConnectionProfile, Lorebook,
        Param, Snowflake, Template, ToolSurface, UserPersona, UserTool, UserWidget,
        VersionedSaveMode,
    },
};

use crate::services::{
    blueprint::BlueprintService,
    character::CharacterService,
    chat::ChatService,
    connection::ConnectionService,
    context_template::{SourceRegistry, TemplateService, builtin::register_builtin_sources},
    conversation::ConversationService,
    permission::service::PermissionService,
    persona::PersonaService,
    plugins::PluginService,
    realm::RealmService,
    snowflake::SnowflakeService,
    state::StateService,
    tokenizer::TokenizerService,
    tools::UserToolService,
    variables::{VariableRegistry, builtin::register_builtin_providers},
};

/// Content-addressed blobs in a map.
#[derive(Debug, Default)]
pub(crate) struct MemoryBlobs(Mutex<HashMap<String, Bytes>>);

#[async_trait]
impl BlobStore for MemoryBlobs {
    async fn put(&self, bytes: &[u8]) -> Result<BlobId, StorageError> {
        let id = BlobId::from_content(bytes);
        self.0
            .lock()
            .unwrap()
            .insert(id.as_hex().to_string(), Bytes::copy_from_slice(bytes));
        Ok(id)
    }

    async fn get(&self, id: &BlobId) -> Result<Option<Bytes>, StorageError> {
        Ok(self.0.lock().unwrap().get(id.as_hex()).cloned())
    }

    async fn exists(&self, id: &BlobId) -> Result<bool, StorageError> {
        Ok(self.0.lock().unwrap().contains_key(id.as_hex()))
    }
}

/// The chat and realm services over in-memory storage
pub(crate) struct Harness {
    pub chat: Arc<ChatService>,
    pub realms: Arc<RealmService>,
    pub blueprints: Arc<BlueprintService>,
    pub store: Arc<MemoryStorage>,
    pub blobs: Arc<dyn BlobStore>,
    pub snow: Arc<SnowflakeService>,
    pub plugins: Arc<PluginService>,
    pub connection: Arc<ConnectionService>,
    pub permissions: Arc<PermissionService>,
    pub user: Snowflake,
    pub stranger: Snowflake,
}

impl Harness {
    pub async fn new() -> Self {
        Self::with_plugins(std::path::PathBuf::new()).await
    }

    /// With the plugins installed in `dir`, wired to the real chat service.
    pub async fn with_plugins(dir: std::path::PathBuf) -> Self {
        let snow = Arc::new(SnowflakeService::new(0));
        let store = Arc::new(MemoryStorage::new());
        let storage = Arc::clone(&store) as Arc<dyn lorealm_storage::Storage>;
        let blob = Arc::new(MemoryBlobs::default()) as Arc<dyn BlobStore>;

        let characters = Arc::new(CharacterService::new(
            Arc::clone(&storage),
            Arc::clone(&blob),
            Arc::clone(&snow),
            Default::default(),
        ));
        let state = Arc::new(StateService::new(Arc::clone(&storage)));
        let sources = {
            let mut registry = SourceRegistry::new();
            register_builtin_sources(&mut registry, Arc::clone(&state));
            Arc::new(registry)
        };
        let variables = {
            let mut registry = VariableRegistry::new();
            register_builtin_providers(&mut registry);
            Arc::new(registry)
        };
        let context = Arc::new(TemplateService::new(
            Arc::clone(&storage),
            Arc::clone(&snow),
            Arc::clone(&sources),
        ));
        let persona = Arc::new(PersonaService::new(
            Arc::clone(&storage),
            Arc::clone(&blob),
            Arc::clone(&snow),
        ));
        let conversation = Arc::new(ConversationService::new(
            Arc::clone(&storage),
            Arc::clone(&snow),
        ));
        let tools = Arc::new(UserToolService::new(
            Arc::clone(&storage),
            Arc::clone(&snow),
        ));
        let connection = Arc::new(ConnectionService::new(
            Arc::clone(&storage),
            Arc::clone(&snow),
            Arc::new(LocalKeyManager::new_random()),
        ));
        let tokenizers = Arc::new(TokenizerService::new(
            Arc::clone(&storage),
            Arc::clone(&snow),
            Arc::clone(&blob),
            reqwest::Client::new(),
            Default::default(),
        ));
        let plugins = Arc::new(PluginService::new(
            Arc::clone(&storage),
            Arc::clone(&snow),
            Arc::clone(&state),
            Arc::clone(&connection),
            Default::default(),
            dir,
        ));
        let chat = Arc::new(ChatService::new(
            Arc::clone(&storage),
            Arc::clone(&snow),
            Arc::clone(&conversation),
            Arc::clone(&characters),
            Arc::clone(&state),
            Arc::clone(&persona),
            Arc::clone(&connection),
            Arc::clone(&tools),
            Arc::clone(&tokenizers),
            Arc::clone(&sources),
            Arc::clone(&variables),
            Arc::clone(&plugins),
        ));
        chat.set_self(Arc::downgrade(&chat));
        plugins.set_generation(
            Arc::downgrade(&chat) as std::sync::Weak<dyn crate::services::commands::Generation>
        );

        let realms = Arc::new(RealmService::new(
            Arc::clone(&storage),
            Arc::clone(&snow),
            Arc::clone(&characters),
            Arc::clone(&context),
            Arc::clone(&connection),
            Arc::clone(&persona),
            Arc::clone(&plugins),
        ));

        let blueprints = Arc::new(BlueprintService::new(
            Arc::clone(&storage),
            Arc::clone(&snow),
            Arc::clone(&realms),
            characters,
            context,
            persona,
            Arc::new(crate::services::module::ModuleService::new(
                Arc::clone(&storage),
                Arc::clone(&snow),
            )),
        ));

        let permissions = Arc::new(PermissionService::new(
            Arc::clone(&storage),
            Arc::clone(&snow),
        ));

        Self {
            chat,
            realms,
            blueprints,
            store,
            blobs: blob,
            user: snow.generate(),
            stranger: snow.generate(),
            snow,
            plugins,
            connection,
            permissions,
        }
    }

    pub async fn character(&self, owner: Snowflake, name: &str) -> Snowflake {
        let id = self.snow.generate();
        let character = Character {
            id,
            user_id: owner,
            current_version_id: self.snow.generate(),
            metadata: CharacterMetadata {
                display_name: None,
                alt_names: Vec::new(),
                tags: Vec::new(),
                creator: String::new(),
                creator_notes: String::new(),
                card_version: "1.0.0".into(),
                avatar_uri: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            definition: CharacterDefinition {
                name: name.into(),
                description: format!("{name} the regent."),
                ..Default::default()
            },
            lorebook_ids: Vec::new(),
            tool_ids: Vec::new(),
            widget_ids: Vec::new(),
            extensions: serde_json::json!({}),
        };
        self.store
            .save_character(&character, VersionedSaveMode::NewVersion)
            .await
            .unwrap();
        id
    }

    pub async fn template(&self, owner: Snowflake, name: &str) -> Snowflake {
        let id = self.snow.generate();
        let template = Template {
            id,
            owner_id: owner,
            name: name.into(),
            description: String::new(),
            nodes_json: serde_json::json!([]),
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.store.save_template(&template).await.unwrap();
        id
    }

    pub async fn profile(&self, owner: Snowflake, name: &str) -> Snowflake {
        let id = self.snow.generate();
        let profile = ConnectionProfile {
            id,
            user_id: owner,
            secret_id: None,
            name: name.into(),
            provider: "openai".into(),
            model: Some("stub".into()),
            token_budget: 4096,
            common_params: serde_json::json!({}),
            provider_params: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.store.save_profile(&profile).await.unwrap();
        id
    }

    pub async fn lorebook(&self, owner: Snowflake, name: &str) -> Snowflake {
        let id = self.snow.generate();
        let book = Lorebook {
            id,
            owner_id: owner,
            current_version_id: self.snow.generate(),
            name: name.into(),
            description: String::new(),
            config_yaml: String::new(),
            schema_version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            entries: Vec::new(),
        };
        self.store.save_lorebook(&book).await.unwrap();
        id
    }

    pub async fn tool(&self, owner: Snowflake, name: &str) -> Snowflake {
        let id = self.snow.generate();
        let tool = UserTool {
            id,
            user_id: owner,
            name: name.into(),
            display_name: None,
            description: String::new(),
            params: Vec::<Param>::new(),
            body: "{# return 1 #}".into(),
            surface: ToolSurface::Tool,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.store.save_tool(tool).await.unwrap();
        id
    }

    pub async fn persona(&self, owner: Snowflake, name: &str) -> Snowflake {
        let id = self.snow.generate();
        let persona = UserPersona {
            id,
            user_id: owner,
            name: name.into(),
            display_name: None,
            pseudonym: None,
            avatar_uri: None,
            description: None,
            updated_at: Utc::now(),
        };
        self.store.save_persona(persona).await.unwrap();
        id
    }

    pub async fn widget(&self, owner: Snowflake, name: &str) -> Snowflake {
        let id = self.snow.generate();
        let widget = UserWidget {
            id,
            user_id: owner,
            name: name.into(),
            description: String::new(),
            source: String::new(),
            prompt: String::new(),
            render: String::new(),
            tools: Vec::new(),
            enabled: true,
            placement: Default::default(),
            mode: Default::default(),
            props: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.store.save_widget(widget).await.unwrap();
        id
    }
}
