use std::sync::Arc;

use lorealm_storage::{
    BlobStore, Storage, StorageCore,
    backends::{MemoryStorage, SqliteStorage},
    models::{Character, CreateUser, Snowflake},
};
use serde_json::Value;

use crate::{
    services::{
        cards::{
            export::{self, CharacterBundle, ExportFormat, ExportProfile},
            import::ImportOptions,
        },
        character::{
            AttachmentKind, CharacterService, EditDefinition, EditMetadata,
            card_parser::parse_character_card_png,
        },
        lorebook::{CreateLorebook, LorebookService},
        snowflake::SnowflakeService,
        tools::{CreateUserTool, UserToolService},
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

async fn tool(e: &Env, owner: Snowflake, name: &str, body: &str) -> Snowflake {
    e.tools
        .create_tool(
            owner,
            CreateUserTool {
                name: name.into(),
                display_name: None,
                description: format!("The {name} tool"),
                params: Vec::new(),
                body: body.into(),
                surface: None,
            },
        )
        .await
        .unwrap()
}

/// A lorebook with one entry whose body is `body`.
async fn lorebook(e: &Env, owner: Snowflake, name: &str, body: &str) -> Snowflake {
    let book = e
        .lorebooks
        .create_lorebook(
            owner,
            CreateLorebook {
                name: name.into(),
                description: Some(format!("About {name}")),
                _preset: None,
            },
        )
        .await
        .unwrap();
    let entry = e.lorebooks.create_entry(book.id, owner).await.unwrap();
    let source = format!(
        "---\nid: {id}\nname: Castle\nkeywords: [castle]\npriority: 100\nslot: preamble\n---\n{body}",
        id = entry.entry_id
    );
    e.lorebooks
        .update_entry(book.id, entry.id, owner, source)
        .await
        .unwrap();
    book.id
}

/// A character using everything a card can carry.
async fn rich_character(e: &Env, owner: Snowflake) -> Snowflake {
    let roll = tool(e, owner, "roll", "{# return 4 #}").await;
    let flip = tool(e, owner, "flip", "{# return true #}").await;
    let gauge = e
        .widgets
        .create_widget(
            owner,
            CreateUserWidget {
                name: "Gauge".into(),
                description: "Shows a roll".into(),
                source: "<b>{{local:value}}</b>".into(),
                prompt: "Use roll.".into(),
                render: String::new(),
                tools: vec![roll],
                placement: Default::default(),
                mode: Default::default(),
                props: vec!["value".into()],
            },
        )
        .await
        .unwrap()
        .id;
    let world = lorebook(e, owner, "World", "The castle of {{char:name}}.").await;
    let secrets = lorebook(e, owner, "Secrets", "{# if {{local:x}} #}hidden{# endif #}").await;

    let character = e
        .characters
        .create_character(owner, "Nyx".into())
        .await
        .unwrap();
    let id = character.id;
    e.characters
        .patch_definition(
            id,
            owner,
            EditDefinition {
                description: Some(
                    "{{char:name}} watches {{user:name}}. {# if {{local:mood}} #}Smiles.{# endif #}"
                        .into(),
                ),
                personality: Some("Quiet, with {{random::a::b}} moods".into()),
                scenario: Some("A tower.".into()),
                template_injection: Some("{{template}} extra".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    e.characters
        .patch_metadata(
            id,
            owner,
            EditMetadata {
                display_name: Some(Some("Nyx the Grey".into())),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    e.characters
        .add_greeting(id, owner, "Hello, {{user:name}}.".into())
        .await
        .unwrap();
    e.characters
        .add_greeting(
            id,
            owner,
            "{# if {{local:night}} #}Evening.{# endif #}".into(),
        )
        .await
        .unwrap();
    e.characters
        .add_example(
            id,
            owner,
            "\n{{user:name}}: hi\n{{char:name}}: hello".into(),
        )
        .await
        .unwrap();
    e.characters
        .set_alt_names(id, owner, vec!["Grey".into()])
        .await
        .unwrap();
    for (kind, ids) in [
        (AttachmentKind::Lorebook, vec![world, secrets]),
        (AttachmentKind::Tool, vec![flip]),
        (AttachmentKind::Widget, vec![gauge]),
    ] {
        e.characters
            .set_attachments(id, owner, kind, ids)
            .await
            .unwrap();
    }
    id
}

async fn latest(e: &Env, id: Snowflake, owner: Snowflake) -> Character {
    let version = e.characters.get_latest_version_id(id).await.unwrap();
    e.characters.load_version(id, version, owner).await.unwrap()
}

async fn export_bytes(
    e: &Env,
    id: Snowflake,
    owner: Snowflake,
    profile: ExportProfile,
    format: ExportFormat,
) -> Vec<u8> {
    let bundle = CharacterBundle::load(e.storage.as_ref(), &e.characters, id, owner)
        .await
        .unwrap();
    export::render(&bundle, profile, format).unwrap().bytes
}

/// The parts of a character a round trip must keep, without ids.
fn comparable(c: &Character) -> Value {
    let d = &c.definition;
    let m = &c.metadata;
    let mut greetings: Vec<_> = d.greetings.iter().collect();
    greetings.sort_by_key(|g| g.order);
    let mut examples: Vec<_> = d.example_messages.iter().collect();
    examples.sort_by_key(|e| e.order);
    serde_json::json!({
        "name": d.name,
        "description": d.description,
        "personality": d.personality,
        "scenario": d.scenario,
        "system_prompt": d.system_prompt,
        "post_history_instructions": d.post_history_instructions,
        "template_injection": d.template_injection,
        "greetings": greetings.iter().map(|g| &g.text).collect::<Vec<_>>(),
        "examples": examples.iter().map(|e| &e.content).collect::<Vec<_>>(),
        "display_name": m.display_name,
        "alt_names": m.alt_names,
        "tags": m.tags,
        "creator": m.creator,
        "creator_notes": m.creator_notes,
        "card_version": m.card_version,
    })
}

async fn attachment_content(e: &Env, c: &Character) -> Value {
    let mut books = Vec::new();
    for id in &c.lorebook_ids {
        let b = e.storage.get_lorebook(*id).await.unwrap().unwrap();
        let entries: Vec<_> = b.entries.iter().map(|x| (&x.entry_id, &x.source)).collect();
        books.push(serde_json::json!([b.name, b.config_yaml, entries]));
    }
    let mut tools = Vec::new();
    for id in &c.tool_ids {
        let t = e.storage.get_tool(*id).await.unwrap().unwrap();
        tools.push(serde_json::json!([t.name, t.body]));
    }
    let mut widgets = Vec::new();
    for id in &c.widget_ids {
        let w = e.storage.get_widget(*id).await.unwrap().unwrap();
        let mut tool_names = Vec::new();
        for t in &w.tools {
            tool_names.push(e.storage.get_tool(*t).await.unwrap().unwrap().name);
        }
        widgets.push(serde_json::json!([w.name, w.source, w.props, tool_names]));
    }
    serde_json::json!({ "lorebooks": books, "tools": tools, "widgets": widgets })
}

#[tokio::test]
async fn every_profile_and_format_round_trips_exactly() {
    for e in both().await {
        let author = user(&e, "author").await;
        let original_id = rich_character(&e, author).await;
        let original = latest(&e, original_id, author).await;

        for profile in ExportProfile::ALL {
            for format in [ExportFormat::Png, ExportFormat::Json] {
                let reader = user(&e, &format!("reader-{profile:?}-{format:?}")).await;
                let bytes = export_bytes(&e, original_id, author, profile, format).await;
                let imported = e
                    .characters
                    .import_card(&bytes, reader, ImportOptions::default())
                    .await
                    .unwrap();
                let copy = latest(&e, imported.id, reader).await;

                assert_eq!(
                    comparable(&copy),
                    comparable(&original),
                    "{profile:?}/{format:?}"
                );
                assert_eq!(
                    attachment_content(&e, &copy).await,
                    attachment_content(&e, &original).await,
                    "{profile:?}/{format:?}"
                );
            }
        }
    }
}

#[tokio::test]
async fn importing_twice_reuses_tools_and_widgets_but_copies_lorebooks() {
    for e in both().await {
        let author = user(&e, "author").await;
        let id = rich_character(&e, author).await;
        let bytes = export_bytes(&e, id, author, ExportProfile::Lorealm, ExportFormat::Json).await;
        let reader = user(&e, "reader").await;

        let first = e
            .characters
            .import_card(&bytes, reader, ImportOptions::default())
            .await
            .unwrap();
        assert_eq!(first.attachments.reused, 0);

        let second = e
            .characters
            .import_card(&bytes, reader, ImportOptions::default())
            .await
            .unwrap();
        // Two tools and one widget reused; both lorebooks copied again.
        assert_eq!(second.attachments.reused, 3);
        assert_eq!(second.attachments.created, 2);
        let a = latest(&e, first.id, reader).await;
        let b = latest(&e, second.id, reader).await;
        assert_eq!(a.tool_ids, b.tool_ids);
        assert_eq!(a.widget_ids, b.widget_ids);
        assert_ne!(a.lorebook_ids, b.lorebook_ids);

        let third = e
            .characters
            .import_card(
                &bytes,
                reader,
                ImportOptions {
                    reuse_lorebooks: true,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(third.attachments.created, 0);
        assert_eq!(
            latest(&e, third.id, reader).await.lorebook_ids,
            a.lorebook_ids
        );
    }
}

#[tokio::test]
async fn a_different_tool_with_the_same_name_gets_renamed() {
    for e in both().await {
        let author = user(&e, "author").await;
        let id = rich_character(&e, author).await;
        let bytes = export_bytes(&e, id, author, ExportProfile::Lorealm, ExportFormat::Json).await;

        let reader = user(&e, "reader").await;
        tool(&e, reader, "roll", "{# return 6 #}").await;
        let imported = e
            .characters
            .import_card(&bytes, reader, ImportOptions::default())
            .await
            .unwrap();

        assert_eq!(
            imported.attachments.renamed,
            vec![("roll".to_string(), "roll_2".to_string())]
        );
        let copy = latest(&e, imported.id, reader).await;
        let widget = e
            .storage
            .get_widget(copy.widget_ids[0])
            .await
            .unwrap()
            .unwrap();
        let used = e.storage.get_tool(widget.tools[0]).await.unwrap().unwrap();
        assert_eq!(used.name, "roll_2");
        assert_eq!(used.body, "{# return 4 #}");
    }
}

#[tokio::test]
async fn a_compatibility_card_reads_like_a_sillytavern_card() {
    for e in both().await {
        let author = user(&e, "author").await;
        let id = rich_character(&e, author).await;
        let png = export_bytes(
            &e,
            id,
            author,
            ExportProfile::Compatibility,
            ExportFormat::Png,
        )
        .await;

        let v3: Value = serde_json::from_slice(
            &parse_character_card_png(&png, &["ccv3"])
                .unwrap()
                .json_bytes,
        )
        .unwrap();
        let v2: Value = serde_json::from_slice(
            &parse_character_card_png(&png, &["chara"])
                .unwrap()
                .json_bytes,
        )
        .unwrap();
        assert_eq!(v3["spec"], "chara_card_v3");
        assert_eq!(v2["spec"], "chara_card_v2");

        let data = &v3["data"];
        assert!(
            data["description"]
                .as_str()
                .unwrap()
                .starts_with("{{char}} watches {{user}}.")
        );
        assert_eq!(data["first_mes"], "Hello, {{user}}.");
        let book = &data["character_book"];
        assert_eq!(book["name"], "World");
        assert_eq!(book["entries"][0]["keys"][0], "castle");
        assert_eq!(book["entries"][0]["content"], "The castle of {{char}}.");
        assert_eq!(book["entries"][0]["position"], "before_char");
        assert!(v2["data"].get("group_only_greetings").is_none());

        let bundle = CharacterBundle::load(e.storage.as_ref(), &e.characters, id, author)
            .await
            .unwrap();
        let codes: Vec<_> =
            export::warnings(&bundle, ExportProfile::Compatibility, ExportFormat::Png)
                .into_iter()
                .map(|w| w.code)
                .collect();
        assert_eq!(
            codes,
            vec![
                "weaver_fields",
                "extra_lorebooks",
                "lorealm_only_attachments",
                "placeholder_avatar"
            ]
        );
    }
}

#[tokio::test]
async fn a_card_stripped_of_lorealm_data_still_imports_its_portable_parts() {
    for e in both().await {
        let author = user(&e, "author").await;
        let id = rich_character(&e, author).await;
        let json = export_bytes(
            &e,
            id,
            author,
            ExportProfile::Compatibility,
            ExportFormat::Json,
        )
        .await;

        // What a site that drops unknown extensions would hand back.
        let mut card: Value = serde_json::from_slice(&json).unwrap();
        card["data"]["extensions"]
            .as_object_mut()
            .unwrap()
            .remove("lorealm");
        let stripped = serde_json::to_vec(&card).unwrap();

        let reader = user(&e, "reader").await;
        let imported = e
            .characters
            .import_card(&stripped, reader, ImportOptions::default())
            .await
            .unwrap();
        let copy = latest(&e, imported.id, reader).await;
        let original = latest(&e, id, author).await;

        assert_eq!(copy.definition.scenario, original.definition.scenario);
        assert_eq!(copy.definition.description, original.definition.description);
        assert!(copy.tool_ids.is_empty());
        assert!(copy.widget_ids.is_empty());
        assert_eq!(
            copy.lorebook_ids.len(),
            1,
            "the main lorebook, via character_book"
        );
        assert!(copy.metadata.display_name.is_none());
    }
}
