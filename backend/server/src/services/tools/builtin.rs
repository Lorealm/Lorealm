use std::sync::LazyLock;

use chrono::DateTime;
use lorealm_storage::models::{Param, ParamType, Snowflake, UserTool};

/// Well-known builtin ids.
pub const SPEAK_TO_ID: Snowflake = Snowflake(1);

/// Owner id stamped on materialized builtin rows. not a real user.
const BUILTIN_OWNER: Snowflake = Snowflake(0);

static CATALOG: LazyLock<Vec<UserTool>> = LazyLock::new(|| {
    vec![builtin(
        SPEAK_TO_ID,
        "speak_to",
        "Speak to",
        "Speak a message on a specific channel instead of your default speech \
         scope. Only agents who can hear that channel observe it — use it for \
         whispers, war councils, and targeted speech.",
        vec![
            Param {
                name: "channel".into(),
                ty: ParamType::String,
                description: Some("The channel to speak on".into()),
                required: true,
            },
            Param {
                name: "message".into(),
                ty: ParamType::String,
                description: Some("What to say".into()),
                required: true,
            },
        ],
        r#"$[emit_event({{cmd:channel}}, {{cmd:message}})]Sent on "{{cmd:channel}}"."#,
    )]
});

fn builtin(
    id: Snowflake,
    name: &str,
    display_name: &str,
    description: &str,
    params: Vec<Param>,
    body: &str,
) -> UserTool {
    UserTool {
        user_id: BUILTIN_OWNER,
        name: name.into(),
        id,
        display_name: Some(display_name.into()),
        description: description.into(),
        params,
        body: body.into(),
        enabled: true,
        created_at: DateTime::UNIX_EPOCH,
        updated_at: DateTime::UNIX_EPOCH,
    }
}

pub fn builtin_tools() -> &'static [UserTool] {
    &CATALOG
}

pub fn builtin_tool(id: Snowflake) -> Option<UserTool> {
    CATALOG.iter().find(|t| t.id == id).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use weaver_lang::CompiledTemplate;

    #[test]
    fn builtin_bodies_compile() {
        for tool in builtin_tools() {
            assert!(
                CompiledTemplate::compile(&tool.body).is_ok(),
                "builtin '{}' body failed to compile",
                tool.name
            );
        }
    }

    #[test]
    fn builtin_lookup_by_id() {
        assert_eq!(builtin_tool(SPEAK_TO_ID).unwrap().name, "speak_to");
        assert!(builtin_tool(Snowflake(999)).is_none());
    }
}
