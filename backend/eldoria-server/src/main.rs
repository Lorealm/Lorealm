use axum::{Router, ServiceExt};
use std::{io::Read, net::SocketAddr, sync::Arc, time::Duration};
use storage_system::{Storage, StorageConfig, StorageManager};
use tokio::time::{Instant, sleep};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    models::{
        CharacterService, UserService,
        users::{EldoriaUser, UserPermissions},
    },
    services::character::parser::parse_character_card,
};

mod app_state;
mod config;
mod error;
mod middleware;
mod models;
mod routes;
mod services;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenv::dotenv();
    let config = config::load_config()?;

    // Init logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::new(config.server.log_level.to_string())
                .add_directive("sqlx=warn".parse().unwrap()),
        )
        .init();
    let storage_config = StorageConfig::server_default();
    let app_state = Arc::new(AppState::new(storage_config).await?);

    // Build app with security middleware
    let app = Router::new()
        .with_state(Arc::clone(&app_state))
        .merge(routes::api_routes(Arc::clone(&app_state)).await);

    let binding = app_state.clone().storage();
    let storage = binding.write().await;

    let user_service = storage.user_service();
    test_user_creation(&user_service).await?;

    drop(storage);

    // Serve
    let addr = SocketAddr::from((config.server.host_addr(), config.server.port));
    tracing::info!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn test_user_creation<S>(user_service: &UserService<S>) -> anyhow::Result<()>
where
    S: Storage + ?Sized,
{
    let user = EldoriaUser::create(
        "admin".to_string(),
        "password123".to_string(),
        Some(UserPermissions::ADMINISTRATOR),
    )
    .await?;

    if user_service.register_user(&user).await.is_err() {
        tracing::info!("Admin user exists.");
        return Ok(());
    }

    // test login
    if let Ok(authenticated_user) =
        EldoriaUser::login("admin".to_string(), "password123".to_string(), user_service).await
    {
        tracing::info!("User logged in!");
        tracing::info!("Loaded permissions: {}", authenticated_user.permissions);
        return Ok(());
    }

    Err(anyhow::anyhow!("boohoo"))
}

// Kept for future reference
async fn _test_character_import<S>(
    card_path: &str,
    user_id: Uuid,
    character_service: &CharacterService<S>,
) -> anyhow::Result<()>
where
    S: Storage + ?Sized,
{
    tracing::info!("Retrieving file buffer...");
    let mut file = std::fs::File::open(card_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    tracing::info!("Parsing character card...");
    let now = Instant::now();
    let chara_card = parse_character_card(&buffer, "chara", user_id)?;
    tracing::info!(
        "Successfully read character card: {:?}. {}μs elapsed",
        chara_card.character.definition.name,
        now.elapsed().as_micros()
    );

    tracing::info!("Saving character card...");
    character_service
        .save_character(&chara_card.character)
        .await?;

    tracing::info!("Loading character card...");
    let loaded_card = character_service
        .load_character(chara_card.character.id)
        .await?;
    tracing::info!(
        "Retrieved character '{}' from database",
        loaded_card.definition.name
    );

    Ok(())
}
