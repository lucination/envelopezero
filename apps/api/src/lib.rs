pub mod models;

use axum::extract::Path;
use axum::extract::State;
use axum::http::header::AUTHORIZATION;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::routing::get;
use axum::routing::post;
use axum::routing::put;
use axum::Json;
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Duration;
use chrono::NaiveDate;
use chrono::Utc;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use sqlx::FromRow;
use sqlx::PgPool;
use uuid::Uuid;

#[async_trait::async_trait]
trait SessionLookup {
    async fn lookup_user_id_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<Uuid>, StatusCode>;
}

struct PgSessionLookup {
    db: PgPool,
}

#[async_trait::async_trait]
impl SessionLookup for PgSessionLookup {
    async fn lookup_user_id_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<Uuid>, StatusCode> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "select user_id from sessions where token_hash = $1 and revoked_at is null and expires_at > now()",
        )
        .bind(token_hash)
        .fetch_optional(&self.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(row.map(|r| r.0))
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub feature_passkeys: bool,
    pub feature_multi_budget: bool,
    pub app_origin: String,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/magic-link/request", post(request_magic_link))
        .route("/api/auth/magic-link/verify", post(verify_magic_link))
        .route("/api/auth/me", get(me))
        .route("/api/auth/passkey/register/start", post(passkey_disabled))
        .route("/api/auth/passkey/register/finish", post(passkey_disabled))
        .route("/api/budgets", get(list_budgets).post(create_budget))
        .route("/api/accounts", get(list_accounts).post(create_account))
        .route(
            "/api/accounts/:id",
            put(update_account).delete(delete_account),
        )
        .route(
            "/api/supercategories",
            get(list_supercategories).post(create_supercategory),
        )
        .route(
            "/api/supercategories/:id",
            put(update_supercategory).delete(delete_supercategory),
        )
        .route(
            "/api/categories",
            get(list_categories).post(create_category),
        )
        .route(
            "/api/categories/:id",
            put(update_category).delete(delete_category),
        )
        .route(
            "/api/transactions",
            get(list_transactions).post(create_transaction),
        )
        .route(
            "/api/transactions/:id",
            put(update_transaction).delete(delete_transaction),
        )
        .route("/api/dashboard", get(dashboard))
        .with_state(state)
}

#[derive(Serialize)]
struct Health {
    ok: bool,
}

async fn health() -> Json<Health> {
    Json(Health { ok: true })
}

#[derive(Deserialize)]
struct MagicLinkRequest {
    email: String,
}

#[derive(Serialize)]
struct MagicLinkRequestResponse {
    message: String,
    debug_token: Option<String>,
}

#[derive(Deserialize)]
struct MagicLinkVerifyRequest {
    token: String,
}

#[derive(Serialize)]
struct SessionResponse {
    token: String,
    user_id: Uuid,
}

async fn request_magic_link(
    State(state): State<AppState>,
    Json(payload): Json<MagicLinkRequest>,
) -> Result<Json<MagicLinkRequestResponse>, StatusCode> {
    let email = payload.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(StatusCode::BAD_REQUEST);
    }
    let token = random_token(32);
    let token_hash = sha256_hex(&token);

    let magic_url = format!("{}/?token={token}", state.app_origin.trim_end_matches('/'));

    sqlx::query("insert into magic_link_tokens (email, token_hash, expires_at) values ($1, $2, now() + interval '15 minutes')")
        .bind(&email)
        .bind(&token_hash)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("insert into email_outbox (to_email, subject, body) values ($1, $2, $3)")
        .bind(&email)
        .bind("Your EnvelopeZero sign-in link")
        .bind(format!("Click to sign in: {magic_url}"))
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(MagicLinkRequestResponse {
        message: "If this email is registered, a magic link will be sent.".into(),
        debug_token: Some(token),
    }))
}

async fn verify_magic_link(
    State(state): State<AppState>,
    Json(payload): Json<MagicLinkVerifyRequest>,
) -> Result<Json<SessionResponse>, StatusCode> {
    let token_hash = sha256_hex(&payload.token);
    let row: Option<(Uuid, String)> = sqlx::query_as("select id, email from magic_link_tokens where token_hash = $1 and consumed_at is null and expires_at > now() order by created_at desc limit 1")
        .bind(token_hash)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (token_id, email) = row.ok_or(StatusCode::UNAUTHORIZED)?;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user_id: Uuid = if let Some((uid,)) =
        sqlx::query_as::<_, (Uuid,)>("select user_id from user_emails where email = $1 limit 1")
            .bind(&email)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        uid
    } else {
        let uid = Uuid::now_v7();
        sqlx::query("insert into users (id) values ($1)")
            .bind(uid)
            .execute(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        sqlx::query("insert into user_emails (user_id, email, verified_at) values ($1, $2, now())")
            .bind(uid)
            .bind(&email)
            .execute(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        sqlx::query("insert into auth_methods (user_id, method_type, label) values ($1, 'magic_link_email', $2)")
            .bind(uid)
            .bind(&email)
            .execute(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        sqlx::query("insert into budgets (user_id, name, currency_code, is_default) values ($1, 'My Budget', 'USD', true)")
            .bind(uid)
            .execute(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        uid
    };

    sqlx::query("update magic_link_tokens set consumed_at = now() where id = $1")
        .bind(token_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let session_token = random_token(48);
    let session_hash = sha256_hex(&session_token);
    let expires_at = Utc::now() + Duration::days(30);
    sqlx::query("insert into sessions (user_id, token_hash, expires_at) values ($1, $2, $3)")
        .bind(user_id)
        .bind(session_hash)
        .bind(expires_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SessionResponse {
        token: session_token,
        user_id,
    }))
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<UserDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let user = sqlx::query_as::<_, UserDto>(
        "select u.id, ue.email from users u join user_emails ue on ue.user_id = u.id where u.id = $1 order by ue.verified_at desc nulls last limit 1",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(user))
}

async fn passkey_disabled(State(state): State<AppState>) -> Result<StatusCode, StatusCode> {
    if state.feature_passkeys {
        return Ok(StatusCode::NOT_IMPLEMENTED);
    }
    Err(StatusCode::NOT_FOUND)
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, StatusCode> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    auth.strip_prefix("Bearer ").ok_or(StatusCode::UNAUTHORIZED)
}

async fn user_from_headers(state: &AppState, headers: &HeaderMap) -> Result<Uuid, StatusCode> {
    let token = extract_bearer_token(headers)?;
    let token_hash = sha256_hex(token);
    let lookup = PgSessionLookup {
        db: state.db.clone(),
    };
    user_from_token_hash(&lookup, &token_hash).await
}

async fn user_from_token_hash<L: SessionLookup + Sync>(
    lookup: &L,
    token_hash: &str,
) -> Result<Uuid, StatusCode> {
    lookup
        .lookup_user_id_by_token_hash(token_hash)
        .await?
        .ok_or(StatusCode::UNAUTHORIZED)
}

#[derive(Serialize, FromRow)]
struct UserDto {
    id: Uuid,
    email: String,
}

#[derive(Serialize, FromRow)]
struct BudgetDto {
    id: Uuid,
    name: String,
    currency_code: String,
    is_default: bool,
}

#[derive(Deserialize)]
struct CreateBudget {
    name: String,
    currency_code: Option<String>,
}

async fn list_budgets(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<BudgetDto>>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let rows = sqlx::query_as::<_, BudgetDto>(
        "select id, name, currency_code, is_default from budgets where user_id = $1 order by created_at",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

async fn create_budget(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateBudget>,
) -> Result<Json<BudgetDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    if !state.feature_multi_budget {
        let (count,): (i64,) = sqlx::query_as("select count(*) from budgets where user_id = $1")
            .bind(user_id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if count > 0 {
            return Err(StatusCode::CONFLICT);
        }
    }
    let currency = payload.currency_code.unwrap_or_else(|| "USD".into());
    let row = sqlx::query_as::<_, BudgetDto>(
        "insert into budgets (user_id, name, currency_code, is_default) values ($1, $2, $3, false) returning id, name, currency_code, is_default",
    )
    .bind(user_id)
    .bind(payload.name)
    .bind(currency)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(row))
}

#[derive(Serialize, FromRow)]
struct AccountDto {
    id: Uuid,
    budget_id: Uuid,
    name: String,
}

#[derive(Deserialize)]
struct SaveAccount {
    budget_id: Uuid,
    name: String,
}

async fn list_accounts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AccountDto>>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let rows = sqlx::query_as::<_, AccountDto>("select id, budget_id, name from accounts where user_id = $1 and deleted_at is null order by created_at")
        .bind(user_id)
        .fetch_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

async fn create_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SaveAccount>,
) -> Result<Json<AccountDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let row = sqlx::query_as::<_, AccountDto>("insert into accounts (user_id, budget_id, name) values ($1, $2, $3) returning id, budget_id, name")
        .bind(user_id)
        .bind(payload.budget_id)
        .bind(payload.name)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(row))
}

async fn update_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<SaveAccount>,
) -> Result<Json<AccountDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let row = sqlx::query_as::<_, AccountDto>("update accounts set budget_id = $1, name = $2, updated_at = now() where id = $3 and user_id = $4 returning id, budget_id, name")
        .bind(payload.budget_id)
        .bind(payload.name)
        .bind(id)
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(row))
}

async fn delete_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    sqlx::query("update accounts set deleted_at = now() where id = $1 and user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, FromRow)]
struct SupercategoryDto {
    id: Uuid,
    budget_id: Uuid,
    name: String,
}
#[derive(Deserialize)]
struct SaveSupercategory {
    budget_id: Uuid,
    name: String,
}

async fn list_supercategories(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<SupercategoryDto>>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let rows = sqlx::query_as::<_, SupercategoryDto>("select id, budget_id, name from supercategories where user_id = $1 and deleted_at is null order by created_at")
        .bind(user_id).fetch_all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}
async fn create_supercategory(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SaveSupercategory>,
) -> Result<Json<SupercategoryDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let row = sqlx::query_as::<_, SupercategoryDto>("insert into supercategories (user_id, budget_id, name) values ($1,$2,$3) returning id, budget_id, name")
        .bind(user_id).bind(payload.budget_id).bind(payload.name).fetch_one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(row))
}
async fn update_supercategory(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<SaveSupercategory>,
) -> Result<Json<SupercategoryDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let row = sqlx::query_as::<_, SupercategoryDto>("update supercategories set budget_id=$1,name=$2,updated_at=now() where id=$3 and user_id=$4 returning id,budget_id,name")
        .bind(payload.budget_id).bind(payload.name).bind(id).bind(user_id).fetch_one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(row))
}
async fn delete_supercategory(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    sqlx::query("update supercategories set deleted_at=now() where id=$1 and user_id=$2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, FromRow)]
struct CategoryDto {
    id: Uuid,
    budget_id: Uuid,
    supercategory_id: Uuid,
    name: String,
}
#[derive(Deserialize)]
struct SaveCategory {
    budget_id: Uuid,
    supercategory_id: Uuid,
    name: String,
}

async fn list_categories(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<CategoryDto>>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let rows = sqlx::query_as::<_, CategoryDto>("select id, budget_id, supercategory_id, name from categories where user_id = $1 and deleted_at is null order by created_at")
        .bind(user_id).fetch_all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}
async fn create_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SaveCategory>,
) -> Result<Json<CategoryDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let row = sqlx::query_as::<_, CategoryDto>("insert into categories (user_id,budget_id,supercategory_id,name) values ($1,$2,$3,$4) returning id,budget_id,supercategory_id,name")
        .bind(user_id).bind(payload.budget_id).bind(payload.supercategory_id).bind(payload.name).fetch_one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(row))
}
async fn update_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<SaveCategory>,
) -> Result<Json<CategoryDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let row = sqlx::query_as::<_, CategoryDto>("update categories set budget_id=$1,supercategory_id=$2,name=$3,updated_at=now() where id=$4 and user_id=$5 returning id,budget_id,supercategory_id,name")
        .bind(payload.budget_id).bind(payload.supercategory_id).bind(payload.name).bind(id).bind(user_id).fetch_one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(row))
}
async fn delete_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    sqlx::query("update categories set deleted_at=now() where id=$1 and user_id=$2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct SplitInput {
    category_id: Uuid,
    memo: Option<String>,
    inflow: i64,
    outflow: i64,
}
#[derive(Deserialize)]
struct SaveTransaction {
    budget_id: Uuid,
    account_id: Uuid,
    date: NaiveDate,
    payee: Option<String>,
    memo: Option<String>,
    splits: Vec<SplitInput>,
}
#[derive(Serialize, FromRow)]
struct SplitDto {
    id: Uuid,
    category_id: Uuid,
    memo: Option<String>,
    inflow: i64,
    outflow: i64,
}
type TransactionRow = (Uuid, Uuid, Uuid, NaiveDate, Option<String>, Option<String>);

#[derive(Serialize)]
struct TransactionDto {
    id: Uuid,
    budget_id: Uuid,
    account_id: Uuid,
    date: NaiveDate,
    payee: Option<String>,
    memo: Option<String>,
    splits: Vec<SplitDto>,
}

async fn list_transactions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TransactionDto>>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let tx_rows: Vec<TransactionRow> = sqlx::query_as(
        "select id,budget_id,account_id,tx_date,payee,memo from transactions where user_id=$1 and deleted_at is null order by tx_date desc, created_at desc",
    ).bind(user_id).fetch_all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut out = Vec::with_capacity(tx_rows.len());
    for (id, budget_id, account_id, date, payee, memo) in tx_rows {
        let splits = sqlx::query_as::<_, SplitDto>("select id, category_id, memo, inflow, outflow from transaction_splits where transaction_id=$1 and deleted_at is null order by created_at")
            .bind(id).fetch_all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        out.push(TransactionDto {
            id,
            budget_id,
            account_id,
            date,
            payee,
            memo,
            splits,
        });
    }
    Ok(Json(out))
}

async fn create_transaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SaveTransaction>,
) -> Result<Json<TransactionDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let (id,): (Uuid,) = sqlx::query_as("insert into transactions (user_id,budget_id,account_id,tx_date,payee,memo) values ($1,$2,$3,$4,$5,$6) returning id")
        .bind(user_id).bind(payload.budget_id).bind(payload.account_id).bind(payload.date).bind(payload.payee.clone()).bind(payload.memo.clone())
        .fetch_one(&mut *tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for s in &payload.splits {
        sqlx::query("insert into transaction_splits (transaction_id,category_id,memo,inflow,outflow) values ($1,$2,$3,$4,$5)")
            .bind(id).bind(s.category_id).bind(s.memo.clone()).bind(s.inflow).bind(s.outflow)
            .execute(&mut *tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(TransactionDto {
        id,
        budget_id: payload.budget_id,
        account_id: payload.account_id,
        date: payload.date,
        payee: payload.payee,
        memo: payload.memo,
        splits: vec![],
    }))
}

async fn update_transaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<SaveTransaction>,
) -> Result<Json<TransactionDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    sqlx::query("update transactions set budget_id=$1,account_id=$2,tx_date=$3,payee=$4,memo=$5,updated_at=now() where id=$6 and user_id=$7")
        .bind(payload.budget_id).bind(payload.account_id).bind(payload.date).bind(payload.payee.clone()).bind(payload.memo.clone()).bind(id).bind(user_id)
        .execute(&mut *tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    sqlx::query("update transaction_splits set deleted_at=now() where transaction_id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for s in &payload.splits {
        sqlx::query("insert into transaction_splits (transaction_id,category_id,memo,inflow,outflow) values ($1,$2,$3,$4,$5)")
            .bind(id).bind(s.category_id).bind(s.memo.clone()).bind(s.inflow).bind(s.outflow)
            .execute(&mut *tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(TransactionDto {
        id,
        budget_id: payload.budget_id,
        account_id: payload.account_id,
        date: payload.date,
        payee: payload.payee,
        memo: payload.memo,
        splits: vec![],
    }))
}

async fn delete_transaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    sqlx::query("update transactions set deleted_at=now() where id=$1 and user_id=$2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct DashboardDto {
    inflow: i64,
    outflow: i64,
    available: i64,
}

async fn dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<DashboardDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let (inflow, outflow): (i64, i64) = sqlx::query_as(
        "select coalesce(sum(ts.inflow),0) as inflow, coalesce(sum(ts.outflow),0) as outflow from transactions t join transaction_splits ts on ts.transaction_id=t.id where t.user_id=$1 and t.deleted_at is null and ts.deleted_at is null",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(DashboardDto {
        inflow,
        outflow,
        available: inflow - outflow,
    }))
}

pub async fn seed_dev_data(pool: &PgPool) -> anyhow::Result<()> {
    let email = "seed@envelopezero.local";
    let mut tx = pool.begin().await?;
    let user_id: Uuid = if let Some((uid,)) =
        sqlx::query_as::<_, (Uuid,)>("select user_id from user_emails where email=$1 limit 1")
            .bind(email)
            .fetch_optional(&mut *tx)
            .await?
    {
        uid
    } else {
        let uid = Uuid::now_v7();
        sqlx::query("insert into users (id) values ($1)")
            .bind(uid)
            .execute(&mut *tx)
            .await?;
        sqlx::query("insert into user_emails (user_id,email,verified_at) values ($1,$2,now())")
            .bind(uid)
            .bind(email)
            .execute(&mut *tx)
            .await?;
        sqlx::query("insert into auth_methods (user_id,method_type,label) values ($1,'magic_link_email',$2)").bind(uid).bind(email).execute(&mut *tx).await?;
        uid
    };

    let budget_id: Uuid = if let Some((bid,)) = sqlx::query_as::<_, (Uuid,)>(
        "select id from budgets where user_id=$1 and is_default=true limit 1",
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        bid
    } else {
        sqlx::query_as::<_, (Uuid,)>("insert into budgets (user_id,name,currency_code,is_default) values ($1,'Seed Budget','USD',true) returning id")
                .bind(user_id).fetch_one(&mut *tx).await?.0
    };

    sqlx::query("insert into accounts (user_id,budget_id,name) values ($1,$2,'Checking') on conflict do nothing")
        .bind(user_id).bind(budget_id).execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(())
}

fn random_token(n_bytes: usize) -> String {
    let mut bytes = vec![0_u8; n_bytes];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn sha256_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    struct FakeLookup {
        result: Option<Uuid>,
    }

    #[async_trait::async_trait]
    impl SessionLookup for FakeLookup {
        async fn lookup_user_id_by_token_hash(
            &self,
            _token_hash: &str,
        ) -> Result<Option<Uuid>, StatusCode> {
            Ok(self.result)
        }
    }

    #[tokio::test]
    async fn di_lookup_returns_user() {
        let uid = Uuid::now_v7();
        let lookup = FakeLookup { result: Some(uid) };
        let got = user_from_token_hash(&lookup, "abc")
            .await
            .expect("user should be found");
        assert_eq!(got, uid);
    }

    #[tokio::test]
    async fn di_lookup_missing_is_unauthorized() {
        let lookup = FakeLookup { result: None };
        let err = user_from_token_hash(&lookup, "abc")
            .await
            .expect_err("missing session should fail");
        assert_eq!(err, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn bearer_parsing_works() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bearer testtoken".parse().unwrap());
        let token = extract_bearer_token(&headers).expect("token parsed");
        assert_eq!(token, "testtoken");
    }
}
