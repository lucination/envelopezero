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

fn app_for(pool: PgPool) -> axum::Router {
    router(AppState {
        db: pool,
        feature_passkeys: false,
        feature_multi_budget: false,
        feature_assignments: true,
        app_origin: "http://localhost:8080".to_string(),
        smtp_host: "127.0.0.1".to_string(),
        smtp_port: 1025,
        smtp_from: "noreply@envelopezero.local".to_string(),
    })
}

#[sqlx::test(migrations = "./migrations")]
async fn health_endpoint_is_ok(pool: PgPool) {
    let app = app_for(pool);
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
    let app = app_for(pool.clone());

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
}

async fn bootstrap_auth(app: axum::Router, email: &str) -> (axum::Router, String, String) {
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/magic-link/request")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "email": email }).to_string()))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
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
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let session = serde_json::from_slice::<Value>(&body).unwrap();

    let auth_token = session["token"].as_str().unwrap().to_string();
    let auth_header = format!("Bearer {auth_token}");

    let req = Request::builder()
        .uri("/api/budgets")
        .header("authorization", auth_header)
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let budgets = serde_json::from_slice::<Vec<Value>>(&body).unwrap();
    let budget_id = budgets[0]["id"].as_str().unwrap().to_string();

    (app, auth_token, budget_id)
}

async fn bootstrap_budget_graph(
    app: axum::Router,
    auth_header: &str,
    budget_id: &str,
) -> (String, String) {
    let req = Request::builder()
        .method("POST")
        .uri("/api/accounts")
        .header("authorization", auth_header)
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "name": "Checking", "budget_id": budget_id }).to_string(),
        ))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let account_id = serde_json::from_slice::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let req = Request::builder()
        .method("POST")
        .uri("/api/supercategories")
        .header("authorization", auth_header)
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "name": "Needs", "budget_id": budget_id }).to_string(),
        ))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let supercategory_id = serde_json::from_slice::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let req = Request::builder()
        .method("POST")
        .uri("/api/categories")
        .header("authorization", auth_header)
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "name": "Groceries", "budget_id": budget_id, "supercategory_id": supercategory_id }).to_string(),
        ))
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let category_id = serde_json::from_slice::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    (account_id, category_id)
}

#[sqlx::test(migrations = "./migrations")]
async fn transaction_rejects_invalid_split_shapes(pool: PgPool) {
    let app = app_for(pool.clone());
    let (app, auth_token, budget_id) = bootstrap_auth(app, "nosplits@example.com").await;
    let auth_header = format!("Bearer {auth_token}");
    let (account_id, category_id) =
        bootstrap_budget_graph(app.clone(), &auth_header, &budget_id).await;

    for splits in [
        json!([]),
        json!([{"category_id": category_id, "inflow": -1, "outflow": 0, "memo": null}]),
        json!([{"category_id": category_id, "inflow": 1, "outflow": 1, "memo": null}]),
        json!([{"category_id": category_id, "inflow": 0, "outflow": 0, "memo": null}]),
    ] {
        let req = Request::builder()
            .method("POST")
            .uri("/api/transactions")
            .header("authorization", auth_header.clone())
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "budget_id": budget_id,
                    "account_id": account_id,
                    "date": "2026-02-19",
                    "payee": "Bad",
                    "memo": null,
                    "splits": splits
                })
                .to_string(),
            ))
            .unwrap();
        let response = app.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
