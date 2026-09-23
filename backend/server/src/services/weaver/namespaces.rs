use serde::Serialize;
use weaver_lang::registry::ValueType;

use super::WeaverProfile;

/// One variable an editor can offer under a namespace.
#[derive(Debug, Serialize)]
pub struct MemberDef {
    pub name: String,
    pub returns: ValueType,
    pub description: String,
}

/// A `scope:` an editor can complete against.
#[derive(Debug, Serialize)]
pub struct NamespaceDef {
    pub scope: String,
    pub description: String,
    pub members: Vec<MemberDef>,
    pub profiles: Vec<WeaverProfile>,
}

fn member(name: &str, returns: ValueType, description: &str) -> MemberDef {
    MemberDef {
        name: name.into(),
        returns,
        description: description.into(),
    }
}

fn group(prefix: &str, whose: &str) -> Vec<MemberDef> {
    let at = |leaf: &str| {
        if prefix.is_empty() {
            leaf.to_string()
        } else {
            format!("{prefix}.{leaf}")
        }
    };
    vec![
        member(
            &at("count"),
            ValueType::Number,
            &format!("How many {whose} the realm holds."),
        ),
        member(
            &at("handles"),
            ValueType::Array,
            &format!("Every handle among {whose}, in turn order."),
        ),
        member(
            &at("names"),
            ValueType::Array,
            &format!("Every display name among {whose}, in turn order."),
        ),
        member(
            &at("members"),
            ValueType::Array,
            &format!(
                "One object per agent among {whose} `handle`, `name`, `kind`, \
                 `turn_order`, and whatever properties that agent currently carries."
            ),
        ),
    ]
}

/// Every namespace an editor can complete against, with what each member means.
pub fn namespaces() -> Vec<NamespaceDef> {
    vec![
        NamespaceDef {
            scope: "cast".into(),
            description: "The pinned roster, in turn order. Read-only.".into(),
            members: [
                group("", "cast members"),
                group("llm", "LLM agents"),
                group("player", "player agents"),
                group("script", "script agents"),
                vec![
                    member(
                        "agents.<handle>.handle",
                        ValueType::String,
                        "An agent's handle.",
                    ),
                    member(
                        "agents.<handle>.name",
                        ValueType::String,
                        "An agent's display name.",
                    ),
                    member(
                        "agents.<handle>.kind",
                        ValueType::String,
                        "An agent's kind: `llm`, `player` or `script`.",
                    ),
                    member(
                        "agents.<handle>.turn_order",
                        ValueType::Number,
                        "Where this agent sits in the rotation, counting from zero.",
                    ),
                ],
            ]
            .into_iter()
            .flatten()
            .collect(),
            profiles: Vec::new(),
        },
        NamespaceDef {
            scope: "turn".into(),
            description: "Info regarding the current turn. Read-only.".into(),
            members: vec![
                member(
                    "index",
                    ValueType::Number,
                    "How many agents has acted so far this turn. Zero on the opening turn.",
                ),
                member(
                    "last.handle",
                    ValueType::String,
                    "Who acted last. Absent on the opening turn.",
                ),
                member(
                    "last.name",
                    ValueType::String,
                    "The display name of whoever acted last.",
                ),
                member(
                    "last.kind",
                    ValueType::String,
                    "The kind of whoever acted last.",
                ),
                member(
                    "last.said",
                    ValueType::String,
                    "What the last message on this path contained.",
                ),
            ],
            profiles: Vec::new(),
        },
        NamespaceDef {
            scope: "scenario".into(),
            description:
                "The opening this playthrough started from. Empty when the realm has none.".into(),
            members: vec![
                member("slug", ValueType::String, "The opening's identifier."),
                member("name", ValueType::String, "The opening's display name."),
            ],
            profiles: Vec::new(),
        },
        NamespaceDef {
            scope: "actor".into(),
            description:
                "The agent this script acts for. Read-only. See `self:*` if you need write access."
                    .into(),
            members: vec![
                member("handle", ValueType::String, "The acting agent's handle."),
                member(
                    "name",
                    ValueType::String,
                    "The acting agent's display name.",
                ),
                member(
                    "kind",
                    ValueType::String,
                    "The acting agent's kind: `llm`, `player` or `script`.",
                ),
            ],
            profiles: Vec::new(),
        },
        NamespaceDef {
            scope: "self".into(),
            description: "The acting agent's own state. Writable. `self:gold` is equivalent to \
                 `agents:<handle>.gold`. Absent where no agent is bound."
                .into(),
            members: Vec::new(),
            profiles: Vec::new(),
        },
        NamespaceDef {
            scope: "agents".into(),
            description: "Per-agent state, as `agents:<handle>.<key>`.".into(),
            members: Vec::new(),
            profiles: Vec::new(),
        },
        NamespaceDef {
            scope: "local".into(),
            description: "Scratchpad for this script. All data within `local:*` is deleted every\
                turn and can always be written to, even in a read-only context."
                .into(),
            members: Vec::new(),
            profiles: Vec::new(),
        },
        NamespaceDef {
            scope: "cmd".into(),
            description: "The arguments this body was called with. Only accessible in tools and commands.".into(),
            members: Vec::new(),
            profiles: Vec::new(),
        },
        NamespaceDef {
            scope: "props".into(),
            description: "The properties used by widgets. Only accessible within widgets and attachements."
                .into(),
            members: Vec::new(),
            profiles: Vec::new(),
        },
        NamespaceDef {
            scope: "input".into(),
            description: "A WTN document's `#inputs`. Can not be used within a playthrough."
                .into(),
            members: Vec::new(),
            profiles: vec![WeaverProfile::RealmTemplate],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `input:` is bound by the document evaluator, so offering it in a script
    /// pane would suggest a read that silently returns nothing. Every other
    /// scope is reachable wherever a pool is.
    #[test]
    fn only_the_document_scope_is_gated_to_a_host() {
        for ns in namespaces() {
            match ns.scope.as_str() {
                "input" => assert_eq!(ns.profiles, vec![WeaverProfile::RealmTemplate]),
                other => assert!(
                    ns.profiles.is_empty(),
                    "'{other}:' claims a host list, so some editor will stop offering it"
                ),
            }
        }
    }
}
