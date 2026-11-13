//! Verifies the error contract that the compiler can't check: code shape,
//! per-domain namespacing, status/code agreement, and message hygiene.

use lorealm_storage::models::Snowflake;

use crate::errors::*;

fn id() -> Snowflake {
    Snowflake(1234567890123456789)
}

/// Every client-facing variant, grouped by the namespace its codes must use.
fn contract_surface() -> Vec<(&'static str, Vec<Box<dyn DomainError>>)> {
    vec![
        (
            "auth",
            vec![
                Box::new(AuthError::InvalidSession),
                Box::new(AuthError::Revoked),
                Box::new(AuthError::Expired),
                Box::new(AuthError::InvalidCsrfToken),
                Box::new(AuthError::MissingCsrfToken),
                Box::new(AuthError::MissingToken),
                Box::new(AuthError::MalformedCredentials),
                Box::new(AuthError::UserNotFound),
                Box::new(AuthError::IncorrectPassword),
                Box::new(AuthError::UsernameTaken("taken".into())),
                Box::new(AuthError::EmailTaken("a@b.c".into())),
                Box::new(AuthError::WrongCurrentPassword),
                Box::new(AuthError::PasswordLength { min: 8, max: 64 }),
                Box::new(AuthError::InviteInvalid),
                Box::new(AuthError::InvalidInviteLimit),
                Box::new(AuthError::RateLimited {
                    retry_after_secs: 90,
                }),
            ],
        ),
        (
            "permission",
            vec![
                Box::new(PermissionError::Missing(
                    crate::services::permission::Permissions::empty(),
                )),
                Box::new(PermissionError::RoleNotFound(id())),
                Box::new(PermissionError::UserNotFound),
                Box::new(PermissionError::Outranked),
                Box::new(PermissionError::UserOutranked),
                Box::new(PermissionError::OwnAccount),
                Box::new(PermissionError::BuiltIn),
                Box::new(PermissionError::LastAdministrator),
                Box::new(PermissionError::NameTaken),
                Box::new(PermissionError::InvalidName),
                Box::new(PermissionError::InvalidPosition),
            ],
        ),
        ("config", vec![Box::new(ConfigError::UnknownKey)]),
        (
            "state",
            vec![
                Box::new(StateError::NoActiveTemplate),
                Box::new(StateError::NoActivePersona),
                Box::new(StateError::NoActiveConnectionProfile),
                Box::new(StateError::NoActiveTokenizer),
                Box::new(StateError::NotFound(TargetType::Persona, id())),
            ],
        ),
        (
            "persona",
            vec![
                Box::new(PersonaError::NotFound(TargetType::Persona, id())),
                Box::new(PersonaError::NotOwned),
                Box::new(PersonaError::UnsupportedMediaType),
                Box::new(PersonaError::LastPersona),
            ],
        ),
        (
            "connection",
            vec![
                Box::new(ConnectionError::NotFound(
                    TargetType::ConnectionProfile,
                    id(),
                )),
                Box::new(ConnectionError::NotOwned(
                    TargetType::ConnectionProfile,
                    id(),
                )),
                Box::new(ConnectionError::Validation("Model is required.".into())),
                Box::new(ConnectionError::Corrupted(id())),
            ],
        ),
        (
            "tool",
            vec![
                Box::new(UserToolError::NotFound(TargetType::UserTool, id())),
                Box::new(UserToolError::NotOwned),
                Box::new(UserToolError::InvalidName("must be snake_case".into())),
                Box::new(UserToolError::InvalidBody("unexpected token".into())),
                Box::new(UserToolError::InvalidParam {
                    name: "profile".into(),
                    reason: "only a string parameter can name a resource".into(),
                }),
            ],
        ),
        (
            "command",
            vec![
                Box::new(CommandError::NotFound(id())),
                Box::new(CommandError::NotOwned(id())),
                Box::new(CommandError::NotACommand("summarize".into())),
                Box::new(CommandError::BadArgument {
                    name: "sides".into(),
                    reason: "'twenty' is not a number".into(),
                }),
                Box::new(CommandError::Failed {
                    name: "summarize".into(),
                    reason: "undefined variable".into(),
                }),
                Box::new(CommandError::Cancelled("summarize".into())),
            ],
        ),
        (
            "widget",
            vec![
                Box::new(UserWidgetError::NotFound(TargetType::UserWidget, id())),
                Box::new(UserWidgetError::NotOwned),
                Box::new(UserWidgetError::Invalid("missing layout".into())),
            ],
        ),
        (
            "module",
            vec![
                Box::new(ModuleError::NotFound(id())),
                Box::new(ModuleError::NameTaken("math".into())),
                Box::new(ModuleError::Invalid("expected an export".into())),
            ],
        ),
        (
            "plugin",
            vec![
                Box::new(PluginError::NotFound("hello".into())),
                Box::new(PluginError::NotListed("hello".into())),
                Box::new(PluginError::Invalid(
                    "hello".into(),
                    lorealm_plugins::PluginError::Init("boom".into()),
                )),
                Box::new(PluginError::CannotEnable(
                    "hello".into(),
                    "requires `lib` ^1.4, which is not loaded".into(),
                )),
                Box::new(PluginError::Required(
                    "lib".into(),
                    vec!["hello".into(), "other".into()],
                )),
                Box::new(PluginError::InvalidOrder),
                Box::new(PluginError::CallFailed("hello".into(), "boom".into())),
                Box::new(PluginError::CannotUpdate(
                    "hello".into(),
                    "`hello_addon` requires `hello` ^0.1, but 0.2.0 is enabled".into(),
                )),
                Box::new(PluginError::InvalidSource("no manifest.toml".into())),
                Box::new(PluginError::AlreadyInstalled("hello@0.1.0".into())),
                Box::new(PluginError::BuildFailed("hello".into())),
            ],
        ),
        (
            "tokenizer",
            vec![
                Box::new(TokenizerError::NotFound(id())),
                Box::new(TokenizerError::Forbidden(id())),
                Box::new(TokenizerError::InvalidData("bad vocab".into())),
                Box::new(TokenizerError::InvalidSource("ftp://x".into())),
                Box::new(TokenizerError::Fetch("connection reset".into())),
                Box::new(TokenizerError::Tokenize("overflow".into())),
            ],
        ),
        (
            "conversation",
            vec![
                Box::new(ConversationError::ConversationNotFound(id())),
                Box::new(ConversationError::MessageNotFound(id())),
            ],
        ),
        (
            "lorebook",
            vec![Box::new(LorebookError::NotOwned(id(), id()))],
        ),
        (
            "character",
            vec![
                Box::new(CharacterError::UnsupportedFormat("chara_card_v1".into())),
                Box::new(CharacterError::NotFound(TargetType::Character, id())),
                Box::new(CharacterError::NotOwned(id())),
                Box::new(CharacterError::PermissionDenied),
                Box::new(CharacterError::UnsupportedMediaType),
            ],
        ),
        (
            "context",
            vec![
                Box::new(ContextError::NotFound(ContextType::Lorebook, id())),
                Box::new(ContextError::ResolverNotFound("char".into())),
                Box::new(ContextError::UnknownSourceNode {
                    source_id: "core".into(),
                    node_id: "persona".into(),
                }),
            ],
        ),
        (
            "template",
            vec![
                Box::new(TemplateError::NodeNotFound(id())),
                Box::new(TemplateError::DuplicateSourceNode {
                    source_id: "core".into(),
                    node_id: "persona".into(),
                }),
                Box::new(TemplateError::InvalidParent(id())),
                Box::new(TemplateError::WouldCreateCircle),
                Box::new(TemplateError::IndexOutOfBounds { index: 9, len: 3 }),
                Box::new(TemplateError::InvalidPath(vec![0, 1])),
            ],
        ),
        (
            "chat",
            vec![
                Box::new(ChatError::NotFound(TargetType::Conversation, id())),
                Box::new(ChatError::NotOwned(TargetType::Conversation, id())),
                Box::new(ChatError::Connection("timed out".into())),
            ],
        ),
        (
            "realm",
            vec![
                Box::new(RealmError::NoAgents(id())),
                Box::new(RealmError::NotYourTurn("regent_ito".into())),
                Box::new(RealmError::NoSuchAgent("regent_ito".into())),
                Box::new(RealmError::NotFound(TargetType::Realm, id())),
                Box::new(RealmError::NotOwned(TargetType::Realm, id())),
                Box::new(RealmError::InvalidAgent("no name".into())),
                Box::new(RealmError::InvalidTurn("already ended".into())),
                Box::new(RealmError::ToolNotBound(id())),
                Box::new(RealmError::ActionFailed("roll failed".into())),
                Box::new(RealmError::InvalidScenario("no opening".into())),
                Box::new(RealmError::InvalidTemplate("no inputs block".into())),
                Box::new(RealmError::DirectorFailed("unknown handle".into())),
                Box::new(RealmError::NotExportable(
                    "Line 4 uses a connection profile".into(),
                )),
                Box::new(RealmError::InvalidBundle(
                    "This file isn't a blueprint bundle".into(),
                )),
            ],
        ),
    ]
}

/// `domain.reason`, lowercase and underscore-separated.
fn is_well_formed(code: &str) -> bool {
    let Some((domain, reason)) = code.split_once('.') else {
        return false;
    };
    let segment_ok = |s: &str| {
        !s.is_empty()
            && !s.starts_with('_')
            && !s.ends_with('_')
            && s.chars().all(|c| c.is_ascii_lowercase() || c == '_')
    };
    segment_ok(domain) && segment_ok(reason) && !reason.contains('.')
}

#[test]
fn codes_are_well_formed_and_namespaced() {
    for (namespace, errors) in contract_surface() {
        for err in errors {
            let code = err.code();
            assert!(
                is_well_formed(code),
                "{code:?} is not a well-formed domain.reason code"
            );
            assert_eq!(
                code.split_once('.').unwrap().0,
                namespace,
                "{code:?} does not live under its own namespace"
            );
        }
    }
}

/// `internal` is the only code for a 500, and a 500 never carries another code.
#[test]
fn client_facing_variants_are_never_internal() {
    for (_, errors) in contract_surface() {
        for err in errors {
            assert_ne!(err.code(), "internal", "{} leaked as internal", err);
            assert_ne!(
                err.status(),
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "{} is a 500 but carries code {:?}",
                err,
                err.code()
            );
        }
    }
}

/// Messages are prose for a user: no ids, no `Debug` output, no bare enum names.
#[test]
fn messages_are_client_safe() {
    let snowflake = id().to_string();

    for (_, errors) in contract_surface() {
        for err in errors {
            let code = err.code();
            let message = err.message();

            assert!(!message.is_empty(), "{code} has an empty message");
            assert!(
                !message.contains(&snowflake),
                "{code} leaks a snowflake: {message:?}"
            );
            assert!(
                !message.contains('{') && !message.contains('['),
                "{code} leaks Debug output: {message:?}"
            );
            assert!(
                message.starts_with(|c: char| c.is_uppercase() || c == '\''),
                "{code} message should read as a sentence: {message:?}"
            );
            assert!(
                message.ends_with('.'),
                "{code} message should end in a period: {message:?}"
            );
        }
    }
}

/// Regenerates the table in `docs/errors.md`:
/// `cargo test -p lorealm_server --bin lorealm_server dump_error_table -- --ignored --nocapture`
#[test]
#[ignore = "documentation generator, not a check"]
fn dump_error_table() {
    let mut rows: Vec<(String, u16, String)> = contract_surface()
        .into_iter()
        .flat_map(|(_, errors)| errors)
        .map(|err| {
            (
                err.code().to_owned(),
                err.status().as_u16(),
                err.message().into_owned(),
            )
        })
        .collect();
    rows.sort();
    rows.dedup_by(|a, b| a.0 == b.0);

    println!("| Code | Status | Message |");
    println!("| --- | --- | --- |");
    for (code, status, message) in rows {
        println!("| `{code}` | {status} | {message} |");
    }
}

/// A 404 must point at what was missing, so clients can prune stale state.
#[test]
fn not_found_variants_carry_details() {
    for (_, errors) in contract_surface() {
        for err in errors {
            if err.status() != axum::http::StatusCode::NOT_FOUND {
                continue;
            }
            let details = err.details();
            assert!(
                details.is_some_and(|d| d.resource.is_some()),
                "{} is a 404 without a resource pointer",
                err.code()
            );
        }
    }
}
