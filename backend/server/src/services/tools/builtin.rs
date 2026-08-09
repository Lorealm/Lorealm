use std::sync::LazyLock;

use chrono::DateTime;
use lorealm_storage::models::{Param, ParamType, Snowflake, UserTool};

/// Well-known builtin ids.
pub const SPEAK_TO_ID: Snowflake = Snowflake(1);
pub const SEARCH_ATTACHMENTS_ID: Snowflake = Snowflake(2);
pub const ATTACH_ID: Snowflake = Snowflake(3);

/// The attachment tools, offered on every chat turn rather than being bound
/// per agent — the model needs both to discover and place an attachment.
pub const ATTACHMENT_TOOL_IDS: [Snowflake; 2] = [SEARCH_ATTACHMENTS_ID, ATTACH_ID];

/// Owner id stamped on materialized builtin rows. not a real user.
const BUILTIN_OWNER: Snowflake = Snowflake(0);

static CATALOG: LazyLock<Vec<UserTool>> = LazyLock::new(|| {
    vec![
        builtin(
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
        ),
        builtin(
            SEARCH_ATTACHMENTS_ID,
            "search_attachments",
            "Search attachments",
            "Search the catalog of visual attachments you can place inline in your reply, \
         e.g. an SMS bubble when a character sends a text message. Returns ids along \
         with the props each one accepts; pass an id to the attach tool to place it.",
            vec![Param {
                name: "keywords".into(),
                ty: ParamType::Array(Box::new(ParamType::String)),
                description: Some("Words describing what you want to show".into()),
                required: true,
            }],
            "$[search_attachments({{cmd:keywords}})]",
        ),
        builtin(
            ATTACH_ID,
            "attach",
            "Attach",
            r#"Place a visual attachment inline, at the point in your reply where it's most
            relevant. This tool returns no content you need to reason about — it only emits
            UI, rendered exactly at the position where the call appears in your message.

            The text immediately before the call must be a natural in-context lead-in that
            the attachment continues — NOT an announcement of the tool or a description of
            showing it. Never narrate the act of rendering (no "I will now show you...",
            "here is...", "let me hand you..."). It should land as if it is simply the next
            beat of your message. Write that lead-in text first, then call this tool
            immediately after.

            Use search_attachments first to get the id and props."#,
            vec![
                Param {
                    name: "id".into(),
                    ty: ParamType::String,
                    description: Some("An id obtained from search_attachments".into()),
                    required: true,
                },
                Param {
                    name: "props".into(),
                    ty: ParamType::Object,
                    description: Some("Property values, keyed by prop name".into()),
                    required: true,
                },
            ],
            "$[attach({{cmd:id}}, {{cmd:props}})]",
        ),
    ]
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

    #[test]
    fn attach_offers_props_as_a_real_object_schema() {
        let attach = builtin_tool(ATTACH_ID).unwrap();
        let props = attach.params.iter().find(|p| p.name == "props").unwrap();
        assert_eq!(
            props.ty.json_schema(),
            serde_json::json!({ "type": "object", "additionalProperties": true })
        );
    }

    /// The frontend's `ParamType` union matches on these wire strings.
    #[test]
    fn param_types_serialize_as_the_frontend_expects() {
        assert_eq!(serde_json::to_value(ParamType::Object).unwrap(), "object");
        assert_eq!(
            serde_json::to_value(ParamType::Array(Box::new(ParamType::String))).unwrap(),
            serde_json::json!({ "array": "string" })
        );
    }

    #[test]
    fn every_attachment_tool_id_resolves() {
        for id in ATTACHMENT_TOOL_IDS {
            assert!(builtin_tool(id).is_some(), "unresolvable builtin id {id:?}");
        }
    }
}
