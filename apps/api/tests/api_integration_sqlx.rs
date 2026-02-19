use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use envelopezero_api::router;
use envelopezero_api::seed_dev_data;
use envelopezero_api::AppState;
use serde_json::json;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

#[sqlx::test(migrations = "./migrations")]
async fn health_endpoint_is_ok(pool: PgPool) {
    let app = router(AppState {
        db: pool,
        feature_passkeys: false,
        feature_multi_budget: false,
        app_origin: "http://localhost:8080".to_string(),
        smtp_host: "127.0.0.1".to_string(),
        smtp_port: 1025,
        smtp_from: "noreply@envelopezero.local".to_string(),
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
        smtp_host: "127.0.0.1".to_string(),
        smtp_port: 1025,
        smtp_from: "noreply@envelopezero.local".to_string(),
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

#[sqlx::test(migrations = "./migrations")]
async fn auth_and_budget_ids_are_pillids(pool: PgPool) {
    let app = router(AppState {
        db: pool.clone(),
        feature_passkeys: false,
        feature_multi_budget: false,
        app_origin: "http://localhost:8080".to_string(),
        smtp_host: "127.0.0.1".to_string(),
        smtp_port: 1025,
        smtp_from: "noreply@envelopezero.local".to_string(),
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/magic-link/request")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "email": "pillid@example.com" }).to_string(),
        ))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let token = serde_json::from_slice::<Value>(&body).unwrap()["debug_token"]
        .as_str()
        .unwrap()
        .to_string();

    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/magic-link/verify")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "token": token }).to_string()))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let session = serde_json::from_slice::<Value>(&body).unwrap();

    let user_id = session["user_id"].as_str().unwrap();
    assert_eq!(user_id.len(), 32);

    let auth = format!("Bearer {}", session["token"].as_str().unwrap());
    let req = Request::builder()
        .uri("/api/budgets")
        .header("authorization", auth)
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let budgets = serde_json::from_slice::<Vec<Value>>(&body).unwrap();
    let budget_id = budgets[0]["id"].as_str().unwrap();
    assert_eq!(budget_id.len(), 32);
}
