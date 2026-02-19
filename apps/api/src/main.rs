use std::env;
use std::net::SocketAddr;

use anyhow::Context;
use envelopezero_api::router;
use envelopezero_api::seed_dev_data;
use envelopezero_api::AppState;
use sqlx::postgres::PgPoolOptions;
use tower_http::services::ServeDir;
use tower_http::services::ServeFile;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let database_url = env::var("DATABASE_URL").context("DATABASE_URL missing")?;
    let port: u16 = env::var("PORT").unwrap_or_else(|_| "8080".into()).parse()?;
    let app_origin = env::var("APP_ORIGIN").unwrap_or_else(|_| "http://localhost:8080".into());
    let feature_passkeys =
        env::var("FEATURE_PASSKEYS").unwrap_or_else(|_| "false".into()) == "true";
    let feature_multi_budget =
        env::var("FEATURE_MULTI_BUDGET").unwrap_or_else(|_| "false".into()) == "true";
    let dev_seed = env::var("DEV_SEED").unwrap_or_else(|_| "true".into()) == "true";
    let smtp_host = env::var("SMTP_HOST").unwrap_or_else(|_| "mailpit".into());
    let smtp_port: u16 = env::var("SMTP_PORT")
        .unwrap_or_else(|_| "1025".into())
        .parse()
        .unwrap_or(1025);
    let smtp_from = env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@envelopezero.local".into());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .context("failed to connect to postgres")?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run migrations")?;

    if dev_seed {
        seed_dev_data(&pool).await.context("seed failed")?;
    }

    let api_router = router(AppState {
        db: pool,
        feature_passkeys,
        feature_multi_budget,
        app_origin,
        smtp_host,
        smtp_port,
        smtp_from,
    });

    let web_dist = env::var("WEB_DIST_DIR").unwrap_or_else(|_| "apps/web/dist".into());
    let app = api_router
        .fallback_service(
            ServeDir::new(web_dist).not_found_service(ServeFile::new("apps/web/dist/index.html")),
        )
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "envelopezero listening");
    axum::serve(listener, app).await?;
    Ok(())
}
