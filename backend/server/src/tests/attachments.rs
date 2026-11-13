use std::{collections::HashSet, sync::Arc};

use lorealm_storage::{
    BlobStore, Storage, StorageCore,
    backends::{MemoryStorage, SqliteStorage},
    models::{CreateUser, Snowflake},
};

use crate::{
    errors::CharacterError,
    services::{
        character::{AttachmentKind, CharacterService},
        lorebook::{CreateLorebook, LorebookService},
        snowflake::SnowflakeService,
        tools::{CreateUserTool, UserToolService, unique_tool_name},
        user_widgets::{CreateUserWidget, UserWidgetService},
    },
    tests::harness::MemoryBlobs,
};

struct Env {
    storage: Arc<dyn Storage>,
    snow: Arc<SnowflakeService>,
    characters: CharacterService,
    lorebooks: LorebookService,
    tools: UserToolService,
    widgets: UserWidgetService,
    _dir: Option<tempfile::TempDir>,
}

fn env(storage: Arc<dyn Storage>, dir: Option<tempfile::TempDir>) -> Env {
    let snow = Arc::new(SnowflakeService::new(0));
    let blob = Arc::new(MemoryBlobs::default()) as Arc<dyn BlobStore>;
    Env {
        characters: CharacterService::new(
            Arc::clone(&storage),
            blob,
            Arc::clone(&snow),
            Default::default(),
        ),
        lorebooks: LorebookService::new(Arc::clone(&storage), Arc::clone(&snow)),
        tools: UserToolService::new(Arc::clone(&storage), Arc::clone(&snow)),
        widgets: UserWidgetService::new(Arc::clone(&storage), Arc::clone(&snow)),
        storage,
        snow,
        _dir: dir,
    }
}

async fn both() -> [Env; 2] {
    let dir = tempfile::tempdir().unwrap();
    let mut db = SqliteStorage::new(dir.path().join("test.db"), 5, true)
        .await
        .unwrap();
    db.initialize().await.unwrap();
    [
        env(Arc::new(MemoryStorage::new()), None),
        env(Arc::new(db), Some(dir)),
    ]
}

async fn user(e: &Env, name: &str) -> Snowflake {
    let id = e.snow.generate();
    e.storage
        .create_user(&CreateUser {
            id,
            username: name.into(),
            email: None,
            password_hash: "hash".into(),
        })
        .await
        .unwrap();
    id
}

async fn tool(e: &Env, owner: Snowflake, name: &str) -> Snowflake {
    e.tools
        .create_tool(
            owner,
            CreateUserTool {
                name: name.into(),
                display_name: None,
                description: "test tool".into(),
                params: Vec::new(),
                body: "{# return 1 #}".into(),
                surface: None,
            },
        )
        .await
        .unwrap()
}

async fn lorebook(e: &Env, owner: Snowflake, name: &str) -> Snowflake {
    let book = e
        .lorebooks
        .create_lorebook(
            owner,
            CreateLorebook {
                name: name.into(),
                description: None,
                _preset: None,
            },
        )
        .await
        .unwrap();
    e.lorebooks.create_entry(book.id, owner).await.unwrap();
    book.id
}

async fn widget(e: &Env, owner: Snowflake, name: &str) -> Snowflake {
    e.widgets
        .create_widget(
            owner,
            CreateUserWidget {
                name: name.into(),
                description: String::new(),
                source: String::new(),
                prompt: String::new(),
                render: String::new(),
                tools: Vec::new(),
                placement: Default::default(),
                mode: Default::default(),
                props: Vec::new(),
            },
        )
        .await
        .unwrap()
        .id
}

async fn latest(e: &Env, id: Snowflake, owner: Snowflake) -> lorealm_storage::models::Character {
    let version = e.characters.get_latest_version_id(id).await.unwrap();
    e.characters.load_version(id, version, owner).await.unwrap()
}

#[tokio::test]
async fn attachments_keep_their_order_and_survive_a_reload() {
    for e in both().await {
        let owner = user(&e, "owner").await;
        let character = e
            .characters
            .create_character(owner, "C".into())
            .await
            .unwrap();
        let world = lorebook(&e, owner, "World").await;
        let personal = lorebook(&e, owner, "Personal").await;
        let roll = tool(&e, owner, "roll").await;
        let flip = tool(&e, owner, "flip").await;

        e.characters
            .set_attachments(
                character.id,
                owner,
                AttachmentKind::Lorebook,
                vec![personal, world, personal],
            )
            .await
            .unwrap();
        e.characters
            .set_attachments(character.id, owner, AttachmentKind::Tool, vec![flip, roll])
            .await
            .unwrap();

        let loaded = latest(&e, character.id, owner).await;
        assert_eq!(
            loaded.lorebook_ids,
            vec![personal, world],
            "deduplicated, order kept"
        );
        assert_eq!(loaded.tool_ids, vec![flip, roll]);
    }
}

#[tokio::test]
async fn someone_elses_resource_cannot_be_attached() {
    for e in both().await {
        let owner = user(&e, "owner").await;
        let stranger = user(&e, "stranger").await;
        let character = e
            .characters
            .create_character(owner, "C".into())
            .await
            .unwrap();
        let theirs = tool(&e, stranger, "theirs").await;

        let err = e
            .characters
            .set_attachments(character.id, owner, AttachmentKind::Tool, vec![theirs])
            .await
            .unwrap_err();
        assert!(
            matches!(err, CharacterError::NotFound(_, id) if id == theirs),
            "{err:?}"
        );
    }
}

#[tokio::test]
async fn deleting_a_resource_detaches_it() {
    for e in both().await {
        let owner = user(&e, "owner").await;
        let character = e
            .characters
            .create_character(owner, "C".into())
            .await
            .unwrap();
        let book = lorebook(&e, owner, "Book").await;
        let roll = tool(&e, owner, "roll").await;
        let gauge = widget(&e, owner, "Gauge").await;
        for (kind, id) in [
            (AttachmentKind::Lorebook, book),
            (AttachmentKind::Tool, roll),
            (AttachmentKind::Widget, gauge),
        ] {
            e.characters
                .set_attachments(character.id, owner, kind, vec![id])
                .await
                .unwrap();
        }

        e.tools.delete_tool(roll, owner).await.unwrap();
        e.lorebooks.delete_lorebook(book, owner).await.unwrap();
        e.widgets.delete_widget(gauge, owner).await.unwrap();

        let loaded = latest(&e, character.id, owner).await;
        assert!(loaded.tool_ids.is_empty());
        assert!(loaded.lorebook_ids.is_empty());
        assert!(loaded.widget_ids.is_empty());
    }
}

#[tokio::test]
async fn a_duplicated_tool_gets_a_free_name_and_takes_the_originals_place() {
    for e in both().await {
        let owner = user(&e, "owner").await;
        let character = e
            .characters
            .create_character(owner, "C".into())
            .await
            .unwrap();
        let first = tool(&e, owner, "first").await;
        let roll = tool(&e, owner, "roll").await;
        tool(&e, owner, "roll_2").await;
        e.characters
            .set_attachments(character.id, owner, AttachmentKind::Tool, vec![first, roll])
            .await
            .unwrap();

        let copy = e.tools.duplicate_tool(roll, owner).await.unwrap();
        assert_eq!(copy.name, "roll_3");
        assert!(!copy.enabled);

        let updated = e
            .characters
            .replace_attachment(character.id, owner, AttachmentKind::Tool, roll, copy.id)
            .await
            .unwrap();
        assert_eq!(updated.tool_ids, vec![first, copy.id]);
        assert_eq!(
            latest(&e, character.id, owner).await.tool_ids,
            vec![first, copy.id]
        );
        assert!(
            e.storage.get_tool(roll).await.unwrap().is_some(),
            "original kept"
        );
    }
}

#[tokio::test]
async fn a_duplicated_lorebook_copies_its_entries() {
    for e in both().await {
        let owner = user(&e, "owner").await;
        let original = lorebook(&e, owner, "World").await;

        let copy = e
            .lorebooks
            .duplicate_lorebook(original, owner)
            .await
            .unwrap();
        let reloaded = e.storage.get_lorebook(copy.id).await.unwrap().unwrap();
        let source = e.storage.get_lorebook(original).await.unwrap().unwrap();

        assert_eq!(reloaded.name, "World (copy)");
        assert_eq!(reloaded.entries.len(), source.entries.len());
        assert!(
            reloaded
                .entries
                .iter()
                .all(|entry| entry.lorebook_id == copy.id)
        );
        let source_ids: HashSet<_> = source.entries.iter().map(|entry| entry.id).collect();
        assert!(
            reloaded
                .entries
                .iter()
                .all(|entry| !source_ids.contains(&entry.id))
        );
    }
}

#[test]
fn unique_tool_names_stay_within_the_limit() {
    let taken: HashSet<String> = ["roll".into(), "roll_2".into()].into();
    assert_eq!(unique_tool_name("roll", &taken), "roll_3");
    assert_eq!(unique_tool_name("flip", &taken), "flip");

    let long = "x".repeat(64);
    let taken: HashSet<String> = [long.clone()].into();
    let renamed = unique_tool_name(&long, &taken);
    assert_eq!(renamed.len(), 64);
    assert!(renamed.ends_with("_2"));
}
