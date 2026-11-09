//! AI-generated runtime smoke test for the SQLite backend.

use std::collections::HashMap;

use chrono::{Duration, Utc};
use lorealm_storage::backends::SqliteStorage;
use lorealm_storage::models::*;
use lorealm_storage::{
    ConversationStorage, PermissionStorage, PluginStorage, SessionStorage, StorageCore,
    UserStorage, UserToolStorage, WeaverStateStorage,
};

async fn db() -> SqliteStorage {
    let dir = std::env::temp_dir().join(format!("lorealm-sqlite-smoke-{}", uuid_like()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = SqliteStorage::new(dir.join("test.db"), 5, true)
        .await
        .unwrap();
    s.initialize().await.unwrap();
    s
}

fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    format!(
        "{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    )
}

async fn a_user(s: &SqliteStorage, n: i64) -> Snowflake {
    let id = Snowflake(7_300_000_000_000_000_000 + n);
    s.create_user(&CreateUser {
        id,
        username: format!("user{n}"),
        email: Some(format!("u{n}@example.com")),
        password_hash: "hash".into(),
    })
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn migrations_apply_and_backend_reports_available() {
    let s = db().await;
    assert!(s.is_available().await);
    assert_eq!(s.backend_type(), "sqlite");
}

#[tokio::test]
async fn user_roundtrip_preserves_snowflake_precision() {
    let s = db().await;
    // A snowflake well past 2^53, where a JSON/float round-trip would corrupt it.
    let id = Snowflake(7_318_349_284_733_321_217);
    s.create_user(&CreateUser {
        id,
        username: "precise".into(),
        email: None,
        password_hash: "h".into(),
    })
    .await
    .unwrap();

    let got = s.get_user_by_id(id).await.unwrap().expect("user");
    assert_eq!(got.id, id, "snowflake must survive the round trip exactly");
    assert_eq!(got.username, "precise");
    assert!(s.get_user_by_username("precise").await.unwrap().is_some());
}

/// The bug this guards against: `expires_at > CURRENT_TIMESTAMP` compares sqlx's
/// `2026-09-24T22:54:17.909+00:00` against SQLite's `2026-09-24 22:54:17`.
/// Because 'T' sorts above ' ', an expired session still matches and
/// authenticates. Both sides must use the same format.
#[tokio::test]
async fn expired_sessions_are_not_returned() {
    let s = db().await;
    let user = a_user(&s, 1).await;

    let live = CreateSession {
        id: Snowflake(1),
        user_id: user,
        token: "live-token".into(),
        csrf: "c".into(),
        ip_address: None,
        user_agent: None,
        expires_at: Utc::now() + Duration::hours(2),
    };
    let dead = CreateSession {
        id: Snowflake(2),
        user_id: user,
        token: "dead-token".into(),
        csrf: "c".into(),
        ip_address: None,
        user_agent: None,
        expires_at: Utc::now() - Duration::hours(2),
    };
    s.create_session(&live).await.unwrap();
    s.create_session(&dead).await.unwrap();

    assert!(
        s.get_session_by_token("live-token")
            .await
            .unwrap()
            .is_some(),
        "a session expiring in 2h must be returned"
    );
    assert!(
        s.get_session_by_token("dead-token")
            .await
            .unwrap()
            .is_none(),
        "a session that expired 2h ago must NOT be returned"
    );

    let listed = s.list_user_sessions(user).await.unwrap();
    assert_eq!(listed.len(), 1, "only the live session should be listed");
    assert_eq!(listed[0].token, "live-token");

    let purged = s.cleanup_expired().await.unwrap();
    assert_eq!(
        purged, 1,
        "cleanup should remove exactly the expired session"
    );
    assert!(
        s.get_session_by_token("live-token")
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn session_expiry_round_trips_through_text_storage() {
    let s = db().await;
    let user = a_user(&s, 2).await;
    let expires = Utc::now() + Duration::hours(5);

    s.create_session(&CreateSession {
        id: Snowflake(10),
        user_id: user,
        token: "t".into(),
        csrf: "c".into(),
        ip_address: Some("127.0.0.1".into()),
        user_agent: Some("agent".into()),
        expires_at: expires,
    })
    .await
    .unwrap();

    let got = s.get_session_by_token("t").await.unwrap().unwrap();
    assert_eq!(
        got.expires_at.timestamp_millis(),
        expires.timestamp_millis(),
        "timestamps must survive TEXT storage to millisecond precision"
    );
}

/// `revoke_session` writes `revoked_at = {NOW}`; the revoked row must then drop
/// out of `list_user_sessions`, which filters on `revoked_at IS NULL`.
#[tokio::test]
async fn revoking_a_session_hides_it_from_listings() {
    let s = db().await;
    let user = a_user(&s, 3).await;
    s.create_session(&CreateSession {
        id: Snowflake(20),
        user_id: user,
        token: "revoke-me".into(),
        csrf: "c".into(),
        ip_address: None,
        user_agent: None,
        expires_at: Utc::now() + Duration::hours(1),
    })
    .await
    .unwrap();

    assert_eq!(s.list_user_sessions(user).await.unwrap().len(), 1);
    s.revoke_session(Snowflake(20)).await.unwrap();
    assert_eq!(s.list_user_sessions(user).await.unwrap().len(), 0);
    assert!(
        s.revoke_session(Snowflake(20)).await.is_err(),
        "double revoke errors"
    );
}

#[tokio::test]
async fn csrf_counter_increments_and_resets() {
    let s = db().await;
    let user = a_user(&s, 4).await;
    s.create_session(&CreateSession {
        id: Snowflake(30),
        user_id: user,
        token: "csrf".into(),
        csrf: "old".into(),
        ip_address: None,
        user_agent: None,
        expires_at: Utc::now() + Duration::hours(1),
    })
    .await
    .unwrap();

    assert_eq!(s.increment_request_count(Snowflake(30)).await.unwrap(), 1);
    assert_eq!(s.increment_request_count(Snowflake(30)).await.unwrap(), 2);
    s.rotate_csrf_token(Snowflake(30), "new").await.unwrap();
    assert_eq!(
        s.increment_request_count(Snowflake(30)).await.unwrap(),
        1,
        "rotation resets the counter to 0"
    );
}

/// Exercises `json_each`, which replaces Postgres' `= ANY($n)` array bind.
#[tokio::test]
async fn batch_message_state_lookup_uses_json_each() {
    let s = db().await;
    let user = a_user(&s, 5).await;
    let convo = Snowflake(500);

    s.create_conversation(&Conversation {
        id: convo,
        user_id: user,
        character_id: None,
        char_version_id: None,
        realm_id: None,
        active_leaf_id: None,
        title: Some("t".into()),
        author_note: String::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
    .await
    .unwrap();

    // Ids deliberately above 2^53 to catch any float round-trip in json_each.
    let ids = [
        Snowflake(9_223_372_036_854_775_800),
        Snowflake(7_318_349_284_733_321_217),
        Snowflake(42),
    ];
    for (i, id) in ids.iter().enumerate() {
        s.add_message(&ConversationMessage {
            id: *id,
            conversation_id: convo,
            parent: None,
            role: ChatRole::User,
            content: format!("m{i}"),
            order: i as i32,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();

        s.put_message_state(&MessageState {
            message_id: *id,
            activation: HashMap::new(),
            persistent: StateDelta::default(),
        })
        .await
        .unwrap();
    }

    let got = s.get_message_states(&ids).await.unwrap();
    assert_eq!(got.len(), 3, "all three states should come back");
    for id in &ids {
        assert!(got.contains_key(id), "missing state for {id:?}");
    }

    // Empty input must behave like Postgres `= ANY('{}')`: no rows, no error.
    let empty = s.get_message_states(&[]).await.unwrap();
    assert!(empty.is_empty(), "empty id list must yield no rows");

    // A subset must return only that subset.
    let subset = s.get_message_states(&ids[..1]).await.unwrap();
    assert_eq!(subset.len(), 1);
}

/// Exercises `WITH RECURSIVE` and the deferred conversations <-> messages cycle.
#[tokio::test]
async fn recursive_active_path_walks_the_message_tree() {
    let s = db().await;
    let user = a_user(&s, 6).await;
    let convo = Snowflake(600);

    s.create_conversation(&Conversation {
        id: convo,
        user_id: user,
        character_id: None,
        char_version_id: None,
        realm_id: None,
        active_leaf_id: None,
        title: None,
        author_note: String::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
    .await
    .unwrap();

    // root -> a -> b
    let chain = [Snowflake(601), Snowflake(602), Snowflake(603)];
    let mut parent = None;
    for (i, id) in chain.iter().enumerate() {
        s.append_and_advance(&ConversationMessage {
            id: *id,
            conversation_id: convo,
            parent,
            role: if i % 2 == 0 {
                ChatRole::User
            } else {
                ChatRole::Assistant
            },
            content: format!("msg{i}"),
            order: 0,
            metadata: serde_json::json!({"i": i}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();
        parent = Some(*id);
    }

    let path = s.get_active_path(convo).await.unwrap();
    let ids: Vec<Snowflake> = path.iter().map(|m| m.id).collect();
    assert_eq!(ids, chain.to_vec(), "active path must be root-first");
    assert_eq!(path[0].content, "msg0");
    assert_eq!(path[2].content, "msg2");

    // metadata is JSON stored as TEXT; it must decode back to the same value
    assert_eq!(path[1].metadata, serde_json::json!({"i": 1}));

    let to_mid = s.get_path_to(convo, chain[1]).await.unwrap();
    assert_eq!(to_mid.len(), 2, "path to the middle node is root + itself");

    let listed = s.list_conversations(user).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].length, 3,
        "active-path length comes from the recursive CTE"
    );
}

/// The Postgres CHECK uses `name ~ '^[a-zA-Z0-9_-]{1,64}$'`; SQLite uses a
/// length bound plus a negated GLOB. Both must accept and reject the same names.
#[tokio::test]
async fn tool_name_check_matches_the_postgres_regex() {
    let s = db().await;
    let user = a_user(&s, 7).await;

    let tool = |name: &str, id: i64| UserTool {
        user_id: user,
        name: name.into(),
        id: Snowflake(id),
        display_name: None,
        description: String::new(),
        params: vec![],
        body: String::new(),
        surface: ToolSurface::default(),
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    for (i, ok) in ["roll_dice", "a", "A-Z_0-9", &"x".repeat(64)]
        .iter()
        .enumerate()
    {
        s.save_tool(tool(ok, 700 + i as i64))
            .await
            .unwrap_or_else(|e| panic!("{ok:?} should be accepted: {e}"));
    }

    for (i, bad) in ["", "has space", "dots.bad", "sym!", &"y".repeat(65)]
        .iter()
        .enumerate()
    {
        assert!(
            s.save_tool(tool(bad, 800 + i as i64)).await.is_err(),
            "{bad:?} should be rejected by the CHECK constraint"
        );
    }

    let listed = s.list_tools(user).await.unwrap();
    assert_eq!(listed.len(), 4);

    s.set_tool_enabled(Snowflake(700), false).await.unwrap();
    assert_eq!(s.list_enabled_tools(user).await.unwrap().len(), 3);
}

/// BYTEA -> BLOB. Includes bytes that are not valid UTF-8, which would break if
/// the column were treated as TEXT anywhere in the path.
#[tokio::test]
async fn plugin_data_round_trips_arbitrary_bytes() {
    let s = db().await;
    let user = a_user(&s, 8).await;

    let payload: Vec<u8> = vec![0x00, 0xff, 0xfe, 0x01, 0x80, b'\n', 0x7f];
    s.set_plugin_data(user, "tts", "voice", &payload)
        .await
        .unwrap();

    let got = s.get_plugin_data(user, "tts", "voice").await.unwrap();
    assert_eq!(
        got.as_deref(),
        Some(payload.as_slice()),
        "bytes must be unchanged"
    );

    // Overwrite via the ON CONFLICT path.
    let replacement = vec![0xde, 0xad, 0xbe, 0xef];
    s.set_plugin_data(user, "tts", "voice", &replacement)
        .await
        .unwrap();
    assert_eq!(
        s.get_plugin_data(user, "tts", "voice")
            .await
            .unwrap()
            .as_deref(),
        Some(replacement.as_slice()),
        "upsert must replace, not append or duplicate"
    );

    s.delete_plugin_data(user, "tts", "voice").await.unwrap();
    assert!(
        s.get_plugin_data(user, "tts", "voice")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn plugin_settings_and_version_upsert() {
    let s = db().await;
    let user = a_user(&s, 9).await;

    s.set_plugin_settings(user, "tts", &serde_json::json!({"rate": 1.5}))
        .await
        .unwrap();
    s.set_plugin_settings(user, "tts", &serde_json::json!({"rate": 2.0}))
        .await
        .unwrap();
    let got = s.get_plugin_settings(user, "tts").await.unwrap();
    assert_eq!(got, Some(serde_json::json!({"rate": 2.0})));

    s.set_plugin_data_version(user, "tts", "1.0.0")
        .await
        .unwrap();
    s.set_plugin_data_version(user, "tts", "1.1.0")
        .await
        .unwrap();
    assert_eq!(
        s.get_plugin_data_version(user, "tts")
            .await
            .unwrap()
            .as_deref(),
        Some("1.1.0")
    );
}

#[tokio::test]
async fn plugin_logs_keep_the_newest_cap_entries() {
    let s = db().await;
    let user = a_user(&s, 10).await;
    let log = |n: i64| PluginLog {
        id: Snowflake(n),
        user_id: user,
        handle: "tts".into(),
        level: PluginLogLevel::Info,
        source: PluginLogSource::Guest,
        message: format!("line {n}"),
        created_at: Utc::now(),
    };

    s.append_plugin_logs(user, "tts", &[log(1), log(2), log(3)], 2)
        .await
        .unwrap();
    s.append_plugin_logs(user, "tts", &[log(4)], 2)
        .await
        .unwrap();

    let kept: Vec<_> = s
        .list_plugin_logs(user, "tts", 10)
        .await
        .unwrap()
        .into_iter()
        .map(|l| l.message)
        .collect();
    assert_eq!(kept, ["line 4", "line 3"]);
}

/// `updated_at` ordering across rows written at different times. Mixed timestamp
/// formats in one column would silently misorder this.
#[tokio::test]
async fn conversation_listing_orders_by_recency() {
    let s = db().await;
    let user = a_user(&s, 10).await;

    for n in 0..3i64 {
        s.create_conversation(&Conversation {
            id: Snowflake(1000 + n),
            user_id: user,
            character_id: None,
            char_version_id: None,
            realm_id: None,
            active_leaf_id: None,
            title: Some(format!("c{n}")),
            author_note: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    // Touching the oldest must float it to the top of the listing.
    s.set_author_note(Snowflake(1000), "touched").await.unwrap();
    let listed = s.list_conversations(user).await.unwrap();
    assert_eq!(listed.len(), 3);
    assert_eq!(
        listed[0].id,
        Snowflake(1000),
        "most-recently-updated conversation must sort first"
    );
}

#[tokio::test]
async fn foreign_keys_are_enforced() {
    let s = db().await;
    // SQLite disables FK enforcement unless the connection opts in; if the
    // pragma were missing this insert would silently succeed.
    let orphan = s
        .create_conversation(&Conversation {
            id: Snowflake(9999),
            user_id: Snowflake(424_242),
            character_id: None,
            char_version_id: None,
            realm_id: None,
            active_leaf_id: None,
            title: None,
            author_note: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await;
    assert!(
        orphan.is_err(),
        "conversation for a nonexistent user must be rejected"
    );
}

#[tokio::test]
async fn cascade_delete_removes_dependent_rows() {
    let s = db().await;
    let user = a_user(&s, 11).await;
    let convo = Snowflake(1100);

    s.create_conversation(&Conversation {
        id: convo,
        user_id: user,
        character_id: None,
        char_version_id: None,
        realm_id: None,
        active_leaf_id: None,
        title: None,
        author_note: String::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
    .await
    .unwrap();

    s.append_and_advance(&ConversationMessage {
        id: Snowflake(1101),
        conversation_id: convo,
        parent: None,
        role: ChatRole::User,
        content: "hi".into(),
        order: 0,
        metadata: serde_json::json!({}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
    .await
    .unwrap();

    assert!(s.delete_conversation(convo).await.unwrap());
    assert!(
        s.get_message(Snowflake(1101)).await.unwrap().is_none(),
        "messages must cascade with their conversation"
    );
}

// ---------------------------------------------------------------------------
// The paths that diverge most from the Postgres source: json_group_array in
// place of ARRAY(), the -> / ->> JSON operators, json_type/json_array_length,
// and the deferred characters <-> character_versions FK cycle.
// ---------------------------------------------------------------------------

use lorealm_storage::{CharacterStorage, TemplateStorage};

fn a_character(id: i64, user: Snowflake, name: &str, tags: Vec<String>) -> Character {
    Character {
        id: Snowflake(id),
        user_id: user,
        current_version_id: Snowflake(id + 1_000_000),
        metadata: CharacterMetadata {
            display_name: Some(format!("{name} (display)")),
            alt_names: vec![],
            tags,
            creator: "tester".into(),
            creator_notes: String::new(),
            card_version: "1.2.0".into(),
            avatar_uri: Some("blob://avatar".into()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        definition: CharacterDefinition {
            name: name.into(),
            description: "desc".into(),
            personality: "p".into(),
            scenario: "s".into(),
            system_prompt: "sys".into(),
            post_history_instructions: "phi".into(),
            example_messages: vec![ExampleDialogue {
                id: Snowflake(id + 10),
                order: 0,
                content: "example".into(),
            }],
            greetings: vec![Greeting {
                id: Snowflake(id + 20),
                order: 0,
                text: "hello".into(),
            }],
            template_injection: "inj {{template}}".into(),
        },
        lorebook_ids: vec![],
        tool_ids: vec![],
        widget_ids: vec![],
        extensions: serde_json::json!({"x": 1}),
    }
}

/// `save_character` inserts the character and its version in one transaction,
/// which only works because the FK on `current_version_id` is
/// DEFERRABLE INITIALLY DEFERRED. Postgres needs that; so does SQLite.
#[tokio::test]
async fn saving_a_character_satisfies_the_deferred_fk_cycle() {
    let s = db().await;
    let user = a_user(&s, 20).await;

    let ch = a_character(2000, user, "Aria", vec!["red".into(), "blue".into()]);
    s.save_character(&ch, VersionedSaveMode::NewVersion)
        .await
        .expect("deferred FK cycle must commit");

    let got = s
        .get_character(Snowflake(2000))
        .await
        .unwrap()
        .expect("character");
    assert_eq!(got.definition.name, "Aria");
    assert_eq!(got.metadata.card_version, "1.2.0");
    assert_eq!(got.extensions, serde_json::json!({"x": 1}));
    assert_eq!(got.definition.greetings.len(), 1);
    assert_eq!(got.definition.greetings[0].text, "hello");
    assert_eq!(got.definition.example_messages[0].content, "example");
    assert_eq!(got.definition.template_injection, "inj {{template}}");
}

/// Exercises json_group_array (tags) plus the chained `->`/`->>` JSON operators
/// that read name/display_name/avatar_uri/card_version out of the snapshot.
#[tokio::test]
async fn character_listing_reads_tags_and_json_fields() {
    let s = db().await;
    let user = a_user(&s, 21).await;

    s.save_character(
        &a_character(2100, user, "Tagged", vec!["red".into(), "blue".into()]),
        VersionedSaveMode::NewVersion,
    )
    .await
    .unwrap();
    s.save_character(
        &a_character(2200, user, "Untagged", vec![]),
        VersionedSaveMode::NewVersion,
    )
    .await
    .unwrap();

    let list = s.list_characters(user).await.unwrap();
    assert_eq!(list.len(), 2);

    let tagged = list
        .iter()
        .find(|c| c.id == Snowflake(2100))
        .expect("tagged");
    let mut tags = tagged.tags.clone();
    tags.sort();
    assert_eq!(
        tags,
        vec!["blue".to_string(), "red".to_string()],
        "json_group_array tags"
    );

    // These four all come from `snapshot->'...'->>'...'`
    assert_eq!(tagged.name, "Tagged", "name via ->> operator");
    assert_eq!(tagged.display_name.as_deref(), Some("Tagged (display)"));
    assert_eq!(tagged.avatar_uri.as_deref(), Some("blob://avatar"));
    assert_eq!(tagged.card_version, "1.2.0");

    let untagged = list
        .iter()
        .find(|c| c.id == Snowflake(2200))
        .expect("untagged");
    assert!(
        untagged.tags.is_empty(),
        "a character with no tags must yield an empty vec, not NULL"
    );
}

#[tokio::test]
async fn character_versions_accumulate_and_restore() {
    let s = db().await;
    let user = a_user(&s, 22).await;

    let mut ch = a_character(2300, user, "V1", vec![]);
    s.save_character(&ch, VersionedSaveMode::NewVersion)
        .await
        .unwrap();

    ch.current_version_id = Snowflake(2300 + 2_000_000);
    ch.definition.name = "V2".into();
    s.save_character(&ch, VersionedSaveMode::NewVersion)
        .await
        .unwrap();

    let versions = s.list_character_versions(Snowflake(2300)).await.unwrap();
    assert_eq!(versions.len(), 2, "both versions must be listed");

    let current = s.get_character(Snowflake(2300)).await.unwrap().unwrap();
    assert_eq!(current.definition.name, "V2");

    let old = versions.iter().min_by_key(|v| v.version_num).unwrap();
    let at_old = s
        .get_character_at_version(Snowflake(2300), old.id)
        .await
        .unwrap()
        .expect("old version");
    assert_eq!(at_old.definition.name, "V1", "old snapshot must be intact");
}

/// The template listing's node_count reads `nodes_json` and counts nested
/// group children too.
#[tokio::test]
async fn template_listing_counts_nodes_via_json1() {
    let s = db().await;
    let user = a_user(&s, 23).await;

    let mk = |id: i64, name: &str, nodes: serde_json::Value| Template {
        id: Snowflake(id),
        owner_id: user,
        name: name.into(),
        description: "d".into(),
        nodes_json: nodes,
        version: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    s.save_template(&mk(
        2400,
        "three",
        serde_json::json!([{"a":1},{"b":2},{"c":3}]),
    ))
    .await
    .unwrap();
    s.save_template(&mk(
        2403,
        "nested",
        serde_json::json!([
            {"content": {"type": "group", "children": [
                {"content": {"type": "text"}},
                {"content": {"type": "group", "children": [{"content": {"type": "text"}}]}}
            ]}},
            {"content": {"type": "text"}}
        ]),
    ))
    .await
    .unwrap();
    s.save_template(&mk(2401, "empty", serde_json::json!([])))
        .await
        .unwrap();
    s.save_template(&mk(2402, "object", serde_json::json!({"not":"an array"})))
        .await
        .unwrap();

    let list = s.list_templates(user).await.unwrap();
    let by_id = |id: i64| {
        list.iter()
            .find(|t| t.id == Snowflake(id))
            .expect("template")
    };

    assert_eq!(by_id(2400).node_count, 3, "array of 3 -> node_count 3");
    assert_eq!(
        by_id(2403).node_count,
        5,
        "nested group children are counted"
    );
    assert_eq!(by_id(2401).node_count, 0, "empty array -> 0");
    assert_eq!(
        by_id(2402).node_count,
        0,
        "a JSON object is not an array, so node_count falls back to 0"
    );

    let fetched = s.get_template(Snowflake(2400)).await.unwrap().unwrap();
    assert_eq!(
        fetched.nodes_json,
        serde_json::json!([{"a":1},{"b":2},{"c":3}])
    );
    assert!(s.template_exists(Snowflake(2400)).await.unwrap());
    assert!(s.template_name_exists(user, "three", None).await.unwrap());
}

#[tokio::test]
async fn deleting_a_character_removes_versions_and_tags() {
    let s = db().await;
    let user = a_user(&s, 24).await;

    s.save_character(
        &a_character(2500, user, "Doomed", vec!["x".into()]),
        VersionedSaveMode::NewVersion,
    )
    .await
    .unwrap();

    assert!(s.delete_character(Snowflake(2500)).await.unwrap());
    assert!(s.get_character(Snowflake(2500)).await.unwrap().is_none());
    assert!(
        s.list_character_versions(Snowflake(2500))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(s.list_characters(user).await.unwrap().is_empty());
}

#[tokio::test]
async fn built_in_roles_are_seeded() {
    let s = db().await;
    let everyone = s.get_role(Snowflake(1)).await.unwrap().unwrap();
    assert_eq!(everyone.name, "@everyone");
    assert_eq!(everyone.permissions, 65547);
    let admin = s.get_role(Snowflake(2)).await.unwrap().unwrap();
    assert_eq!(admin.name, "Administrator");
    assert_eq!(admin.permissions, 4611686018427650048);
}

#[tokio::test]
async fn roles_round_trip_with_position_and_count_members() {
    let s = db().await;
    let role = s
        .create_role(&CreateRole {
            id: Snowflake(10),
            name: "moderator".into(),
            permissions: 5,
            position: 3,
        })
        .await
        .unwrap();
    assert_eq!(role.position, 3);

    let updated = s
        .update_role(
            role.id,
            &UpdateRole {
                position: Some(7),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!((updated.name.as_str(), updated.position), ("moderator", 7));

    assert_eq!(s.count_role_members(role.id).await.unwrap(), 0);
    let user = a_user(&s, 1).await;
    s.assign_role(user, role.id).await.unwrap();
    assert_eq!(s.count_role_members(role.id).await.unwrap(), 1);
    assert_eq!(s.get_user_roles(user).await.unwrap()[0].id, role.id);
}

#[tokio::test]
async fn role_members_lists_holders_oldest_first() {
    let s = db().await;
    let first = a_user(&s, 1).await;
    let second = a_user(&s, 2).await;
    let _outsider = a_user(&s, 3).await;
    s.assign_role(second, Snowflake(2)).await.unwrap();
    s.assign_role(first, Snowflake(2)).await.unwrap();

    let members: Vec<_> = s
        .get_role_members(Snowflake(2))
        .await
        .unwrap()
        .into_iter()
        .map(|u| u.id)
        .collect();
    assert_eq!(members, vec![first, second]);
}
