use lorealm_storage::models::{
    AgentFilter, AgentKind, ConversationMessage, Director, RealmAgent, RealmCast,
};
use rand::seq::SliceRandom;

use crate::{
    errors::RealmError,
    services::{plugins::PluginVocabulary, state::StatePool, tools::RealmVariables},
};

use super::weaver;

pub(super) fn handle_of(message: &ConversationMessage) -> Option<&str> {
    message.metadata.get("handle")?.as_str()
}

pub(super) struct Stage<'a> {
    director: Director,
    agents: &'a [RealmAgent],
    last: Option<&'a RealmAgent>,
    world: &'a StatePool,
    pub view: RealmVariables,
    plugins: Option<PluginVocabulary>,
}

impl<'a> Stage<'a> {
    pub fn new(cast: &'a RealmCast, path: &'a [ConversationMessage], world: &'a StatePool) -> Self {
        // Whoever produced the last message; `None` restarts the rotation.
        let last = path
            .last()
            .and_then(handle_of)
            .and_then(|h| cast.agents.iter().find(|a| a.handle == h));
        let view = weaver::build_realm_state(
            &cast.agents,
            &weaver::TurnInfo {
                index: path.iter().filter(|m| handle_of(m).is_some()).count(),
                last: last.zip(path.last().map(|m| m.content.as_str())),
            },
            cast.scenario.as_ref(),
            world,
        );

        Self {
            director: cast.director.clone().unwrap_or_default(),
            agents: &cast.agents,
            last,
            world,
            view,
            plugins: None,
        }
    }

    /// Lets directors and run conditions call the owner's plugins.
    pub fn with_plugins(mut self, plugins: Option<PluginVocabulary>) -> Self {
        self.plugins = plugins;
        self
    }

    /// Who the director offers this turn, in preference order.
    fn candidates(&self) -> Result<Vec<&'a RealmAgent>, RealmError> {
        let drawn = |over: &AgentFilter| -> Vec<&'a RealmAgent> {
            self.agents.iter().filter(|a| over.admits(a)).collect()
        };

        Ok(match &self.director {
            Director::RoundRobin { over } => {
                let pool = drawn(over);
                // Resume after whoever acted last
                let start = self
                    .last
                    .and_then(|l| pool.iter().position(|a| a.handle == l.handle))
                    .map_or(0, |i| i + 1);
                (0..pool.len())
                    .map(|k| pool[(start + k) % pool.len()])
                    .collect()
            }
            Director::Random { over } => {
                let mut pool = drawn(over);
                pool.shuffle(&mut rand::rng());
                pool
            }
            Director::Script { source } => {
                weaver::run_director(source, self.world, &self.view, self.plugins.as_ref())?
                    .into_iter()
                    .map(|handle| {
                        self.agents.iter().find(|a| a.handle == handle).ok_or(
                            RealmError::DirectorFailed(format!(
                                "no cast member holds the handle '{handle}'"
                            )),
                        )
                    })
                    .collect::<Result<_, _>>()?
            }
        })
    }

    /// A list of agents who's eligble to run next
    fn turn_candidates(&self) -> Result<Vec<&'a RealmAgent>, RealmError> {
        Ok(self
            .candidates()?
            .into_iter()
            .filter(|a| {
                weaver::run_condition_passes(a, self.world, &self.view, self.plugins.as_ref())
            })
            .collect())
    }

    /// The next agent to run. The first entry in the list of candidates.
    pub fn next_up(&self) -> Result<Option<&'a RealmAgent>, RealmError> {
        Ok(self.turn_candidates()?.into_iter().next())
    }

    pub fn player(&self) -> Result<&'a RealmAgent, RealmError> {
        let offered = self.turn_candidates()?;
        offered
            .iter()
            .find(|a| a.kind == AgentKind::Player)
            .copied()
            .ok_or_else(|| match offered.first() {
                Some(next) => RealmError::NotYourTurn(next.handle.clone()),
                None => RealmError::InvalidTurn("nobody can act right now".into()),
            })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use lorealm_storage::models::{AgentAccess, AgentGrant, ChatRole, Snowflake};
    use serde_json::json;

    fn agent(slot: i64, name: &str, kind: AgentKind, run_condition: Option<&str>) -> RealmAgent {
        RealmAgent {
            handle: format!("a{slot}"),
            name: name.into(),
            kind,
            character_id: None,
            char_version_id: None,
            template_id: None,
            profile_id: None,
            user_id: None,
            persona_id: None,
            properties: HashMap::new(),
            grants: vec![AgentGrant {
                channel: "state".into(),
                access: AgentAccess::ReadWrite,
            }],
            tools: Vec::new(),
            speech_channel: None,
            lorebooks: Vec::new(),
            widgets: Vec::new(),
            script: None,
            run_condition: run_condition.map(Into::into),
        }
    }

    fn cast() -> Vec<RealmAgent> {
        vec![
            agent(0, "Player", AgentKind::Player, None),
            agent(1, "Script", AgentKind::Script, None),
            agent(
                2,
                "Agent #1",
                AgentKind::Llm,
                Some("{{state:index}} == 0 || {{state:index}} == 1"),
            ),
            agent(
                3,
                "Agent #2",
                AgentKind::Llm,
                Some("{{state:index}} == 0 || {{state:index}} == 2"),
            ),
        ]
    }

    fn world(index: i64) -> StatePool {
        let mut w = StatePool::new();
        w.entry("state".into())
            .or_default()
            .insert("index".into(), json!(index));
        w
    }

    fn message(id: i64, agent: Option<&RealmAgent>) -> ConversationMessage {
        ConversationMessage {
            id: Snowflake(id),
            conversation_id: Snowflake(100),
            parent: None,
            role: ChatRole::Assistant,
            content: String::new(),
            order: id as i32,
            metadata: match agent {
                Some(a) => json!({ "handle": a.handle, "agent_name": a.name }),
                None => json!({}),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn casting(agents: Vec<RealmAgent>, director: Option<Director>) -> RealmCast {
        RealmCast {
            version_id: None,
            agents,
            director,
            scenario: None,
            config: Default::default(),
        }
    }

    fn next_after(
        agents: &[RealmAgent],
        director: Option<Director>,
        last_slot: Option<usize>,
        index: i64,
    ) -> Option<String> {
        let path: Vec<ConversationMessage> = last_slot
            .map(|slot| vec![message(0, Some(&agents[slot]))])
            .unwrap_or_default();
        let cast = casting(agents.to_vec(), director);
        let world = world(index);

        Stage::new(&cast, &path, &world)
            .next_up()
            .unwrap()
            .map(|a| a.name.clone())
    }

    #[test]
    fn an_empty_path_starts_at_the_top_of_the_cast() {
        assert_eq!(
            next_after(&cast(), None, None, 0).as_deref(),
            Some("Player")
        );
    }

    #[test]
    fn the_rotation_follows_the_last_actor_not_the_path_length() {
        let cast = cast();
        let path: Vec<ConversationMessage> = [0, 1, 2, 3, 0, 1, 2, 0]
            .iter()
            .enumerate()
            .map(|(i, slot)| message(i as i64, Some(&cast[*slot])))
            .collect();

        assert_eq!(path.len(), 8);
        let casting = casting(cast.clone(), None);
        let world = world(1);
        assert_eq!(
            Stage::new(&casting, &path, &world)
                .next_up()
                .unwrap()
                .map(|a| a.name.as_str()),
            Some("Script"),
        );
    }

    #[test]
    fn an_unattributable_last_turn_restarts_the_rotation() {
        let path = [message(0, None)];
        let casting = casting(cast(), None);
        let world = world(0);
        assert_eq!(
            Stage::new(&casting, &path, &world)
                .next_up()
                .unwrap()
                .map(|a| a.name.as_str()),
            Some("Player"),
        );
    }

    #[test]
    fn a_gated_agent_is_skipped_without_stalling_the_rotation() {
        let cast = cast();
        // After the script agent, #1 is ungated at index 1 and runs.
        assert_eq!(
            next_after(&cast, None, Some(1), 1).as_deref(),
            Some("Agent #1")
        );
        // After #1, #2 is gated at index 1, so the rotation wraps to the player.
        assert_eq!(
            next_after(&cast, None, Some(2), 1).as_deref(),
            Some("Player")
        );
    }

    #[test]
    fn skips_keep_the_group_chat_cycle_aligned() {
        let cast = cast();
        let next = |last_slot: usize, index: i64| next_after(&cast, None, Some(last_slot), index);

        // index 0: nobody is gated, so the whole cast speaks in order.
        assert_eq!(next(0, 0).as_deref(), Some("Script"));
        assert_eq!(next(1, 0).as_deref(), Some("Agent #1"));
        assert_eq!(next(2, 0).as_deref(), Some("Agent #2"));
        assert_eq!(next(3, 0).as_deref(), Some("Player"));

        // index 2: Agent #1 sits out and Agent #2 speaks in its place.
        assert_eq!(next(1, 2).as_deref(), Some("Agent #2"));
        assert_eq!(next(3, 2).as_deref(), Some("Player"));
    }

    #[test]
    fn no_runnable_agent_when_every_condition_is_false() {
        let llms = cast()[2..].to_vec();
        assert_eq!(next_after(&llms, None, None, 9), None);
    }

    #[test]
    fn a_filter_narrows_the_rotation_to_the_agents_it_admits() {
        let cast = cast();
        let llm_only = || {
            Some(Director::RoundRobin {
                over: AgentFilter::Kind {
                    kind: AgentKind::Llm,
                },
            })
        };

        assert_eq!(
            next_after(&cast, llm_only(), Some(2), 0).as_deref(),
            Some("Agent #2")
        );
        // Wrapping from the end of the pool comes back to its start.
        assert_eq!(
            next_after(&cast, llm_only(), Some(3), 0).as_deref(),
            Some("Agent #1")
        );
        // A last actor outside the pool resumes at the next member after it.
        assert_eq!(
            next_after(&cast, llm_only(), Some(1), 0).as_deref(),
            Some("Agent #1")
        );
    }

    #[test]
    fn a_script_director_names_who_acts() {
        let cast = cast();
        let pick = |source: &str| {
            next_after(
                &cast,
                Some(Director::Script {
                    source: source.into(),
                }),
                Some(0),
                0,
            )
        };

        assert_eq!(pick("a3").as_deref(), Some("Agent #2"));
        // An array is a preference order, and the first entry wins when its
        // run condition passes.
        assert_eq!(
            pick(r#"{# return ["a2", "a3"] #}"#).as_deref(),
            Some("Agent #1")
        );
        // Naming nobody is a legal answer: no one acts.
        assert_eq!(pick(""), None);
    }

    #[test]
    fn a_script_director_that_names_a_stranger_fails_loudly() {
        let casting = casting(
            cast(),
            Some(Director::Script {
                source: "nobody".into(),
            }),
        );
        let world = world(0);
        let err = Stage::new(&casting, &[], &world).next_up().unwrap_err();
        assert!(matches!(err, RealmError::DirectorFailed(_)), "{err}");
    }

    #[test]
    fn a_script_director_reads_the_realm_it_schedules() {
        let cast = cast();
        let source = r#"@[array.at(
            items: {{cast:llm.handles}},
            index: @[math.mod(a: {{turn:index}}, b: {{cast:llm.count}})]
        )]"#;

        let path: Vec<ConversationMessage> = vec![message(0, Some(&cast[0]))];
        let casting = casting(
            cast,
            Some(Director::Script {
                source: source.into(),
            }),
        );
        let world = world(0);
        // One turn on the path, two LLMs: 1 % 2 picks the second.
        assert_eq!(
            Stage::new(&casting, &path, &world)
                .next_up()
                .unwrap()
                .map(|a| a.name.as_str()),
            Some("Agent #2"),
        );
    }
}
