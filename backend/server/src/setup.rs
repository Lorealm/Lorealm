use std::{path::PathBuf, sync::Arc};

use dialoguer::{Confirm, Input, Password, Select, theme::ColorfulTheme};
use lorealm_storage::{BackendConfig, CacheConfig, StorageConfig};
use serde_json::json;

use crate::{config::HostingConfig, core::AppState, util::scaffold_user};

struct AdvancedStorageSetup;

enum StorageSetup {
    Local,
    Hosted,
    Advanced(AdvancedStorageSetup),
}

pub async fn run_storage_setup(state: &Arc<AppState>) -> anyhow::Result<()> {
    let theme = ColorfulTheme::default();

    let backends = [
        "Local Use (few users, local network only)",
        "Server Use (more users, hosted instance)",
        "Advanced (cherry pick your own)",
    ];
    let backend = match Select::with_theme(&theme)
        .with_prompt("How would you like to configure this server?")
        .items(&backends)
        .default(0)
        .interact()?
    {
        0 => StorageSetup::Local,
        1 => StorageSetup::Hosted,
        _ => StorageSetup::Advanced(AdvancedStorageSetup),
    };

    let config = match backend {
        StorageSetup::Local => lorealm_storage::StorageConfig::local_default(),
        StorageSetup::Hosted => {
            let conn_string: String = Input::with_theme(&theme)
                .with_prompt("Postgres connection string")
                .default("postgresql://localhost/lorealm".to_string())
                .interact_text()?;

            let create_automatically: bool = Confirm::with_theme(&theme)
                .with_prompt("Create Postgres database automatically?")
                .default(true)
                .show_default(true)
                .interact()?;

            let backends = vec![BackendConfig::Postgres {
                connection_string: Some(conn_string),
                max_connections: 10,
                create_automatically,
            }];

            let data_dir: String = Input::with_theme(&theme)
                .with_prompt("Data directory (for file blobs)")
                .default("data".to_string())
                .interact_text()?;

            let cache = match Confirm::with_theme(&theme)
                .with_prompt("Configure Cache? (Redis)")
                .default(false)
                .show_default(true)
                .interact()?
            {
                true => {
                    let redis_url: String = Input::with_theme(&theme)
                        .with_prompt("Postgres connection string")
                        .default("redis://localhost:6379".to_string())
                        .interact_text()?;

                    let key_prefix: String = Input::with_theme(&theme)
                        .with_prompt("Redis key prefix")
                        .default("lorealm".to_string())
                        .interact_text()?;

                    Some(CacheConfig {
                        redis_url,
                        ttl_seconds: 300,
                        key_prefix,
                    })
                }
                false => None,
            };

            lorealm_storage::StorageConfig {
                backends,
                data_directory: PathBuf::from(data_dir),
                require_postgres_fallback: true,
                cache,
            }
        }
        StorageSetup::Advanced(_advanced_storage_setup) => {
            lorealm_storage::StorageConfig::server_default()
        }
    };

    state.config().patch_by_key("storage", json!(config))?;
    state.init_storage(config).await?;

    Ok(())
}

pub async fn run_admin_setup(state: &Arc<AppState>) -> anyhow::Result<()> {
    let theme = ColorfulTheme::default();

    let admin_user: String = Input::with_theme(&theme)
        .with_prompt("Admin username")
        .validate_with(|input: &String| {
            if input.trim().len() >= 3 {
                Ok(())
            } else {
                Err("Username must be at least 3 characters")
            }
        })
        .interact_text()?;

    let admin_password = Password::with_theme(&theme)
        .with_prompt("Admin password")
        .with_confirmation("Confirm password", "Passwords don't match")
        .interact()?;

    let user = state
        .services()
        .users()
        .register(admin_user, None, admin_password)
        .await?;

    state
        .services()
        .permissions()
        .claim_administrator_if_unclaimed(user.id)
        .await?;

    if Confirm::with_theme(&theme)
        .with_prompt("Registration complete. Would you like to scaffold default resources?")
        .default(true)
        .interact()?
    {
        scaffold_user(&state, &user).await?;
    }

    Ok(())
}

pub async fn run_setup(state: Arc<AppState>) -> anyhow::Result<()> {
    let mut cfg_host = state.config().get::<HostingConfig>();

    let curr_version = env!("CARGO_PKG_VERSION");

    if cfg_host.last_setup_version.is_none() {
        run_storage_setup(&state).await?;
        run_admin_setup(&state).await?;

        cfg_host.last_setup_version = Some(curr_version.to_string());
        state.config().patch_by_key("hosting", json!(cfg_host))?;
        return Ok(());
    }

    let cfg_storage = state.config().get::<StorageConfig>();
    state.init_storage(cfg_storage).await?;

    if curr_version == &cfg_host.last_setup_version.unwrap() {
        return Ok(());
    }

    Ok(())
}
