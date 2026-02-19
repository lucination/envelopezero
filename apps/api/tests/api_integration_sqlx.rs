use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use envelopezero_api::router;
use envelopezero_api::seed_dev_data;
use envelopezero_api::AppState;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

#[sqlx::test(migrations = "./migrations")]
async fn health_endpoint_is_ok(pool: PgPool) {
    let app = router(AppState {
        db: pool,
        feature_passkeys: false,
        feature_multi_budget: false,
        app_origin: "http://localhost:8080".to_string(),
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn magic_link_request_writes_outbox(pool: PgPool) {
    let app = router(AppState {
        db: pool.clone(),
        feature_passkeys: false,
        feature_multi_budget: false,
        app_origin: "http://localhost:8080".to_string(),
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/magic-link/request")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "email": "test@example.com" }).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let (count,): (i64,) = sqlx::query_as("select count(*) from email_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn seed_dev_data_is_idempotent(pool: PgPool) {
    seed_dev_data(&pool).await.unwrap();
    seed_dev_data(&pool).await.unwrap();

    let (users,): (i64,) =
        sqlx::query_as("select count(*) from user_emails where email = 'seed@envelopezero.local'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(users, 1);

    let (budgets,): (i64,) = sqlx::query_as(
        "select count(*) from budgets b join user_emails ue on ue.user_id = b.user_id where ue.email = 'seed@envelopezero.local'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(budgets, 1);
}
