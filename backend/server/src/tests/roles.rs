//! Role management through the service, against the guard.

use lorealm_storage::{
    PermissionStorage, UserStorage,
    models::{CreateRole, CreateUser, Snowflake},
};

use crate::{
    errors::PermissionError,
    services::permission::{ADMINISTRATOR_ROLE_ID, EVERYONE_ROLE_ID, Permissions},
    tests::Harness,
};

/// Seeds the built-in roles the SQL migration would, and an administrator.
async fn server() -> (Harness, Snowflake) {
    let h = Harness::new().await;
    for (id, name, permissions, position) in [
        (EVERYONE_ROLE_ID, "@everyone", Permissions::USE_PLUGINS, 0),
        (
            ADMINISTRATOR_ROLE_ID,
            "Administrator",
            Permissions::ADMINISTRATOR | Permissions::BUILD_PLUGINS,
            1000,
        ),
    ] {
        h.store
            .create_role(&CreateRole {
                id,
                name: name.into(),
                permissions: permissions.bits(),
                position,
            })
            .await
            .unwrap();
    }
    let admin = user(&h, "admin").await;
    h.store
        .assign_role(admin, ADMINISTRATOR_ROLE_ID)
        .await
        .unwrap();
    (h, admin)
}

async fn user(h: &Harness, name: &str) -> Snowflake {
    let id = h.snow.generate();
    h.store
        .create_user(&CreateUser {
            id,
            username: name.into(),
            email: Some(format!("{name}@example.com")),
            password_hash: "hash".into(),
        })
        .await
        .unwrap();
    id
}

#[tokio::test]
async fn a_moderator_manages_only_what_is_below_and_theirs_to_give() {
    let (h, admin) = server().await;
    let p = &h.permissions;

    let moderator = p
        .create_role(
            admin,
            "Moderator",
            Permissions::MANAGE_ROLES | Permissions::IMPORT_CONTENT,
            10,
        )
        .await
        .unwrap();
    let mod_user = user(&h, "mod").await;
    p.assign_role(admin, moderator.id, "mod").await.unwrap();

    // Below and within their own permissions: fine.
    let helper = p
        .create_role(mod_user, "Helper", Permissions::IMPORT_CONTENT, 5)
        .await
        .unwrap();

    // Granting what they lack, or reaching their own level, is refused.
    assert!(matches!(
        p.create_role(mod_user, "Boss", Permissions::ADMINISTRATOR, 5)
            .await,
        Err(PermissionError::Missing(_))
    ));
    assert!(matches!(
        p.create_role(mod_user, "Peer", Permissions::empty(), 10)
            .await,
        Err(PermissionError::Outranked)
    ));
    assert!(matches!(
        p.update_role(mod_user, moderator.id, Some("Mine"), None, None)
            .await,
        Err(PermissionError::Outranked)
    ));
    assert!(matches!(
        p.assign_role(mod_user, ADMINISTRATOR_ROLE_ID, "mod").await,
        Err(PermissionError::Outranked)
    ));

    // A role below them holding a flag they lack: editing other flags is fine.
    p.update_role(
        admin,
        helper.id,
        None,
        Some(Permissions::IMPORT_CONTENT | Permissions::MANAGE_SERVER),
        None,
    )
    .await
    .unwrap();
    p.update_role(
        mod_user,
        helper.id,
        None,
        Some(Permissions::MANAGE_SERVER),
        None,
    )
    .await
    .unwrap();

    p.delete_role(mod_user, helper.id).await.unwrap();
}

#[tokio::test]
async fn built_ins_hold_their_shape() {
    let (h, admin) = server().await;
    let p = &h.permissions;

    assert!(matches!(
        p.delete_role(admin, EVERYONE_ROLE_ID).await,
        Err(PermissionError::BuiltIn)
    ));
    assert!(matches!(
        p.update_role(admin, EVERYONE_ROLE_ID, None, None, Some(5))
            .await,
        Err(PermissionError::BuiltIn)
    ));
    assert!(matches!(
        p.assign_role(admin, EVERYONE_ROLE_ID, "admin").await,
        Err(PermissionError::BuiltIn)
    ));
    // Its permissions are editable.
    p.update_role(
        admin,
        EVERYONE_ROLE_ID,
        None,
        Some(Permissions::empty()),
        None,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn administrators_promote_each_other_but_never_leave_none() {
    let (h, admin) = server().await;
    let p = &h.permissions;

    assert!(matches!(
        p.revoke_role(admin, ADMINISTRATOR_ROLE_ID, admin).await,
        Err(PermissionError::LastAdministrator)
    ));

    let second = user(&h, "second").await;
    p.assign_role(admin, ADMINISTRATOR_ROLE_ID, "second")
        .await
        .unwrap();
    // Assigning twice is harmless.
    p.assign_role(admin, ADMINISTRATOR_ROLE_ID, "second")
        .await
        .unwrap();

    let members: Vec<_> = p
        .role_members(ADMINISTRATOR_ROLE_ID)
        .await
        .unwrap()
        .into_iter()
        .map(|u| u.id)
        .collect();
    assert_eq!(members, vec![admin, second]);

    p.revoke_role(second, ADMINISTRATOR_ROLE_ID, admin)
        .await
        .unwrap();
    assert!(matches!(
        p.assign_role(admin, ADMINISTRATOR_ROLE_ID, "admin").await,
        Err(PermissionError::Outranked)
    ));
}

#[tokio::test]
async fn names_are_unique_and_unknown_users_are_reported() {
    let (h, admin) = server().await;
    let p = &h.permissions;

    let role = p
        .create_role(admin, "Writers", Permissions::empty(), 3)
        .await
        .unwrap();
    assert!(matches!(
        p.create_role(admin, " Writers ", Permissions::empty(), 3)
            .await,
        Err(PermissionError::NameTaken)
    ));
    assert!(matches!(
        p.create_role(admin, "  ", Permissions::empty(), 3).await,
        Err(PermissionError::InvalidName)
    ));
    assert!(matches!(
        p.assign_role(admin, role.id, "nobody").await,
        Err(PermissionError::UserNotFound)
    ));
}

#[tokio::test]
async fn administrators_edit_their_own_role_but_cannot_break_it() {
    let (h, admin) = server().await;
    let p = &h.permissions;
    let all = Permissions::ADMINISTRATOR | Permissions::BUILD_PLUGINS;

    p.update_role(
        admin,
        ADMINISTRATOR_ROLE_ID,
        Some("Owners"),
        Some(Permissions::ADMINISTRATOR),
        None,
    )
    .await
    .unwrap();
    // The exempt flags can be toggled back on.
    p.update_role(
        admin,
        ADMINISTRATOR_ROLE_ID,
        None,
        Some(all | Permissions::UNSAFE_PLUGINS),
        None,
    )
    .await
    .unwrap();

    assert!(matches!(
        p.update_role(
            admin,
            ADMINISTRATOR_ROLE_ID,
            None,
            Some(Permissions::BUILD_PLUGINS),
            None
        )
        .await,
        Err(PermissionError::BuiltIn)
    ));
    assert!(matches!(
        p.update_role(admin, ADMINISTRATOR_ROLE_ID, None, None, Some(5))
            .await,
        Err(PermissionError::BuiltIn)
    ));
    // New roles still go below it.
    assert!(matches!(
        p.create_role(admin, "Above", Permissions::empty(), 1000)
            .await,
        Err(PermissionError::Outranked)
    ));
}
