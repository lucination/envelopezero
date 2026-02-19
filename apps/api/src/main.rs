use std::{env, net::SocketAddr};

use anyhow::Context;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

#[derive(Clone)]
struct AppState {
    db: PgPool,
}

#[derive(Serialize)]
struct Health {
    ok: bool,
    service: &'static str,
}

#[derive(Deserialize)]
struct MagicLinkRequest {
    email: String,
}

#[derive(Serialize)]
struct ApiMessage {
    message: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let database_url = env::var("DATABASE_URL").context("DATABASE_URL missing")?;
    let port: u16 = env::var("PORT").unwrap_or_else(|_| "8080".into()).parse()?;
    let app_origin = env::var("APP_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".into());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .context("failed to connect to postgres")?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run migrations")?;

    let cors = CorsLayer::new()
        .allow_origin(app_origin.parse::<axum::http::HeaderValue>()?)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .route("/auth/magic-link/request", post(request_magic_link))
        .with_state(AppState { db: pool })
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, "api listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<Health> {
    Json(Health {
        ok: true,
        service: "envelopezero-api",
    })
}

async fn request_magic_link(
    State(state): State<AppState>,
    Json(payload): Json<MagicLinkRequest>,
) -> Result<Json<ApiMessage>, axum::http::StatusCode> {
    let email = payload.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    }

    let _ = sqlx::query("insert into magic_link_tokens (email, token_hash, expires_at) values ($1, $2, now() + interval '15 minutes')")
        .bind(&email)
        .bind("placeholder_hash")
        .execute(&state.db)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiMessage {
        message: "If this email is registered, a magic link will be sent.".into(),
    }))
}
