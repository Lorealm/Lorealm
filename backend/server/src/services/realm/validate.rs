use std::collections::HashMap;

use lorealm_storage::models::{
    AgentFilter, AgentGrant, AgentKind, Director, ImportKind, ImportRule, RealmAgent, ScopePattern,
    Snowflake, is_identifier,
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
    /// One import rule points at an existing resource the caller owns.
    pub(super) async fn validate_import(
        &self,
        user_id: Snowflake,
        rule: &ImportRule,
    ) -> Result<(), RealmError> {
        let id = rule.resource_id;
        match rule.kind {
            ImportKind::Character => {
                let character = self
                    .storage
                    .get_character(id)
                    .await?
                    .ok_or(RealmError::NotFound(TargetType::Character, id))?;
                if character.user_id != user_id {
                    return Err(RealmError::NotOwned(TargetType::Character, id));
                }
            }
            ImportKind::Lorebook => {
                let lorebook = self
                    .storage
                    .get_lorebook(id)
                    .await?
                    .ok_or(RealmError::NotFound(TargetType::Lorebook, id))?;
                if lorebook.owner_id != user_id {
                    return Err(RealmError::NotOwned(TargetType::Lorebook, id));
                }
            }
            ImportKind::Template => {
                self.templates.get_template(id, user_id).await?;
            }
            ImportKind::Tool => {
                let tool = self
                    .storage
                    .get_tool(id)
                    .await?
                    .ok_or(RealmError::NotFound(TargetType::UserTool, id))?;
                if tool.user_id != user_id {
                    return Err(RealmError::NotOwned(TargetType::UserTool, id));
                }
            }
            ImportKind::Persona => {
                self.personas.get_persona(id, user_id).await?;
            }
            ImportKind::ConnectionProfile => {
                self.connection.get_profile(user_id, id).await?;
            }
            ImportKind::Widget => {
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
        imports: &[ImportRule],
        cast: &[RealmAgent],
        exclude: Option<Snowflake>,
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
                        ensure_imported(imports, ImportKind::ConnectionProfile, id)?;
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
                        ensure_imported(imports, ImportKind::Character, c)?;
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
                ensure_imported(imports, ImportKind::Template, template_id)?;

                let profile_id = req.profile_id.ok_or_else(|| {
                    RealmError::InvalidAgent("llm agents require an explicit profile_id".into())
                })?;
                self.connection.get_profile(user_id, profile_id).await?;
                ensure_imported(imports, ImportKind::ConnectionProfile, profile_id)?;

                for &lorebook_id in &req.lorebooks {
                    ensure_imported(imports, ImportKind::Lorebook, lorebook_id)?;
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
            ensure_imported(imports, ImportKind::Persona, id)?;
        }

        for &widget_id in &req.widgets {
            ensure_imported(imports, ImportKind::Widget, widget_id)?;
        }

        for binding in &req.tools {
            if builtin_tool(binding.tool_id).is_some() {
                continue;
            }
            let tool = self
                .storage
                .get_tool(binding.tool_id)
                .await?
                .ok_or(RealmError::NotFound(TargetType::UserTool, binding.tool_id))?;
            if tool.user_id != user_id {
                return Err(RealmError::NotOwned(TargetType::UserTool, binding.tool_id));
            }
            ensure_imported(imports, ImportKind::Tool, binding.tool_id)?;
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
    exclude: Option<Snowflake>,
) -> Result<(), RealmError> {
    if !is_identifier(handle) {
        return Err(RealmError::InvalidAgent(format!(
            "handle '{handle}' must be a bare identifier ([A-Za-z_][A-Za-z0-9_]*)"
        )));
    }
    if cast
        .iter()
        .filter(|a| Some(a.id) != exclude)
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

/// One resource of `kind` is declared in the realm's imports.
pub(super) fn ensure_imported(
    imports: &[ImportRule],
    kind: ImportKind,
    id: Snowflake,
) -> Result<(), RealmError> {
    if imports
        .iter()
        .any(|r| r.kind == kind && r.resource_id == id)
    {
        Ok(())
    } else {
        Err(RealmError::NotImported(kind, id))
    }
}

/// Everything an agent references that the realm's imports must cover.
pub(super) fn agent_required_imports(agent: &RealmAgent) -> Vec<(ImportKind, Snowflake)> {
    let mut required = Vec::new();
    if let Some(c) = agent.character_id {
        required.push((ImportKind::Character, c));
    }
    if let Some(t) = agent.template_id {
        required.push((ImportKind::Template, t));
    }
    if let Some(p) = agent.profile_id {
        required.push((ImportKind::ConnectionProfile, p));
    }
    if let Some(p) = agent.persona_id {
        required.push((ImportKind::Persona, p));
    }
    required.extend(
        agent
            .tools
            .iter()
            .filter(|b| builtin_tool(b.tool_id).is_none())
            .map(|b| (ImportKind::Tool, b.tool_id)),
    );
    required.extend(agent.lorebooks.iter().map(|&l| (ImportKind::Lorebook, l)));
    required.extend(agent.widgets.iter().map(|&w| (ImportKind::Widget, w)));
    required
}

pub(super) fn apply_turn_order(
    agents: &mut [RealmAgent],
    order: &[Snowflake],
) -> Result<(), RealmError> {
    if order.len() != agents.len() {
        return Err(RealmError::InvalidAgent(format!(
            "order lists {} agents, the cast has {}",
            order.len(),
            agents.len()
        )));
    }
    let positions: HashMap<Snowflake, usize> =
        order.iter().enumerate().map(|(i, &id)| (id, i)).collect();
    if positions.len() != order.len() {
        return Err(RealmError::InvalidAgent(
            "order contains duplicate agent ids".into(),
        ));
    }
    if agents.iter().any(|a| !positions.contains_key(&a.id)) {
        return Err(RealmError::InvalidAgent(
            "order must list every cast agent exactly once".into(),
        ));
    }

    agents.sort_by_key(|a| positions[&a.id]);
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
            id: sf(1),
            name: name.into(),
            handle: name.into(),
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

    fn rule(kind: ImportKind, id: i64) -> ImportRule {
        ImportRule {
            kind,
            resource_id: sf(id),
        }
    }

    #[test]
    fn required_imports_cover_every_reference_kind() {
        let required = agent_required_imports(&agent("spymaster"));
        assert_eq!(
            required,
            vec![
                (ImportKind::Character, sf(10)),
                (ImportKind::Template, sf(20)),
                (ImportKind::ConnectionProfile, sf(30)),
                (ImportKind::Persona, sf(60)),
                (ImportKind::Tool, sf(40)),
                (ImportKind::Lorebook, sf(50)),
            ]
        );
    }

    #[test]
    fn player_agents_require_only_their_tools() {
        let mut player = agent("you");
        player.kind = AgentKind::Player;
        player.character_id = None;
        player.char_version_id = None;
        player.template_id = None;
        player.profile_id = None;
        player.persona_id = None;
        player.lorebooks.clear();

        assert_eq!(
            agent_required_imports(&player),
            vec![(ImportKind::Tool, sf(40))]
        );
    }

    #[test]
    fn player_agents_may_require_a_connection_profile() {
        let mut player = agent("you");
        player.kind = AgentKind::Player;
        player.character_id = None;
        player.char_version_id = None;
        player.template_id = None;
        player.persona_id = None;
        player.lorebooks.clear();
        // profile_id stays Some(sf(30)) from the fixture

        assert_eq!(
            agent_required_imports(&player),
            vec![
                (ImportKind::ConnectionProfile, sf(30)),
                (ImportKind::Tool, sf(40))
            ]
        );
    }

    #[test]
    fn script_agents_depend_on_no_imports() {
        let mut script = agent("log-keeper");
        script.kind = AgentKind::Script;
        script.character_id = None;
        script.char_version_id = None;
        script.template_id = None;
        script.profile_id = None;
        script.persona_id = None;
        script.tools.clear();
        script.lorebooks.clear();
        script.script = Some("state.set('log.events', 'entries', [])".into());

        assert_eq!(agent_required_imports(&script), vec![]);
    }

    #[test]
    fn ensure_imported_distinguishes_kinds() {
        let imports = vec![rule(ImportKind::Tool, 50)];
        assert!(ensure_imported(&imports, ImportKind::Tool, sf(50)).is_ok());
        assert!(matches!(
            ensure_imported(&imports, ImportKind::Lorebook, sf(50)),
            Err(RealmError::NotImported(ImportKind::Lorebook, _))
        ));
    }

    #[test]
    fn ensure_imported_rejects_missing() {
        assert!(matches!(
            ensure_imported(&[], ImportKind::Character, sf(10)),
            Err(RealmError::NotImported(ImportKind::Character, _))
        ));
    }

    fn cast_of(ids: &[i64]) -> Vec<RealmAgent> {
        ids.iter()
            .enumerate()
            .map(|(i, &id)| {
                let mut a = agent(&format!("a{id}"));
                a.id = sf(id);
                a.turn_order = i as i32;
                a
            })
            .collect()
    }

    #[test]
    fn reorder_applies_permutation_and_reindexes() {
        let mut agents = cast_of(&[1, 2, 3]);
        apply_turn_order(&mut agents, &[sf(3), sf(1), sf(2)]).unwrap();
        let got: Vec<(Snowflake, i32)> = agents.iter().map(|a| (a.id, a.turn_order)).collect();
        assert_eq!(got, vec![(sf(3), 0), (sf(1), 1), (sf(2), 2)]);
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
        let mut agents = cast_of(&[1, 2, 3]);
        assert!(apply_turn_order(&mut agents, &[sf(1), sf(2)]).is_err());
        // duplicate id
        let mut agents = cast_of(&[1, 2, 3]);
        assert!(apply_turn_order(&mut agents, &[sf(1), sf(1), sf(2)]).is_err());
        // unknown id in place of a real one
        let mut agents = cast_of(&[1, 2, 3]);
        assert!(apply_turn_order(&mut agents, &[sf(1), sf(2), sf(99)]).is_err());
        // failures leave turn_order untouched
        assert_eq!(
            agents.iter().map(|a| a.turn_order).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }
}
