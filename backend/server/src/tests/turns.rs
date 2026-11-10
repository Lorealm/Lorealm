use futures_util::StreamExt;
use lorealm_storage::{
    CharacterStorage, PersonaStorage,
    models::{AgentKind, ConversationMessage, Greeting, Snowflake, VersionedSaveMode},
};
use serde_json::{Value, json};

use crate::{
    errors::{ChatError, RealmError},
    services::chat::TurnRequest,
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
        json!({ "content": "Count it twice.", "target": first[0].id }),
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
