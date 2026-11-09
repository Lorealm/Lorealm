use std::sync::LazyLock;

use chrono::DateTime;
use lorealm_storage::models::{Snowflake, UserWidget, WidgetMode, WidgetPlacement};

const BUILTIN_OWNER: Snowflake = Snowflake(0);

static CATALOG: LazyLock<Vec<UserWidget>> = LazyLock::new(|| {
    let sms = include_str!("builtin/sms.html");
    vec![
        attachment(
            1,
            "SMS (sent)",
            "SMS text message bubble, sent from the phone (outgoing). For texting, DMs and phone messages.",
            sms.replace("DIRECTION", "outgoing"),
            &["sender", "content"],
            "[Text message sent{# if {{props:sender}} #} by {{props:sender}}{# endif #}: {{props:content}}]",
        ),
        attachment(
            2,
            "SMS (received)",
            "SMS text message bubble, received on the phone (incoming). For texting, DMs and phone messages.",
            sms.replace("DIRECTION", "incoming"),
            &["sender", "content"],
            "[Text message received{# if {{props:sender}} #} from {{props:sender}}{# endif #}: {{props:content}}]",
        ),
        attachment(
            3,
            "Note",
            "A handwritten paper note, letter or scribble.",
            include_str!("builtin/note.html").into(),
            &["content"],
            "[Handwritten note: {{props:content}}]",
        ),
        attachment(
            4,
            "Parchment",
            "An aged parchment note sealed with wax, for medieval or fantasy proclamations, quests, notices and letters.",
            include_str!("builtin/note-parchment.html").into(),
            &["title", "content"],
            "[Parchment{# if {{props:title}} #} titled \"{{props:title}}\"{# endif #}: {{props:content}}]",
        ),
        attachment(
            5,
            "Arcane note",
            "A glowing arcane note with a pulsing sigil, for magic spells, runes and incantations.",
            include_str!("builtin/note-arcane.html").into(),
            &["title", "content"],
            "[Arcane note{# if {{props:title}} #} titled \"{{props:title}}\"{# endif #}: {{props:content}}]",
        ),
        attachment(
            6,
            "Tavern sign",
            "A carved wooden tavern or inn sign with engraved text and iron studs, for medieval or fantasy settings.",
            include_str!("builtin/sign-medieval.html").into(),
            &["title", "subtitle"],
            "[Wooden sign: {{props:title}}{# if {{props:subtitle}} #} — {{props:subtitle}}{# endif #}]",
        ),
        attachment(
            7,
            "Brass plaque",
            "A polished brass plaque sign with riveted corners and embossed lettering, for steampunk, victorian or industrial settings.",
            include_str!("builtin/sign-steampunk.html").into(),
            &["title", "subtitle"],
            "[Brass plaque: {{props:title}}{# if {{props:subtitle}} #} — {{props:subtitle}}{# endif #}]",
        ),
        attachment(
            8,
            "Neon sign",
            "A glowing neon sign for a bar, storefront or cyberpunk city at night. color is a hex tube color such as #ff3fa4.",
            include_str!("builtin/sign-neon.html").into(),
            &["text", "color"],
            "[Neon sign: {{props:text}}]",
        ),
    ]
});

fn attachment(
    id: i64,
    name: &str,
    description: &str,
    source: String,
    props: &[&str],
    render: &str,
) -> UserWidget {
    UserWidget {
        id: Snowflake(id),
        user_id: BUILTIN_OWNER,
        name: name.into(),
        description: description.into(),
        source,
        prompt: String::new(),
        render: render.into(),
        tools: Vec::new(),
        enabled: true,
        placement: WidgetPlacement::Attachment,
        mode: WidgetMode::Every,
        props: props.iter().map(|p| (*p).to_string()).collect(),
        created_at: DateTime::UNIX_EPOCH,
        updated_at: DateTime::UNIX_EPOCH,
    }
}

pub fn builtin_widgets() -> &'static [UserWidget] {
    &CATALOG
}
