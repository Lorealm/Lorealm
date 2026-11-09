use std::sync::Arc;

use chrono::{Duration, Utc};
use lorealm_encryption::Hashable;
use lorealm_storage::{
    BlobStore, Storage, StorageCore,
    backends::{MemoryStorage, SqliteStorage},
    models::{
        Conversation, CreateRole, CreateSession, CreateUser, PersonaOverride, Snowflake, Tokenizer,
    },
};

use crate::{
    errors::{AuthError, CharacterError, PersonaError},
    services::{
        character::CharacterService, persona::PersonaService, session::SessionService,
        snowflake::SnowflakeService, users::UserService,
    },
    tests::harness::MemoryBlobs,
};

struct Env {
    storage: Arc<dyn Storage>,
    snow: Arc<SnowflakeService>,
    characters: CharacterService,
    personas: PersonaService,
    users: UserService,
    _dir: Option<tempfile::TempDir>,
}

async fn memory() -> Env {
    env(Arc::new(MemoryStorage::new()), None)
}

async fn sqlite() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let mut db = SqliteStorage::new(dir.path().join("test.db"), 5, true)
        .await
        .unwrap();
    db.initialize().await.unwrap();
    env(Arc::new(db), Some(dir))
}

fn env(storage: Arc<dyn Storage>, dir: Option<tempfile::TempDir>) -> Env {
    let snow = Arc::new(SnowflakeService::new(0));
    let blob = Arc::new(MemoryBlobs::default()) as Arc<dyn BlobStore>;
    let sessions = Arc::new(SessionService::new(
        Arc::clone(&storage),
        Arc::clone(&snow),
        Default::default(),
    ));
    Env {
        characters: CharacterService::new(
            Arc::clone(&storage),
            Arc::clone(&blob),
            Arc::clone(&snow),
            Default::default(),
        ),
        personas: PersonaService::new(Arc::clone(&storage), Arc::clone(&blob), Arc::clone(&snow)),
        users: UserService::new(
            Arc::clone(&storage),
            blob,
            Arc::clone(&snow),
            Default::default(),
            sessions,
        ),
        storage,
        snow,
        _dir: dir,
    }
}

async fn both() -> [Env; 2] {
    [memory().await, sqlite().await]
}

async fn user(e: &Env, name: &str, password: &str) -> Snowflake {
    let id = e.snow.generate();
    e.storage
        .create_user(&CreateUser {
            id,
            username: name.into(),
            email: None,
            password_hash: password.to_string().hash().await.unwrap(),
        })
        .await
        .unwrap();
    id
}

async fn chat(e: &Env, user: Snowflake, character: Snowflake, version: Snowflake) -> Snowflake {
    let id = e.snow.generate();
    e.storage
        .create_conversation(&Conversation {
            id,
            user_id: user,
            character_id: Some(character),
            char_version_id: Some(version),
            realm_id: None,
            active_leaf_id: None,
            title: None,
            author_note: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();
    id
}

async fn session(e: &Env, user: Snowflake) -> Snowflake {
    let id = e.snow.generate();
    e.storage
        .create_session(&CreateSession {
            id,
            user_id: user,
            token: format!("token-{id}"),
            csrf: format!("csrf-{id}"),
            ip_address: None,
            user_agent: None,
            expires_at: Utc::now() + Duration::hours(1),
        })
        .await
        .unwrap();
    id
}

#[tokio::test]
async fn deleting_a_character_deletes_its_chats_and_nothing_else() {
    for e in both().await {
        let owner = user(&e, "owner", "password1").await;
        let doomed = e
            .characters
            .create_character(owner, "Doomed".into())
            .await
            .unwrap();
        let kept = e
            .characters
            .create_character(owner, "Kept".into())
            .await
            .unwrap();

        chat(&e, owner, doomed.id, doomed.current_version_id).await;
        chat(&e, owner, doomed.id, doomed.current_version_id).await;
        let survivor = chat(&e, owner, kept.id, kept.current_version_id).await;

        e.characters
            .delete_character(doomed.id, owner)
            .await
            .unwrap();

        let chats = e.storage.list_conversations(owner).await.unwrap();
        assert_eq!(
            chats.iter().map(|c| c.id).collect::<Vec<_>>(),
            vec![survivor]
        );

        let remaining = e.characters.list_characters(owner).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert!(e.storage.get_latest_version(doomed.id).await.is_err());
    }
}

#[tokio::test]
async fn only_the_owner_can_delete_a_character() {
    for e in both().await {
        let owner = user(&e, "owner", "password1").await;
        let stranger = user(&e, "stranger", "password1").await;
        let character = e
            .characters
            .create_character(owner, "Mine".into())
            .await
            .unwrap();
        let mine = chat(&e, owner, character.id, character.current_version_id).await;

        let err = e
            .characters
            .delete_character(character.id, stranger)
            .await
            .unwrap_err();
        assert!(matches!(err, CharacterError::PermissionDenied), "{err:?}");

        let chats = e.storage.list_conversations(owner).await.unwrap();
        assert_eq!(chats.iter().map(|c| c.id).collect::<Vec<_>>(), vec![mine]);
    }
}

#[tokio::test]
async fn the_last_persona_cannot_be_deleted() {
    for e in both().await {
        let owner = user(&e, "owner", "password1").await;
        let only = e.personas.create_default(owner, "Me".into()).await.unwrap();

        let err = e.personas.delete_persona(only, owner).await.unwrap_err();
        assert!(matches!(err, PersonaError::LastPersona), "{err:?}");
    }
}

#[tokio::test]
async fn deleting_the_active_persona_hands_over_and_drops_its_overrides() {
    for e in both().await {
        let owner = user(&e, "owner", "password1").await;
        let doomed = e
            .personas
            .create_default(owner, "Doomed".into())
            .await
            .unwrap();
        let other = e
            .personas
            .create_default(owner, "Other".into())
            .await
            .unwrap();
        let character = e
            .characters
            .create_character(owner, "Char".into())
            .await
            .unwrap();

        e.storage.set_active_persona(owner, doomed).await.unwrap();
        e.storage
            .save_override(PersonaOverride {
                character_id: character.id,
                persona_id: doomed,
                user_id: owner,
            })
            .await
            .unwrap();

        e.personas.delete_persona(doomed, owner).await.unwrap();

        assert_eq!(
            e.storage.get_active_persona(owner).await.unwrap(),
            Some(other)
        );
        assert!(
            e.storage
                .get_override(character.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(e.storage.get_persona(doomed).await.unwrap().is_none());
    }
}

#[tokio::test]
async fn a_stranger_cannot_delete_a_persona() {
    for e in both().await {
        let owner = user(&e, "owner", "password1").await;
        let stranger = user(&e, "stranger", "password1").await;
        let persona = e
            .personas
            .create_default(owner, "Mine".into())
            .await
            .unwrap();
        e.personas
            .create_default(owner, "Spare".into())
            .await
            .unwrap();

        let err = e
            .personas
            .delete_persona(persona, stranger)
            .await
            .unwrap_err();
        assert!(matches!(err, PersonaError::NotOwned), "{err:?}");
        assert!(e.storage.get_persona(persona).await.unwrap().is_some());
    }
}

#[tokio::test]
async fn changing_the_password_checks_the_old_one_and_signs_out_other_sessions() {
    for e in both().await {
        let owner = user(&e, "owner", "old-password").await;
        let here = session(&e, owner).await;
        session(&e, owner).await;
        session(&e, owner).await;

        let err = e
            .users
            .change_password(owner, here, "wrong".into(), "new-password".into())
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::WrongCurrentPassword), "{err:?}");

        let err = e
            .users
            .change_password(owner, here, "old-password".into(), "short".into())
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::PasswordLength { .. }), "{err:?}");
        assert_eq!(e.storage.list_user_sessions(owner).await.unwrap().len(), 3);

        e.users
            .change_password(owner, here, "old-password".into(), "new-password".into())
            .await
            .unwrap();

        let stored = e.storage.get_user_by_id(owner).await.unwrap().unwrap();
        assert!(
            lorealm_encryption::verify_password("new-password", &stored.password_hash).unwrap()
        );
        assert!(
            !lorealm_encryption::verify_password("old-password", &stored.password_hash).unwrap()
        );

        let live = e.storage.list_user_sessions(owner).await.unwrap();
        assert_eq!(live.iter().map(|s| s.id).collect::<Vec<_>>(), vec![here]);
    }
}

#[tokio::test]
async fn an_invite_is_used_up_and_a_failed_registration_gives_its_use_back() {
    for e in both().await {
        let admin = user(&e, "admin", "password1").await;
        let (_, code) = e
            .users
            .create_invite(admin, None, Some(1), None)
            .await
            .unwrap();

        // Too short a password: the use is handed back.
        let err = e
            .users
            .register_with_invite("first".into(), None, "short".into(), &code)
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::MalformedCredentials), "{err:?}");

        e.users
            .register_with_invite("first".into(), None, "password1".into(), &code)
            .await
            .unwrap();

        let err = e
            .users
            .register_with_invite("second".into(), None, "password1".into(), &code)
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InviteInvalid), "{err:?}");

        let invites = e.users.list_invites().await.unwrap();
        assert_eq!(invites[0].uses, 1);
    }
}

#[tokio::test]
async fn expired_and_unknown_invites_are_refused() {
    for e in both().await {
        let admin = user(&e, "admin", "password1").await;
        let (_, code) = e
            .users
            .create_invite(admin, None, None, Some(Utc::now() - Duration::minutes(1)))
            .await
            .unwrap();

        for code in [code.as_str(), "not-a-code"] {
            let err = e
                .users
                .register_with_invite("someone".into(), None, "password1".into(), code)
                .await
                .unwrap_err();
            assert!(matches!(err, AuthError::InviteInvalid), "{err:?}");
        }

        let err = e
            .users
            .create_invite(admin, None, Some(0), None)
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidInviteLimit), "{err:?}");
    }
}

#[tokio::test]
async fn an_invite_gives_its_role() {
    for e in both().await {
        let admin = user(&e, "admin", "password1").await;
        let role = e
            .storage
            .create_role(&CreateRole {
                id: e.snow.generate(),
                name: "Guest".into(),
                permissions: 0,
                position: 5,
            })
            .await
            .unwrap();
        let (_, code) = e
            .users
            .create_invite(admin, Some(role.id), None, None)
            .await
            .unwrap();

        let guest = e
            .users
            .register_with_invite("guest".into(), None, "password1".into(), &code)
            .await
            .unwrap();

        let roles = e.storage.get_user_roles(guest.id).await.unwrap();
        assert_eq!(
            roles.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![role.id]
        );
    }
}

#[tokio::test]
async fn deleting_a_user_removes_what_does_not_cascade() {
    for e in both().await {
        let doomed = user(&e, "doomed", "password1").await;
        let kept = user(&e, "kept", "password1").await;
        session(&e, doomed).await;

        let character = e
            .characters
            .create_character(doomed, "Theirs".into())
            .await
            .unwrap();
        chat(&e, doomed, character.id, character.current_version_id).await;
        e.storage
            .save_tokenizer(&Tokenizer {
                id: e.snow.generate(),
                name: "tok".into(),
                blob_uri: "blob://tok".into(),
                owner_id: doomed,
                is_public: true,
            })
            .await
            .unwrap();
        let other = e
            .characters
            .create_character(kept, "Mine".into())
            .await
            .unwrap();

        e.users.delete_user(doomed).await.unwrap();

        assert!(e.storage.get_user_by_id(doomed).await.unwrap().is_none());
        assert!(
            e.storage
                .list_conversations(doomed)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            e.storage
                .list_characters_by_owner(doomed)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            e.storage
                .list_tokenizers_by_owner(doomed)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            e.storage
                .list_user_sessions(doomed)
                .await
                .unwrap()
                .is_empty()
        );

        let theirs = e.storage.list_characters_by_owner(kept).await.unwrap();
        assert_eq!(
            theirs.iter().map(|c| c.id).collect::<Vec<_>>(),
            vec![other.id]
        );
    }
}

#[tokio::test]
async fn resetting_a_password_signs_the_user_out_everywhere() {
    for e in both().await {
        let target = user(&e, "target", "old-password").await;
        session(&e, target).await;
        session(&e, target).await;

        e.users
            .admin_update(target, None, None, Some("new-password".into()))
            .await
            .unwrap();

        let stored = e.storage.get_user_by_id(target).await.unwrap().unwrap();
        assert!(
            lorealm_encryption::verify_password("new-password", &stored.password_hash).unwrap()
        );
        assert!(
            e.storage
                .list_user_sessions(target)
                .await
                .unwrap()
                .is_empty()
        );
    }
}
