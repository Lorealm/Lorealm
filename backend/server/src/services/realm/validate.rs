use std::collections::{HashMap, HashSet};

use lorealm_storage::models::{
    AgentFilter, AgentGrant, AgentKind, Director, RealmAgent, RealmDefinition, RealmResourceKind,
    RealmResourceRef, ScopePattern, Snowflake, is_identifier,
};
use weaver_lang::CompiledExpr;

use crate::{
    errors::{RealmError, TargetType},
    services::{realm::RealmService, tools::builtin_tool},
    util::weaver::describe_parse_errors,
};

impl RealmService {
    /// The resource exists and the caller owns it.
    pub(super) async fn validate_resource(
        &self,
        user_id: Snowflake,
        kind: RealmResourceKind,
        id: Snowflake,
    ) -> Result<(), RealmError> {
        match kind {
            RealmResourceKind::Character => {
                let character = self
                    .storage
                    .get_character(id)
                    .await?
                    .ok_or(RealmError::NotFound(TargetType::Character, id))?;
                if character.user_id != user_id {
                    return Err(RealmError::NotOwned(TargetType::Character, id));
                }
            }
            RealmResourceKind::Lorebook => {
                let lorebook = self
                    .storage
                    .get_lorebook(id)
                    .await?
                    .ok_or(RealmError::NotFound(TargetType::Lorebook, id))?;
                if lorebook.owner_id != user_id {
                    return Err(RealmError::NotOwned(TargetType::Lorebook, id));
                }
            }
            RealmResourceKind::Template => {
                self.templates.get_template(id, user_id).await?;
            }
            RealmResourceKind::Tool => {
                let tool = self
                    .storage
                    .get_tool(id)
                    .await?
                    .ok_or(RealmError::NotFound(TargetType::UserTool, id))?;
                if tool.user_id != user_id {
                    return Err(RealmError::NotOwned(TargetType::UserTool, id));
                }
            }
            RealmResourceKind::Persona => {
                self.personas.get_persona(id, user_id).await?;
            }
            RealmResourceKind::ConnectionProfile => {
                self.connection.get_profile(user_id, id).await?;
            }
            RealmResourceKind::Widget => {
                let widget = self
                    .storage
                    .get_widget(id)
                    .await?
                    .ok_or(RealmError::NotFound(TargetType::UserWidget, id))?;
                if widget.user_id != user_id {
                    return Err(RealmError::NotOwned(TargetType::UserWidget, id));
                }
            }
        }
        Ok(())
    }
}

impl RealmService {
    /// Fill in the parts of a cast that a document has no way to author.
    pub(super) async fn bind_cast(
        &self,
        user_id: Snowflake,
        definition: &mut RealmDefinition,
    ) -> Result<(), RealmError> {
        for agent in &mut definition.agents {
            match agent.kind {
                AgentKind::Player => {
                    agent.user_id.get_or_insert(user_id);
                }
                AgentKind::Llm => {
                    if let (Some(character_id), None) = (agent.character_id, agent.char_version_id)
                    {
                        // Ownership first: the version lookup below does not
                        // check it, and ownership is the only gate a realm has.
                        self.validate_resource(user_id, RealmResourceKind::Character, character_id)
                            .await?;
                        agent.char_version_id =
                            Some(self.characters.get_latest_version_id(character_id).await?);
                    }
                }
                AgentKind::Script => {}
            }
        }
        Ok(())
    }

    /// Everything an expanded document must satisfy before it becomes a
    /// realm.
    pub(super) async fn validate_definition(
        &self,
        user_id: Snowflake,
        definition: &RealmDefinition,
    ) -> Result<(), RealmError> {
        for (i, agent) in definition.agents.iter().enumerate() {
            // Checked against the members before it, so no pair collides.
            validate_handle(&agent.handle, &definition.agents[..i], None)?;
            validate_properties(&agent.properties)?;
            validate_grants(&agent.grants)?;
            validate_channel("speech_channel", agent.speech_channel.as_deref())?;
            for binding in &agent.tools {
                validate_channel("event_channel", binding.event_channel.as_deref())?;
            }
            if let Some(src) = agent
                .run_condition
                .as_ref()
                .filter(|s| !s.trim().is_empty())
            {
                CompiledExpr::compile(src).map_err(|errors| {
                    RealmError::InvalidAgent(format!(
                        "run condition of '{}' failed to compile: {}",
                        agent.handle,
                        describe_parse_errors(src, &errors)
                    ))
                })?;
            }
            // The shapes that would otherwise fail at play time instead.
            match agent.kind {
                AgentKind::Llm
                    if agent.character_id.is_none() && agent.char_version_id.is_some() =>
                {
                    return Err(RealmError::InvalidAgent(format!(
                        "llm agent '{}' has a char_version_id but no character_id",
                        agent.handle
                    )));
                }
                AgentKind::Script if agent.script.as_ref().is_none_or(|s| s.trim().is_empty()) => {
                    return Err(RealmError::InvalidAgent(format!(
                        "script agent '{}' needs inline script content",
                        agent.handle
                    )));
                }
                _ => {}
            }
        }

        for resource in realm_resources(definition) {
            self.validate_resource(user_id, resource.kind, resource.resource_id)
                .await?;
        }

        if definition
            .scenarios
            .iter()
            .any(|s| s.name.trim().is_empty())
        {
            return Err(RealmError::InvalidScenario(
                "a scenario needs a name".into(),
            ));
        }

        if let Some(director) = &definition.director {
            validate_director(director, &definition.agents)?;
        }

        Ok(())
    }
}

pub(super) fn validate_handle(
    handle: &str,
    cast: &[RealmAgent],
    exclude: Option<&str>,
) -> Result<(), RealmError> {
    if !is_identifier(handle) {
        return Err(RealmError::InvalidAgent(format!(
            "handle '{handle}' must be a bare identifier ([A-Za-z_][A-Za-z0-9_]*)"
        )));
    }
    if cast
        .iter()
        .filter(|a| Some(a.handle.as_str()) != exclude)
        .any(|a| a.handle == handle)
    {
        return Err(RealmError::InvalidAgent(format!(
            "handle '{handle}' is already taken by another cast member"
        )));
    }
    Ok(())
}

pub(super) fn validate_properties(
    properties: &HashMap<String, serde_json::Value>,
) -> Result<(), RealmError> {
    for key in properties.keys() {
        if key.is_empty() || !key.split('.').all(is_identifier) {
            return Err(RealmError::InvalidAgent(format!(
                "property key '{key}' must be a dotted path of identifiers"
            )));
        }
    }
    Ok(())
}

/// A director must compile, and must be able to reach the cast it schedules.
pub(super) fn validate_director(
    director: &Director,
    cast: &[RealmAgent],
) -> Result<(), RealmError> {
    let over = match director {
        Director::Script { source } => {
            if source.trim().is_empty() {
                return Err(RealmError::DirectorFailed(
                    "the script director has no body".into(),
                ));
            }
            weaver_lang::CompiledTemplate::compile(source).map_err(|errors| {
                RealmError::DirectorFailed(format!(
                    "body failed to compile: {}",
                    describe_parse_errors(source, &errors)
                ))
            })?;
            return Ok(());
        }
        Director::RoundRobin { over } | Director::Random { over } => over,
    };

    if let AgentFilter::Handles { handles } = over {
        for handle in handles {
            if !cast.iter().any(|a| &a.handle == handle) {
                return Err(RealmError::DirectorFailed(format!(
                    "no cast member holds the handle '{handle}'"
                )));
            }
        }
    }

    if !cast.is_empty() && !cast.iter().any(|a| over.admits(a)) {
        return Err(RealmError::DirectorFailed(
            "this filter admits nobody in the cast".into(),
        ));
    }

    let players = cast.iter().filter(|a| a.kind == AgentKind::Player);
    if players.clone().count() > 0 && !players.clone().any(|a| over.admits(a)) {
        return Err(RealmError::DirectorFailed(
            "this filter excludes every player, so no one could ever take a turn".into(),
        ));
    }

    Ok(())
}

/// Grants must be weaver `scope[:name]` patterns ([`ScopePattern::validate`]).
pub(super) fn validate_grants(grants: &[AgentGrant]) -> Result<(), RealmError> {
    for g in grants {
        ScopePattern::validate(&g.channel).map_err(RealmError::InvalidAgent)?;
    }
    Ok(())
}

pub(super) fn validate_channel(field: &str, channel: Option<&str>) -> Result<(), RealmError> {
    match channel {
        None => Ok(()),
        Some(c) if is_identifier(c) => Ok(()),
        Some(c) => Err(RealmError::InvalidAgent(format!(
            "{field} '{c}' must be a bare identifier ([A-Za-z_][A-Za-z0-9_]*)"
        ))),
    }
}

/// Every owner-scoped resource one agent references.
pub(super) fn agent_resources(agent: &RealmAgent) -> Vec<RealmResourceRef> {
    let mut refs = Vec::new();
    let mut push = |kind, resource_id| refs.push(RealmResourceRef { kind, resource_id });

    if let Some(c) = agent.character_id {
        push(RealmResourceKind::Character, c);
    }
    if let Some(t) = agent.template_id {
        push(RealmResourceKind::Template, t);
    }
    if let Some(p) = agent.profile_id {
        push(RealmResourceKind::ConnectionProfile, p);
    }
    if let Some(p) = agent.persona_id {
        push(RealmResourceKind::Persona, p);
    }
    for binding in agent
        .tools
        .iter()
        .filter(|b| builtin_tool(b.tool_id).is_none())
    {
        push(RealmResourceKind::Tool, binding.tool_id);
    }
    for &l in &agent.lorebooks {
        push(RealmResourceKind::Lorebook, l);
    }
    for &w in &agent.widgets {
        push(RealmResourceKind::Widget, w);
    }
    refs
}

/// Everything the realm's cast references, deduplicated, in cast order.
pub(super) fn realm_resources(definition: &RealmDefinition) -> Vec<RealmResourceRef> {
    let mut seen = HashSet::new();
    definition
        .agents
        .iter()
        .flat_map(agent_resources)
        .filter(|r| seen.insert(*r))
        .collect()
}

#[cfg(test)]
mod tests {
    use lorealm_storage::models::AgentToolBinding;

    use super::*;

    fn sf(n: i64) -> Snowflake {
        Snowflake(n)
    }

    fn agent(name: &str) -> RealmAgent {
        RealmAgent {
            handle: name.into(),
            name: name.into(),
            kind: AgentKind::Llm,
            character_id: Some(sf(10)),
            char_version_id: Some(sf(11)),
            template_id: Some(sf(20)),
            profile_id: Some(sf(30)),
            user_id: None,
            persona_id: Some(sf(60)),
            properties: HashMap::new(),
            grants: Vec::new(),
            tools: vec![AgentToolBinding {
                tool_id: sf(40),
                event_channel: None,
            }],
            speech_channel: None,
            lorebooks: vec![sf(50)],
            widgets: Vec::new(),
            script: None,
            run_condition: None,
        }
    }

    fn res(kind: RealmResourceKind, id: i64) -> RealmResourceRef {
        RealmResourceRef {
            kind,
            resource_id: sf(id),
        }
    }

    #[test]
    fn inference_covers_every_reference_kind() {
        assert_eq!(
            agent_resources(&agent("spymaster")),
            vec![
                res(RealmResourceKind::Character, 10),
                res(RealmResourceKind::Template, 20),
                res(RealmResourceKind::ConnectionProfile, 30),
                res(RealmResourceKind::Persona, 60),
                res(RealmResourceKind::Tool, 40),
                res(RealmResourceKind::Lorebook, 50),
            ]
        );
    }

    #[test]
    fn player_agents_reference_only_their_tools() {
        let mut player = agent("you");
        player.kind = AgentKind::Player;
        player.character_id = None;
        player.char_version_id = None;
        player.template_id = None;
        player.profile_id = None;
        player.persona_id = None;
        player.lorebooks.clear();

        assert_eq!(
            agent_resources(&player),
            vec![res(RealmResourceKind::Tool, 40)]
        );
    }

    #[test]
    fn player_agents_may_reference_a_connection_profile() {
        let mut player = agent("you");
        player.kind = AgentKind::Player;
        player.character_id = None;
        player.char_version_id = None;
        player.template_id = None;
        player.persona_id = None;
        player.lorebooks.clear();
        // profile_id stays Some(sf(30)) from the fixture

        assert_eq!(
            agent_resources(&player),
            vec![
                res(RealmResourceKind::ConnectionProfile, 30),
                res(RealmResourceKind::Tool, 40),
            ]
        );
    }

    #[test]
    fn script_agents_reference_nothing() {
        let mut script = agent("log-keeper");
        script.kind = AgentKind::Script;
        script.character_id = None;
        script.char_version_id = None;
        script.template_id = None;
        script.profile_id = None;
        script.persona_id = None;
        script.tools.clear();
        script.lorebooks.clear();
        script.script = Some("$[set_var(\"log:events\", [])]".into());

        assert_eq!(agent_resources(&script), vec![]);
    }

    #[test]
    fn a_kind_bounds_a_reference() {
        let mut a = agent("a");
        a.tools = vec![AgentToolBinding {
            tool_id: sf(50),
            event_channel: None,
        }];
        a.lorebooks = vec![sf(50)];
        a.character_id = None;
        a.char_version_id = None;
        a.template_id = None;
        a.profile_id = None;
        a.persona_id = None;

        assert_eq!(
            agent_resources(&a),
            vec![
                res(RealmResourceKind::Tool, 50),
                res(RealmResourceKind::Lorebook, 50),
            ]
        );
    }

    #[test]
    fn a_realm_deduplicates_what_its_cast_shares() {
        let definition = RealmDefinition {
            agents: vec![agent("one"), agent("two")],
            ..Default::default()
        };

        assert_eq!(
            realm_resources(&definition),
            vec![
                res(RealmResourceKind::Character, 10),
                res(RealmResourceKind::Template, 20),
                res(RealmResourceKind::ConnectionProfile, 30),
                res(RealmResourceKind::Persona, 60),
                res(RealmResourceKind::Tool, 40),
                res(RealmResourceKind::Lorebook, 50),
            ]
        );
    }

    #[test]
    fn a_blank_script_director_is_rejected() {
        let blank = Director::Script {
            source: String::new(),
        };
        assert!(validate_director(&blank, &[]).is_err());

        let written = Director::Script {
            source: "alice".into(),
        };
        assert!(validate_director(&written, &[]).is_ok());
    }

    #[test]
    fn a_director_body_that_does_not_compile_is_rejected_on_save() {
        let broken = Director::Script {
            source: "{# if #}".into(),
        };
        assert!(validate_director(&broken, &[]).is_err());
    }

    #[test]
    fn a_filter_may_not_starve_the_players() {
        let mut player = agent("you");
        player.kind = AgentKind::Player;
        let cast = vec![agent("alice"), player];

        let llm_only = Director::RoundRobin {
            over: AgentFilter::Kind {
                kind: AgentKind::Llm,
            },
        };
        assert!(validate_director(&llm_only, &cast).is_err());
        // The same filter is fine when there is no player to strand.
        assert!(validate_director(&llm_only, &[agent("alice")]).is_ok());
    }
}
