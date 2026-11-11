use futures_util::StreamExt;
use lorealm_storage::{
    CharacterStorage, ConnectionStorage, LorebookStorage, PersonaStorage, TemplateStorage,
    TokenizerStorage, WeaverStateStorage,
    models::{
        AgentKind, ConversationMessage, Greeting, LorebookEntry, Snowflake, Tokenizer,
        VersionedSaveMode,
    },
};
use serde_json::{Value, json};

use crate::{
    errors::{ChatError, RealmError},
    services::{chat::TurnRequest, context_template::ContextNode},
    tests::Harness,
};

/// You, and a clerk who counts a coin every time it is their turn.
const COUNTING_HOUSE: &str = r##"
{
  name: "Counting house"
  agents: [
    {
      handle: "you"
      name: "You"
      kind: "player"
      grants: [{ channel: "world", access: "read" }]
    }
    {
      handle: "clerk"
      name: "Clerk"
      kind: "script"
      script: r#"$[inc_var("world:coins", 1)]$[emit_event("main", "A coin drops.", "system")]"#
    }
  ]
}"##;

/// Run one request to its end and hand back what the stream carried.
async fn turn(h: &Harness, convo: Snowflake, request: Value) -> Result<Vec<Value>, ChatError> {
    let request: TurnRequest = serde_json::from_value(request).unwrap();
    let mut stream = h.chat.turn(convo, h.user, request).await?;
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(serde_json::to_value(event?).unwrap());
    }
    Ok(events)
}

fn messages(events: &[Value]) -> Vec<ConversationMessage> {
    events
        .iter()
        .filter(|e| e["type"] == "message")
        .map(|e| serde_json::from_value(e["message"].clone()).unwrap())
        .collect()
}

async fn counting_house(h: &Harness, document: &str) -> Snowflake {
    let realm = h
        .realms
        .import(h.user, document, Default::default(), None)
        .await
        .expect("imports");
    h.chat
        .start_with_realm(h.user, realm.id, None)
        .await
        .expect("starts")
        .metadata
        .id
}

#[tokio::test]
async fn a_chat_with_a_character_is_a_cast_of_two_with_nothing_pinned() {
    let h = Harness::new().await;
    let me = h.persona(h.user, "Me").await;
    h.store.set_active_persona(h.user, me).await.unwrap();

    let sayuri = h.character(h.user, "Sayuri").await;
    let mut character = h.store.get_character(sayuri).await.unwrap().unwrap();
    let greeting = h.snow.generate();
    character.definition.greetings.push(Greeting {
        id: greeting,
        order: 0,
        text: "Welcome back.".into(),
    });
    h.store
        .save_character(&character, VersionedSaveMode::UpdateCurrent)
        .await
        .unwrap();

    let chat = h
        .chat
        .start_with_character(h.user, sayuri, None, greeting, String::new())
        .await
        .expect("starts");
    let convo = chat.metadata.id;

    let cast = h.chat.cast(convo, h.user).await.unwrap();
    let [you, her] = cast.agents.as_slice() else {
        panic!("expected two cast members, got {:?}", cast.agents);
    };
    assert_eq!(
        (you.kind.clone(), you.name.as_str()),
        (AgentKind::Player, "Me")
    );
    assert_eq!(her.kind, AgentKind::Llm);
    assert_eq!(her.character_id, Some(sayuri));
    // Left open, so each turn uses whatever is active when it runs.
    assert!(her.template_id.is_none() && her.profile_id.is_none() && her.persona_id.is_none());

    // The greeting is her turn, so the next one is yours.
    assert_eq!(chat.messages[0].content, "Welcome back.");
    assert_eq!(chat.messages[0].metadata["handle"], "char");
    assert_eq!(
        h.chat.next_agent(convo, h.user).await.unwrap().as_deref(),
        Some("user")
    );
}

#[tokio::test]
async fn one_request_commits_the_players_turn_and_runs_whoever_is_next() {
    let h = Harness::new().await;
    let convo = counting_house(&h, COUNTING_HOUSE).await;

    let events = turn(&h, convo, json!({ "content": "Count it." }))
        .await
        .unwrap();
    let kinds: Vec<&str> = events.iter().map(|e| e["type"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["message", "turn_started", "message", "done"]);

    let [mine, clerks] = messages(&events).try_into().unwrap();
    assert_eq!(mine.content, "Count it.");
    assert_eq!(mine.metadata["handle"], "you");
    assert_eq!(clerks.parent, Some(mine.id));
    assert_eq!(clerks.metadata["handle"], "clerk");
    // What the script announced travels with its message.
    assert_eq!(clerks.metadata["events"][0]["content"], "A coin drops.");

    assert_eq!(events.last().unwrap()["next"], "you");
    let world = h
        .chat
        .agent_state(convo, "you", h.user, None)
        .await
        .unwrap();
    assert_eq!(world["world"]["coins"], json!(1));
}

#[tokio::test]
async fn a_players_turn_can_be_rewritten_as_an_alternative() {
    let h = Harness::new().await;
    let convo = counting_house(&h, COUNTING_HOUSE).await;

    let first = messages(
        &turn(&h, convo, json!({ "content": "Count it." }))
            .await
            .unwrap(),
    );
    let events = turn(
        &h,
        convo,
        json!({ "content": "Count it twice.", "swipe_from": first[0].id }),
    )
    .await
    .unwrap();

    let [mine, clerks] = messages(&events).try_into().unwrap();
    assert_eq!(mine.content, "Count it twice.");
    assert_eq!(
        mine.parent, first[0].parent,
        "a sibling of the turn it replaces"
    );
    assert_eq!(clerks.parent, Some(mine.id));

    // The first take's coin is on the other branch.
    let world = h
        .chat
        .agent_state(convo, "you", h.user, None)
        .await
        .unwrap();
    assert_eq!(world["world"]["coins"], json!(1));
}

#[tokio::test]
async fn a_request_with_nothing_to_say_runs_only_whoever_is_next() {
    let h = Harness::new().await;
    let convo = counting_house(&h, COUNTING_HOUSE).await;

    // You are up first, so there is nothing to run yet.
    let events = turn(&h, convo, json!({})).await.unwrap();
    assert_eq!(events, [json!({ "type": "done", "next": "you" })]);
}

#[tokio::test]
async fn a_scripted_director_can_keep_a_player_waiting() {
    let h = Harness::new().await;
    let document = COUNTING_HOUSE.replacen(
        "agents: [",
        r#"director: { kind: "script", source: r"clerk" }
  agents: ["#,
        1,
    );
    let convo = counting_house(&h, &document).await;

    let err = turn(&h, convo, json!({ "content": "Me first." }))
        .await
        .unwrap_err();
    assert!(
        matches!(&err, ChatError::Realm(RealmError::NotYourTurn(next)) if next == "clerk"),
        "{err:?}"
    );
}

async fn stub_model(reply: &'static str) -> String {
    let sse = format!(
        "data: {{\"choices\":[{{\"delta\":{{\"content\":\"{reply}\"}},\"finish_reason\":\"stop\"}}]}}\n\ndata: [DONE]\n\n"
    );
    let endpoint = axum::Router::new().route(
        "/v1/chat/completions",
        axum::routing::post(move || async move {
            (
                [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                sse,
            )
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}/v1", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, endpoint).await.unwrap() });
    base_url
}

async fn byte_tokenizer(h: &Harness) -> Snowflake {
    use base64::Engine;
    let ranks: String = (0..=255u8)
        .map(|b| {
            let token = base64::engine::general_purpose::STANDARD.encode([b]);
            format!("{token} {b}\n")
        })
        .collect();
    let blob = h.blobs.put(ranks.as_bytes()).await.unwrap();
    let id = h.snow.generate();
    h.store
        .save_tokenizer(&Tokenizer {
            id,
            name: "bytes".into(),
            blob_uri: blob.to_uri(),
            owner_id: h.user,
            is_public: false,
        })
        .await
        .unwrap();
    id
}

async fn ready_for_models(h: &Harness, base_url: &str) {
    let me = h.persona(h.user, "Me").await;
    h.store.set_active_persona(h.user, me).await.unwrap();

    let template = h.template(h.user, "Lore only").await;
    let mut stored = h.store.get_template(template).await.unwrap().unwrap();
    stored.nodes_json = json!([ContextNode::source(
        &h.snow,
        "Lore",
        "",
        "world_info",
        "backdrop"
    )]);
    h.store.save_template(&stored).await.unwrap();
    h.store
        .set_active_template(h.user, Some(template))
        .await
        .unwrap();

    let tokenizer = byte_tokenizer(h).await;
    h.store
        .set_active_tokenizer(h.user, Some(tokenizer))
        .await
        .unwrap();

    let profile = h.profile(h.user, "local").await;
    let mut stored = h.store.get_profile(profile).await.unwrap().unwrap();
    stored.provider_params = json!({ "base_url": base_url });
    h.store.save_profile(&stored).await.unwrap();
    h.store
        .set_active_profile(h.user, Some(profile))
        .await
        .unwrap();
}

#[tokio::test]
async fn the_first_reply_of_a_chat_keeps_its_lorebook_state() {
    let h = Harness::new().await;
    ready_for_models(&h, &stub_model("It sleeps.").await).await;

    let book = h.lorebook(h.user, "Bestiary").await;
    let mut stored = h.store.get_lorebook(book).await.unwrap().unwrap();
    stored.entries.push(LorebookEntry {
        id: h.snow.generate(),
        lorebook_id: book,
        entry_id: "dragon".into(),
        source: "---\nid: dragon\nkeywords: [\"dragon\"]\nsticky_turns: 3\n---\nA dragon sleeps under the hill.\n".into(),
    });
    h.store.save_lorebook(&stored).await.unwrap();
    h.store
        .set_enabled_lorebooks(h.user, &[book])
        .await
        .unwrap();

    let sayuri = h.character(h.user, "Sayuri").await;
    let mut character = h.store.get_character(sayuri).await.unwrap().unwrap();
    let greeting = h.snow.generate();
    character.definition.greetings.push(Greeting {
        id: greeting,
        order: 0,
        text: "Welcome back.".into(),
    });
    h.store
        .save_character(&character, VersionedSaveMode::UpdateCurrent)
        .await
        .unwrap();
    let convo = h
        .chat
        .start_with_character(h.user, sayuri, None, greeting, String::new())
        .await
        .expect("starts")
        .metadata
        .id;

    let events = turn(&h, convo, json!({ "content": "Is there a dragon here?" }))
        .await
        .unwrap();
    let [_, reply] = messages(&events).try_into().unwrap();
    assert_eq!(reply.content, "It sleeps.");
    assert!(
        reply.metadata["context"]
            .to_string()
            .contains("A dragon sleeps"),
        "lorebook entry missing from the prompt: {}",
        reply.metadata["context"]
    );

    let state = h
        .store
        .get_message_state(reply.id)
        .await
        .unwrap()
        .expect("the first reply has a state row");
    let activation = state
        .activation
        .get(&book)
        .expect("the reply holds the book's activation");
    assert!(activation.last_activated.contains_key("dragon"));
    assert!(activation.sticky_remaining.contains_key("dragon"));
}

/// A lorebook entry reaches the user's modules the way a tool body does.
#[tokio::test(flavor = "multi_thread")]
async fn a_lorebook_entry_calls_a_module() {
    let h = Harness::new().await;
    ready_for_models(&h, &stub_model("It sleeps.").await).await;

    crate::services::module::ModuleService::new(
        std::sync::Arc::clone(&h.store) as std::sync::Arc<dyn lorealm_storage::Storage>,
        std::sync::Arc::clone(&h.snow),
    )
    .create(
        h.user,
        "lore".into(),
        String::new(),
        "export beast = \"wyrm\"\nexport proc loud(text: string)\n  {# return @[text.upper(text: {{input:text}})] #}\n{# endproc #}".into(),
    )
    .await
    .unwrap();

    let book = h.lorebook(h.user, "Bestiary").await;
    let mut stored = h.store.get_lorebook(book).await.unwrap().unwrap();
    stored.entries.push(LorebookEntry {
        id: h.snow.generate(),
        lorebook_id: book,
        entry_id: "dragon".into(),
        source:
            "---\nid: dragon\nkeywords: [\"dragon\"]\n---\nA @[lore.loud({{lore:beast}})] sleeps.\n"
                .into(),
    });
    h.store.save_lorebook(&stored).await.unwrap();
    h.store
        .set_enabled_lorebooks(h.user, &[book])
        .await
        .unwrap();

    let sayuri = h.character(h.user, "Sayuri").await;
    let mut character = h.store.get_character(sayuri).await.unwrap().unwrap();
    let greeting = h.snow.generate();
    character.definition.greetings.push(Greeting {
        id: greeting,
        order: 0,
        text: "Welcome back.".into(),
    });
    h.store
        .save_character(&character, VersionedSaveMode::UpdateCurrent)
        .await
        .unwrap();
    let convo = h
        .chat
        .start_with_character(h.user, sayuri, None, greeting, String::new())
        .await
        .expect("starts")
        .metadata
        .id;

    let events = turn(&h, convo, json!({ "content": "Is there a dragon here?" }))
        .await
        .unwrap();
    let [_, reply] = messages(&events).try_into().unwrap();
    assert!(
        reply.metadata["context"]
            .to_string()
            .contains("A WYRM sleeps"),
        "module output missing from the prompt: {}",
        reply.metadata["context"]
    );
}

/// A counting house whose clerk counts through a module's proc.
fn tallied_house(modules: &str) -> String {
    format!(
        r##"
{{
  name: "Tallied house"
  modules: {modules}
  agents: [
    {{ handle: "you", name: "You", kind: "player", grants: [{{ channel: "world", access: "read" }}] }}
    {{ handle: "clerk", name: "Clerk", kind: "script", script: r#"@[tally.bump(by: 5)]"# }}
  ]
}}"##
    )
}

async fn disabled_tally(h: &Harness) -> Snowflake {
    let modules = crate::services::module::ModuleService::new(
        std::sync::Arc::clone(&h.store) as std::sync::Arc<dyn lorealm_storage::Storage>,
        std::sync::Arc::clone(&h.snow),
    );
    let tally = modules
        .create(
            h.user,
            "tally".into(),
            String::new(),
            "export proc bump(by: number)\n  $[inc_var(\"world:coins\", {{input:by}})]\n{# endproc #}"
                .into(),
        )
        .await
        .unwrap();
    modules
        .update(tally.id, h.user, None, None, None, Some(false))
        .await
        .unwrap();
    tally.id
}

/// Imported modules run in the realm's turns even while disabled everywhere else.
#[tokio::test(flavor = "multi_thread")]
async fn a_realm_runs_the_modules_it_imports() {
    let h = Harness::new().await;
    let tally = disabled_tally(&h).await;
    let convo = counting_house(&h, &tallied_house(&format!("[\"{tally}\"]"))).await;

    turn(&h, convo, json!({ "content": "Count it." }))
        .await
        .unwrap();

    let world = h
        .chat
        .agent_state(convo, "you", h.user, None)
        .await
        .unwrap();
    assert_eq!(world["world"]["coins"], json!(5));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_disabled_module_a_realm_does_not_import_is_absent() {
    let h = Harness::new().await;
    disabled_tally(&h).await;
    let convo = counting_house(&h, &tallied_house("[]")).await;

    assert!(
        turn(&h, convo, json!({ "content": "Count it." }))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_realm_may_not_import_someone_elses_module() {
    let h = Harness::new().await;
    let theirs = crate::services::module::ModuleService::new(
        std::sync::Arc::clone(&h.store) as std::sync::Arc<dyn lorealm_storage::Storage>,
        std::sync::Arc::clone(&h.snow),
    )
    .create(h.stranger, "tally".into(), String::new(), String::new())
    .await
    .unwrap();

    let refused = h
        .realms
        .import(
            h.user,
            &tallied_house(&format!("[\"{}\"]", theirs.id)),
            Default::default(),
            None,
        )
        .await;

    assert!(
        matches!(refused, Err(RealmError::NotOwned(..))),
        "{refused:?}"
    );
}
