use lorealm_storage::models::{
    AGENTS_SCOPE, ConversationMessage, EventKind, MAIN_CHANNEL, RealmAgent, RealmCast,
    ResolvedRealmConfig, TurnEvent,
};

use crate::services::{
    chat::WidgetRenderer,
    context_template::{ContentRef, PromptRole, PromptTurn},
    state::StatePool,
};

use super::stage::handle_of;

fn agent_channels(world: &StatePool, handle: &str) -> Vec<String> {
    match world
        .get(AGENTS_SCOPE)
        .and_then(|agents| agents.get(handle))
        .and_then(|agent| agent.get("channels"))
    {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec![MAIN_CHANNEL.to_string()],
    }
}

fn channel_observers(
    cast: &RealmCast,
    world: &StatePool,
    speaker: &str,
    on: &[String],
) -> Vec<String> {
    cast.agents
        .iter()
        .filter(|a| {
            a.handle == speaker
                || agent_channels(world, &a.handle)
                    .iter()
                    .any(|c| on.contains(c))
        })
        .map(|a| a.handle.clone())
        .collect()
}

pub(super) fn observers(cast: &RealmCast, world: &StatePool, speaker: &str) -> Vec<String> {
    channel_observers(cast, world, speaker, &agent_channels(world, speaker))
}

pub(super) fn observe_events(
    cast: &RealmCast,
    world: &StatePool,
    speaker: &str,
    events: &mut [TurnEvent],
) {
    for event in events {
        let on = [event.channel.clone()];
        event.observers = Some(channel_observers(cast, world, speaker, &on));
    }
}

fn hears(viewer: &RealmAgent, observers: Option<&[String]>, channel: &str) -> bool {
    match observers {
        Some(observers) => observers.contains(&viewer.handle),
        None => channel == MAIN_CHANNEL,
    }
}

/// A list stored under `key` in a message's metadata.
fn stored<T: serde::de::DeserializeOwned>(message: &ConversationMessage, key: &str) -> Vec<T> {
    message
        .metadata
        .get(key)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

fn verbatim(role: PromptRole, content: String) -> PromptTurn {
    PromptTurn {
        role,
        content,
        content_ref: ContentRef::Verbatim,
    }
}

/// The path as `viewer` perceives it, oldest first.
pub(super) fn perceive(
    cast: &RealmCast,
    viewer: &RealmAgent,
    path: &[ConversationMessage],
    widgets: &mut WidgetRenderer,
) -> Vec<PromptTurn> {
    let config = ResolvedRealmConfig::from_config(&cast.config);

    let mut turns: Vec<Vec<PromptTurn>> = path
        .iter()
        .rev()
        .map(|message| {
            let speaker = handle_of(message);
            let own = speaker == Some(viewer.handle.as_str());
            let name = message.metadata["agent_name"].as_str().unwrap_or_default();
            let channel = message.metadata["channel"].as_str().unwrap_or(MAIN_CHANNEL);
            let observers: Option<Vec<String>> = message
                .metadata
                .get("observers")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let mut heard = Vec::new();
            if !message.content.trim().is_empty()
                && (own || hears(viewer, observers.as_deref(), channel))
            {
                let mut turn = PromptTurn::from_message(message, widgets);
                if own {
                    turn.role = PromptRole::Assistant;
                } else if speaker.is_some() {
                    turn.role = PromptRole::User;
                    let said = config.speech(name, &turn.content);
                    if said != turn.content {
                        turn = verbatim(PromptRole::User, said);
                    }
                }
                heard.push(turn);
            }

            for event in stored::<TurnEvent>(message, "events") {
                if !own && !hears(viewer, event.observers.as_deref(), &event.channel) {
                    continue;
                }
                heard.push(match event.kind {
                    EventKind::Spoke if own => verbatim(PromptRole::Assistant, event.content),
                    EventKind::Spoke => {
                        verbatim(PromptRole::User, config.speech(name, &event.content))
                    }
                    _ => verbatim(PromptRole::User, event.content),
                });
            }

            if own {
                heard.extend(
                    stored::<String>(message, "tool_results")
                        .into_iter()
                        .map(|note| verbatim(PromptRole::User, note)),
                );
            }
            heard
        })
        .collect();

    turns.reverse();
    turns.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use lorealm_storage::models::{
        AgentAccess, AgentGrant, AgentKind, ChatRole, RealmConfig, Snowflake,
    };
    use serde_json::json;

    use super::*;

    fn agent(handle: &str, kind: AgentKind, hears: &[&str]) -> RealmAgent {
        RealmAgent {
            handle: handle.into(),
            name: handle.to_uppercase(),
            kind,
            character_id: None,
            char_version_id: None,
            template_id: None,
            profile_id: None,
            persona_id: None,
            user_id: None,
            properties: HashMap::new(),
            grants: hears
                .iter()
                .map(|channel| AgentGrant {
                    channel: channel.to_string(),
                    access: AgentAccess::Read,
                })
                .collect(),
            tools: Vec::new(),
            lorebooks: Vec::new(),
            widgets: Vec::new(),
            script: None,
            run_condition: None,
        }
    }

    fn cast(agents: Vec<RealmAgent>, speech_format: Option<&str>) -> RealmCast {
        RealmCast {
            version_id: None,
            agents,
            director: None,
            scenario: None,
            config: RealmConfig {
                speech_format: speech_format.map(Into::into),
                ..Default::default()
            },
            modules: Vec::new(),
        }
    }

    fn message(
        id: i64,
        role: ChatRole,
        content: &str,
        metadata: serde_json::Value,
    ) -> ConversationMessage {
        ConversationMessage {
            id: Snowflake(id),
            conversation_id: Snowflake(100),
            parent: None,
            role,
            content: content.into(),
            order: 0,
            metadata,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn said(id: i64, by: &RealmAgent, content: &str) -> ConversationMessage {
        let role = match by.kind {
            AgentKind::Player => ChatRole::User,
            _ => ChatRole::Assistant,
        };
        message(
            id,
            role,
            content,
            json!({ "handle": by.handle, "agent_name": by.name }),
        )
    }

    fn seen(
        cast: &RealmCast,
        viewer: &RealmAgent,
        path: &[ConversationMessage],
    ) -> Vec<(PromptRole, String)> {
        perceive(cast, viewer, path, &mut WidgetRenderer::default())
            .into_iter()
            .map(|t| (t.role, t.content))
            .collect()
    }

    #[test]
    fn a_chat_with_one_character_reads_as_plain_user_and_assistant_turns() {
        let you = agent("user", AgentKind::Player, &["*"]);
        let her = agent("char", AgentKind::Llm, &["*"]);
        let cast = cast(vec![you.clone(), her.clone()], Some("{content}"));
        let path = [
            said(1, &her, "Welcome back."),
            said(2, &you, "I missed this place."),
        ];

        let turns = perceive(&cast, &her, &path, &mut WidgetRenderer::default());
        assert_eq!(
            turns
                .iter()
                .map(|t| (t.role.clone(), t.content.as_str()))
                .collect::<Vec<_>>(),
            [
                (PromptRole::Assistant, "Welcome back."),
                (PromptRole::User, "I missed this place."),
            ]
        );
        // Untouched text stays a reference to its message, not a copy.
        assert!(
            turns
                .iter()
                .all(|t| matches!(t.content_ref, ContentRef::Message(_)))
        );
    }

    #[test]
    fn others_are_named_and_only_heard_by_their_observers() {
        let ito = agent("ito", AgentKind::Llm, &[]);
        let mori = agent("mori", AgentKind::Llm, &[]);
        let courtier = agent("courtier", AgentKind::Llm, &[]);
        let cast = cast(vec![ito.clone(), mori.clone(), courtier.clone()], None);

        let mut whisper = said(2, &mori, "The treasury is empty.");
        whisper.metadata["observers"] = json!(["mori", "courtier"]);
        let path = [said(1, &ito, "Good morning."), whisper];

        assert_eq!(
            seen(&cast, &ito, &path),
            [(PromptRole::Assistant, "Good morning.".to_string())]
        );
        assert_eq!(
            seen(&cast, &courtier, &path),
            [
                (PromptRole::User, "ITO: Good morning.".to_string()),
                (PromptRole::User, "MORI: The treasury is empty.".to_string()),
            ]
        );
    }

    #[test]
    fn observers_are_whoever_shares_a_channel_and_main_is_the_default() {
        let names = ["ito", "mori", "takeda", "hermit"];
        let cast = cast(
            names
                .iter()
                .map(|h| agent(h, AgentKind::Llm, &[]))
                .collect(),
            None,
        );
        let world: StatePool = HashMap::from([(
            AGENTS_SCOPE.to_string(),
            HashMap::from([
                (
                    "mori".to_string(),
                    json!({ "channels": ["cell_1", "cell_2"] }),
                ),
                ("takeda".to_string(), json!({ "channels": ["cell_2"] })),
                ("hermit".to_string(), json!({ "channels": [] })),
            ]),
        )]);

        // ito has no list, so is on main alone.
        assert_eq!(observers(&cast, &world, "ito"), ["ito"]);
        assert_eq!(observers(&cast, &world, "takeda"), ["mori", "takeda"]);
        assert_eq!(observers(&cast, &world, "hermit"), ["hermit"]);

        let mut events = vec![TurnEvent {
            channel: "main".into(),
            kind: EventKind::System,
            content: "Night falls.".into(),
            observers: None,
        }];
        observe_events(&cast, &world, "hermit", &mut events);
        assert_eq!(
            events[0].observers.as_deref(),
            Some(&["ito".to_string(), "hermit".to_string()][..])
        );
    }

    #[test]
    fn an_opening_nobody_said_reaches_everyone_as_written() {
        let ito = agent("ito", AgentKind::Llm, &[]);
        let cast = cast(vec![ito.clone()], None);
        let path = [message(
            1,
            ChatRole::System,
            "The thaw comes early.",
            json!({}),
        )];

        assert_eq!(
            seen(&cast, &ito, &path),
            [(PromptRole::System, "The thaw comes early.".to_string())]
        );
    }

    #[test]
    fn events_follow_their_channel_and_rool_results_stay_with_whoever_acted() {
        let ito = agent("ito", AgentKind::Llm, &[]);
        let mori = agent("mori", AgentKind::Llm, &[]);
        let cast = cast(vec![ito.clone(), mori.clone()], None);

        let mut acted = said(1, &ito, "");
        acted.metadata["events"] = json!([
            { "channel": "main", "kind": "system", "content": "Night falls." },
            { "channel": "", "kind": "acted", "content": "[ITO used hire_spy]" },
        ]);
        acted.metadata["tool_results"] = json!(["[hire_spy result: a shinobi slips out]"]);
        let path = [acted];

        assert_eq!(
            seen(&cast, &ito, &path),
            [
                (PromptRole::User, "Night falls.".to_string()),
                (PromptRole::User, "[ITO used hire_spy]".to_string()),
                (
                    PromptRole::User,
                    "[hire_spy result: a shinobi slips out]".to_string()
                ),
            ]
        );
        assert_eq!(
            seen(&cast, &mori, &path),
            [(PromptRole::User, "Night falls.".to_string())]
        );
    }
}
