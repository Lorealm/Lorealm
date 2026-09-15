use std::collections::{HashMap, HashSet};

use lorealm_storage::models::{
    AgentFilter, AgentGrant, AgentKind, Director, RealmAgent, RealmDefinition, RealmResourceKind,
    RealmResourceRef, ScopePattern, Snowflake, is_identifier,
};
use weaver_lang::CompiledExpr;

use crate::{
    errors::{RealmError, TargetType},
    routes::CreateAgentRequest,
    services::{realm::RealmService, tools::builtin_tool},
    util::weaver::describe_parse_errors,
};

/// The identity bundle a validated create/update agent request resolves to.
pub(super) struct AgentParts {
    pub character_id: Option<Snowflake>,
    pub char_version_id: Option<Snowflake>,
    pub template_id: Option<Snowflake>,
    pub profile_id: Option<Snowflake>,
    pub user_id: Option<Snowflake>,
}

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

    pub(super) async fn validate_agent(
        &self,
        user_id: Snowflake,
        req: &CreateAgentRequest,
        cast: &[RealmAgent],
        exclude: Option<&str>,
    ) -> Result<AgentParts, RealmError> {
        validate_handle(&req.handle, cast, exclude)?;
        validate_properties(&req.properties)?;

        let parts = match req.kind {
            AgentKind::Player => {
                if req.character_id.is_some()
                    || req.char_version_id.is_some()
                    || req.template_id.is_some()
                    || !req.lorebooks.is_empty()
                {
                    return Err(RealmError::InvalidAgent(
                        "player agents cannot have character/template/lorebooks".into(),
                    ));
                }
                let profile_id = match req.profile_id {
                    Some(id) => {
                        self.connection.get_profile(user_id, id).await?;
                        Some(id)
                    }
                    None => None,
                };
                AgentParts {
                    character_id: None,
                    char_version_id: None,
                    template_id: None,
                    profile_id,
                    user_id: Some(user_id),
                }
            }
            AgentKind::Script => {
                if req.character_id.is_some()
                    || req.char_version_id.is_some()
                    || req.template_id.is_some()
                    || req.profile_id.is_some()
                    || req.persona_id.is_some()
                    || !req.lorebooks.is_empty()
                    || !req.tools.is_empty()
                    || !req.widgets.is_empty()
                {
                    return Err(RealmError::InvalidAgent(
                        "script agents cannot have character/template/profile/persona/lorebooks/tools/widgets"
                            .into(),
                    ));
                }
                if req.script.as_ref().is_none_or(|s| s.trim().is_empty()) {
                    return Err(RealmError::InvalidAgent(
                        "a script agent needs inline script content — the weaver logic it runs"
                            .into(),
                    ));
                }
                AgentParts {
                    character_id: None,
                    char_version_id: None,
                    template_id: None,
                    profile_id: None,
                    user_id: None,
                }
            }
            AgentKind::Llm => {
                // both-or-neither, validated for ownership when present
                let character = match (req.character_id, req.char_version_id) {
                    (Some(c), Some(v)) => {
                        self.characters.load_version(c, v, user_id).await?;
                        (Some(c), Some(v))
                    }
                    (None, None) => (None, None),
                    _ => {
                        return Err(RealmError::InvalidAgent(
                            "character_id and char_version_id must be set together".into(),
                        ));
                    }
                };

                let template_id = req.template_id.ok_or_else(|| {
                    RealmError::InvalidAgent("llm agents require an explicit template_id".into())
                })?;
                self.templates.get_template(template_id, user_id).await?;

                let profile_id = req.profile_id.ok_or_else(|| {
                    RealmError::InvalidAgent("llm agents require an explicit profile_id".into())
                })?;
                self.connection.get_profile(user_id, profile_id).await?;

                for &lorebook_id in &req.lorebooks {
                    self.validate_resource(user_id, RealmResourceKind::Lorebook, lorebook_id)
                        .await?;
                }

                AgentParts {
                    character_id: character.0,
                    char_version_id: character.1,
                    template_id: Some(template_id),
                    profile_id: Some(profile_id),
                    user_id: None,
                }
            }
        };

        if !matches!(req.kind, AgentKind::Script)
            && req.script.as_ref().is_some_and(|s| !s.trim().is_empty())
        {
            return Err(RealmError::InvalidAgent(
                "only script agents carry inline script content".into(),
            ));
        }

        if let Some(id) = req.persona_id {
            self.personas.get_persona(id, user_id).await?;
        }

        for &widget_id in &req.widgets {
            self.validate_resource(user_id, RealmResourceKind::Widget, widget_id)
                .await?;
        }

        // Builtins ship with the server and belong to nobody.
        for binding in &req.tools {
            if builtin_tool(binding.tool_id).is_some() {
                continue;
            }
            self.validate_resource(user_id, RealmResourceKind::Tool, binding.tool_id)
                .await?;
        }

        // A run_condition must compile now so the author sees the error at
        // edit time
        if let Some(src) = req.run_condition.as_ref().filter(|s| !s.trim().is_empty()) {
            CompiledExpr::compile(src).map_err(|errors| {
                RealmError::InvalidAgent(format!(
                    "run condition failed to compile: {}",
                    describe_parse_errors(src, &errors)
                ))
            })?;
        }

        validate_grants(&req.grants)?;
        validate_channel("speech_channel", req.speech_channel.as_deref())?;
        for binding in &req.tools {
            validate_channel("event_channel", binding.event_channel.as_deref())?;
        }

        Ok(parts)
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

/// A scenario needs a name, its seed script must compile and its overrides must
/// target current cast handles with valid property keys.
pub(super) fn validate_scenario(
    req: &crate::routes::CreateScenarioRequest,
    cast: &[RealmAgent],
) -> Result<(), RealmError> {
    if req.name.trim().is_empty() {
        return Err(RealmError::InvalidScenario(
            "a scenario needs a name".into(),
        ));
    }
    if !req.seed_script.trim().is_empty() {
        weaver_lang::CompiledTemplate::compile(&req.seed_script).map_err(|errors| {
            RealmError::InvalidScenario(format!(
                "seed script failed to compile: {}",
                describe_parse_errors(&req.seed_script, &errors)
            ))
        })?;
    }
    for (handle, props) in &req.agent_overrides {
        if !cast.iter().any(|a| &a.handle == handle) {
            return Err(RealmError::InvalidScenario(format!(
                "override targets unknown agent handle '{handle}'"
            )));
        }
        validate_properties(props)?;
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
                return Ok(());
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

/// A realm about to be played needs a director that can actually answer.
pub(super) fn validate_playable(director: &Director) -> Result<(), RealmError> {
    match director {
        Director::Script { source } if source.trim().is_empty() => Err(RealmError::DirectorFailed(
            "the script director has no body yet".into(),
        )),
        _ => Ok(()),
    }
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

pub(super) fn apply_turn_order(
    agents: &mut [RealmAgent],
    order: &[String],
) -> Result<(), RealmError> {
    if order.len() != agents.len() {
        return Err(RealmError::InvalidAgent(format!(
            "order lists {} agents, the cast has {}",
            order.len(),
            agents.len()
        )));
    }
    let positions: HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(i, handle)| (handle.as_str(), i))
        .collect();
    if positions.len() != order.len() {
        return Err(RealmError::InvalidAgent(
            "order contains duplicate handles".into(),
        ));
    }
    if agents
        .iter()
        .any(|a| !positions.contains_key(a.handle.as_str()))
    {
        return Err(RealmError::InvalidAgent(
            "order must list every cast member exactly once".into(),
        ));
    }

    agents.sort_by_key(|a| positions[a.handle.as_str()]);
    for (index, agent) in agents.iter_mut().enumerate() {
        agent.turn_order = index as i32;
    }
    Ok(())
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
            turn_order: 0,
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

    fn cast_of(handles: &[&str]) -> Vec<RealmAgent> {
        handles
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let mut a = agent(h);
                a.turn_order = i as i32;
                a
            })
            .collect()
    }

    fn order(handles: &[&str]) -> Vec<String> {
        handles.iter().map(|h| h.to_string()).collect()
    }

    #[test]
    fn reorder_applies_permutation_and_reindexes() {
        let mut agents = cast_of(&["a", "b", "c"]);
        apply_turn_order(&mut agents, &order(&["c", "a", "b"])).unwrap();
        let got: Vec<(&str, i32)> = agents
            .iter()
            .map(|a| (a.handle.as_str(), a.turn_order))
            .collect();
        assert_eq!(got, vec![("c", 0), ("a", 1), ("b", 2)]);
    }

    #[test]
    fn a_blank_script_director_saves_but_will_not_play() {
        let blank = Director::Script {
            source: String::new(),
        };
        assert!(validate_director(&blank, &[]).is_ok());
        assert!(validate_playable(&blank).is_err());

        let written = Director::Script {
            source: "alice".into(),
        };
        assert!(validate_director(&written, &[]).is_ok());
        assert!(validate_playable(&written).is_ok());
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

    #[test]
    fn reorder_rejects_non_permutations() {
        // wrong length
        let mut agents = cast_of(&["a", "b", "c"]);
        assert!(apply_turn_order(&mut agents, &order(&["a", "b"])).is_err());
        // duplicate handle
        let mut agents = cast_of(&["a", "b", "c"]);
        assert!(apply_turn_order(&mut agents, &order(&["a", "a", "b"])).is_err());
        // unknown handle in place of a real one
        let mut agents = cast_of(&["a", "b", "c"]);
        assert!(apply_turn_order(&mut agents, &order(&["a", "b", "nobody"])).is_err());
        // failures leave turn_order untouched
        assert_eq!(
            agents.iter().map(|a| a.turn_order).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }
}
