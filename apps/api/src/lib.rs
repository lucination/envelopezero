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
use lettre::message::Mailbox;
use lettre::AsyncSmtpTransport;
use lettre::AsyncTransport;
use lettre::Message;
use lettre::Tokio1Executor;
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
    pub feature_assignments: bool,
    pub app_origin: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_from: String,
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
        .route("/api/projections/month/:month", get(month_projection))
        .route(
            "/api/category-assignments",
            get(list_category_assignments).post(create_category_assignment),
        )
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
    user_id: String,
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

    let body = format!("Click to sign in: {magic_url}");
    sqlx::query("insert into email_outbox (to_email, subject, body) values ($1, $2, $3)")
        .bind(&email)
        .bind("Your EnvelopeZero sign-in link")
        .bind(&body)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = send_magic_link_email(&state, &email, &body).await;

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
        sqlx::query("insert into user_emails (user_id, user_pillid, email, verified_at) select u.id, u.pillid, $2, now() from users u where u.id = $1")
            .bind(uid)
            .bind(&email)
            .execute(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        sqlx::query("insert into auth_methods (user_id, user_pillid, method_type, label) select u.id, u.pillid, 'magic_link_email', $2 from users u where u.id = $1")
            .bind(uid)
            .bind(&email)
            .execute(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        sqlx::query("insert into budgets (user_id, user_pillid, name, currency_code, is_default) select u.id, u.pillid, 'My Budget', 'USD', true from users u where u.id = $1")
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
    sqlx::query("insert into sessions (user_id, user_pillid, token_hash, expires_at) select u.id, u.pillid, $2, $3 from users u where u.id = $1")
        .bind(user_id)
        .bind(session_hash)
        .bind(expires_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (user_pillid,): (String,) = sqlx::query_as("select pillid from users where id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SessionResponse {
        token: session_token,
        user_id: user_pillid,
    }))
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<UserDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let user = sqlx::query_as::<_, UserDto>(
        "select u.pillid as id, ue.email from users u join user_emails ue on ue.user_id = u.id where u.id = $1 order by ue.verified_at desc nulls last limit 1",
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
    id: String,
    email: String,
}

#[derive(Serialize, FromRow)]
struct BudgetDto {
    id: String,
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
    let rows = sqlx::query_as::<_, BudgetDto>("select pillid as id, name, currency_code, is_default from budgets where user_id = $1 order by created_at")
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
    let row = sqlx::query_as::<_, BudgetDto>("insert into budgets (user_id, user_pillid, name, currency_code, is_default) select u.id, u.pillid, $2, $3, false from users u where u.id = $1 returning pillid as id, name, currency_code, is_default")
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
    id: String,
    budget_id: String,
    name: String,
}

#[derive(Deserialize)]
struct SaveAccount {
    budget_id: String,
    name: String,
}

async fn list_accounts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AccountDto>>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let rows = sqlx::query_as::<_, AccountDto>("select pillid as id, budget_pillid as budget_id, name from accounts where user_id = $1 and deleted_at is null order by created_at")
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
    let row = sqlx::query_as::<_, AccountDto>("insert into accounts (user_id, user_pillid, budget_id, budget_pillid, name) select u.id, u.pillid, b.id, b.pillid, $3 from users u join budgets b on b.pillid = $2 and b.user_id = u.id and b.deleted_at is null where u.id = $1 returning pillid as id, budget_pillid as budget_id, name")
        .bind(user_id)
        .bind(payload.budget_id)
        .bind(payload.name)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(row))
}

async fn update_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<SaveAccount>,
) -> Result<Json<AccountDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let row = sqlx::query_as::<_, AccountDto>("update accounts a set budget_id = b.id, budget_pillid = b.pillid, name = $3, updated_at = now() from budgets b where a.pillid = $1 and a.user_id = $4 and b.pillid = $2 and b.user_id = $4 and b.deleted_at is null returning a.pillid as id, a.budget_pillid as budget_id, a.name")
        .bind(id)
        .bind(payload.budget_id)
        .bind(payload.name)
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(row))
}

async fn delete_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    sqlx::query("update accounts set deleted_at = now() where pillid = $1 and user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, FromRow)]
struct SupercategoryDto {
    id: String,
    budget_id: String,
    name: String,
}
#[derive(Deserialize)]
struct SaveSupercategory {
    budget_id: String,
    name: String,
}

async fn list_supercategories(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<SupercategoryDto>>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let rows = sqlx::query_as::<_, SupercategoryDto>("select pillid as id, budget_pillid as budget_id, name from supercategories where user_id = $1 and deleted_at is null order by created_at")
        .bind(user_id).fetch_all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}
async fn create_supercategory(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SaveSupercategory>,
) -> Result<Json<SupercategoryDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let row = sqlx::query_as::<_, SupercategoryDto>("insert into supercategories (user_id, user_pillid, budget_id, budget_pillid, name) select u.id, u.pillid, b.id, b.pillid, $3 from users u join budgets b on b.pillid = $2 and b.user_id = u.id and b.deleted_at is null where u.id = $1 returning pillid as id, budget_pillid as budget_id, name")
        .bind(user_id).bind(payload.budget_id).bind(payload.name).fetch_one(&state.db).await.map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(row))
}
async fn update_supercategory(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<SaveSupercategory>,
) -> Result<Json<SupercategoryDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let row = sqlx::query_as::<_, SupercategoryDto>("update supercategories s set budget_id=b.id,budget_pillid=b.pillid,name=$3,updated_at=now() from budgets b where s.pillid=$1 and s.user_id=$4 and b.pillid=$2 and b.user_id=$4 and b.deleted_at is null returning s.pillid as id,s.budget_pillid as budget_id,s.name")
        .bind(id).bind(payload.budget_id).bind(payload.name).bind(user_id).fetch_one(&state.db).await.map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(row))
}
async fn delete_supercategory(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    sqlx::query("update supercategories set deleted_at=now() where pillid=$1 and user_id=$2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, FromRow)]
struct CategoryDto {
    id: String,
    budget_id: String,
    supercategory_id: String,
    name: String,
}
#[derive(Deserialize)]
struct SaveCategory {
    budget_id: String,
    supercategory_id: String,
    name: String,
}

async fn list_categories(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<CategoryDto>>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let rows = sqlx::query_as::<_, CategoryDto>("select pillid as id, budget_pillid as budget_id, supercategory_pillid as supercategory_id, name from categories where user_id = $1 and deleted_at is null order by created_at")
        .bind(user_id).fetch_all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}
async fn create_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SaveCategory>,
) -> Result<Json<CategoryDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let row = sqlx::query_as::<_, CategoryDto>("insert into categories (user_id, user_pillid, budget_id, budget_pillid, supercategory_id, supercategory_pillid, name) select u.id, u.pillid, b.id, b.pillid, s.id, s.pillid, $4 from users u join budgets b on b.pillid=$2 and b.user_id=u.id and b.deleted_at is null join supercategories s on s.pillid=$3 and s.user_id=u.id and s.deleted_at is null and s.budget_id=b.id where u.id=$1 returning pillid as id,budget_pillid as budget_id,supercategory_pillid as supercategory_id,name")
        .bind(user_id).bind(payload.budget_id).bind(payload.supercategory_id).bind(payload.name).fetch_one(&state.db).await.map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(row))
}
async fn update_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<SaveCategory>,
) -> Result<Json<CategoryDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let row = sqlx::query_as::<_, CategoryDto>("update categories c set budget_id=b.id,budget_pillid=b.pillid,supercategory_id=s.id,supercategory_pillid=s.pillid,name=$4,updated_at=now() from budgets b, supercategories s where c.pillid=$1 and c.user_id=$5 and b.pillid=$2 and b.user_id=$5 and b.deleted_at is null and s.pillid=$3 and s.user_id=$5 and s.deleted_at is null and s.budget_id=b.id returning c.pillid as id,c.budget_pillid as budget_id,c.supercategory_pillid as supercategory_id,c.name")
        .bind(id).bind(payload.budget_id).bind(payload.supercategory_id).bind(payload.name).bind(user_id).fetch_one(&state.db).await.map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(row))
}
async fn delete_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    sqlx::query("update categories set deleted_at=now() where pillid=$1 and user_id=$2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct SplitInput {
    category_id: String,
    memo: Option<String>,
    inflow: i64,
    outflow: i64,
}

fn validate_splits(splits: &[SplitInput]) -> Result<(), StatusCode> {
    if splits.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    for split in splits {
        if split.inflow < 0 || split.outflow < 0 {
            return Err(StatusCode::BAD_REQUEST);
        }
        if split.inflow > 0 && split.outflow > 0 {
            return Err(StatusCode::BAD_REQUEST);
        }
        if split.inflow == 0 && split.outflow == 0 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    Ok(())
}
#[derive(Deserialize)]
struct SaveTransaction {
    budget_id: String,
    account_id: String,
    date: NaiveDate,
    payee: Option<String>,
    memo: Option<String>,
    splits: Vec<SplitInput>,
}
#[derive(Serialize, FromRow)]
struct SplitDto {
    id: String,
    category_id: String,
    memo: Option<String>,
    inflow: i64,
    outflow: i64,
}

type TransactionRow = (
    String,
    String,
    String,
    NaiveDate,
    Option<String>,
    Option<String>,
);

#[derive(Serialize)]
struct TransactionDto {
    id: String,
    budget_id: String,
    account_id: String,
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
    let tx_rows: Vec<TransactionRow> = sqlx::query_as("select pillid,budget_pillid,account_pillid,tx_date,payee,memo from transactions where user_id=$1 and deleted_at is null order by tx_date desc, created_at desc")
        .bind(user_id).fetch_all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut out = Vec::with_capacity(tx_rows.len());
    for (id, budget_id, account_id, date, payee, memo) in tx_rows {
        let splits = sqlx::query_as::<_, SplitDto>("select pillid as id, category_pillid as category_id, memo, inflow, outflow from transaction_splits where transaction_pillid=$1 and deleted_at is null order by created_at")
            .bind(&id).fetch_all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    validate_splits(&payload.splits)?;

    let user_id = user_from_headers(&state, &headers).await?;
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let (id, budget_id, account_id): (String, String, String) = sqlx::query_as("insert into transactions (user_id,user_pillid,budget_id,budget_pillid,account_id,account_pillid,tx_date,payee,memo) select u.id,u.pillid,b.id,b.pillid,a.id,a.pillid,$4,$5,$6 from users u join budgets b on b.pillid=$2 and b.user_id=u.id and b.deleted_at is null join accounts a on a.pillid=$3 and a.user_id=u.id and a.budget_id=b.id and a.deleted_at is null where u.id=$1 returning pillid,budget_pillid,account_pillid")
        .bind(user_id).bind(payload.budget_id.clone()).bind(payload.account_id.clone()).bind(payload.date).bind(payload.payee.clone()).bind(payload.memo.clone())
        .fetch_one(&mut *tx).await.map_err(|_| StatusCode::BAD_REQUEST)?;
    for s in &payload.splits {
        sqlx::query("insert into transaction_splits (transaction_id,transaction_pillid,category_id,category_pillid,memo,inflow,outflow) select t.id,t.pillid,c.id,c.pillid,$3,$4,$5 from transactions t join categories c on c.pillid=$2 and c.user_id=$6 and c.deleted_at is null and c.budget_id=t.budget_id where t.pillid=$1")
            .bind(&id).bind(&s.category_id).bind(s.memo.clone()).bind(s.inflow).bind(s.outflow).bind(user_id)
            .execute(&mut *tx).await.map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(TransactionDto {
        id,
        budget_id,
        account_id,
        date: payload.date,
        payee: payload.payee,
        memo: payload.memo,
        splits: vec![],
    }))
}

async fn update_transaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<SaveTransaction>,
) -> Result<Json<TransactionDto>, StatusCode> {
    validate_splits(&payload.splits)?;

    let user_id = user_from_headers(&state, &headers).await?;
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let (budget_id, account_id): (String, String) = sqlx::query_as("update transactions t set budget_id=b.id,budget_pillid=b.pillid,account_id=a.id,account_pillid=a.pillid,tx_date=$4,payee=$5,memo=$6,updated_at=now() from budgets b, accounts a where t.pillid=$1 and t.user_id=$7 and b.pillid=$2 and b.user_id=$7 and b.deleted_at is null and a.pillid=$3 and a.user_id=$7 and a.budget_id=b.id and a.deleted_at is null returning t.budget_pillid,t.account_pillid")
        .bind(&id).bind(payload.budget_id.clone()).bind(payload.account_id.clone()).bind(payload.date).bind(payload.payee.clone()).bind(payload.memo.clone()).bind(user_id)
        .fetch_one(&mut *tx).await.map_err(|_| StatusCode::BAD_REQUEST)?;
    sqlx::query("update transaction_splits set deleted_at=now() where transaction_pillid=$1")
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for s in &payload.splits {
        sqlx::query("insert into transaction_splits (transaction_id,transaction_pillid,category_id,category_pillid,memo,inflow,outflow) select t.id,t.pillid,c.id,c.pillid,$3,$4,$5 from transactions t join categories c on c.pillid=$2 and c.user_id=$6 and c.deleted_at is null and c.budget_id=t.budget_id where t.pillid=$1")
            .bind(&id).bind(&s.category_id).bind(s.memo.clone()).bind(s.inflow).bind(s.outflow).bind(user_id)
            .execute(&mut *tx).await.map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(TransactionDto {
        id,
        budget_id,
        account_id,
        date: payload.date,
        payee: payload.payee,
        memo: payload.memo,
        splits: vec![],
    }))
}

async fn delete_transaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    sqlx::query("update transactions set deleted_at=now() where pillid=$1 and user_id=$2")
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

fn project_available(inflow: i64, outflow: i64) -> i64 {
    inflow - outflow
}

async fn compute_dashboard_projection(
    db: &PgPool,
    user_id: Uuid,
) -> Result<DashboardDto, StatusCode> {
    let (inflow, outflow): (i64, i64) = sqlx::query_as(
        "select coalesce(sum(ts.inflow),0)::bigint as inflow, coalesce(sum(ts.outflow),0)::bigint as outflow from transactions t join transaction_splits ts on ts.transaction_id=t.id where t.user_id=$1 and t.deleted_at is null and ts.deleted_at is null",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(DashboardDto {
        inflow,
        outflow,
        available: project_available(inflow, outflow),
    })
}

async fn dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<DashboardDto>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    Ok(Json(
        compute_dashboard_projection(&state.db, user_id).await?,
    ))
}

#[derive(Serialize, FromRow)]
struct CategoryProjectionDto {
    category_id: String,
    assigned: i64,
    activity: i64,
    available: i64,
}

#[derive(Deserialize)]
struct SaveCategoryAssignment {
    budget_id: String,
    category_id: String,
    month: String,
    amount: i64,
}

#[derive(Serialize, FromRow)]
struct CategoryAssignmentDto {
    id: String,
    budget_id: String,
    category_id: String,
    month: String,
    amount: i64,
}

fn parse_projection_month(month: &str) -> Result<NaiveDate, StatusCode> {
    let stamped = format!("{month}-01");
    NaiveDate::parse_from_str(&stamped, "%Y-%m-%d").map_err(|_| StatusCode::BAD_REQUEST)
}

async fn month_projection(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(month): Path<String>,
) -> Result<Json<Vec<CategoryProjectionDto>>, StatusCode> {
    let user_id = user_from_headers(&state, &headers).await?;
    let period = parse_projection_month(&month)?;

    let categories: Vec<(String, Uuid)> = sqlx::query_as(
        "select pillid, id from categories where user_id = $1 and deleted_at is null order by created_at",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut rows = Vec::with_capacity(categories.len());
    for (category_pillid, category_id) in categories {
        let (assigned,): (i64,) = sqlx::query_as(
            "select coalesce(sum(amount), 0)::bigint from category_assignments where category_id = $1 and month = $2 and deleted_at is null",
        )
        .bind(category_id)
        .bind(period)
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));

        let (activity,): (i64,) = sqlx::query_as(
            "select coalesce(sum(ts.outflow - ts.inflow), 0)::bigint
             from transaction_splits ts
             join transactions t on t.id = ts.transaction_id
             where ts.category_id = $1
               and ts.deleted_at is null
               and t.deleted_at is null
               and date_trunc('month', t.tx_date::timestamp)::date = $2",
        )
        .bind(category_id)
        .bind(period)
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));

        rows.push(CategoryProjectionDto {
            category_id: category_pillid,
            assigned,
            activity,
            available: assigned - activity,
        });
    }

    Ok(Json(rows))
}

async fn list_category_assignments(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<CategoryAssignmentDto>>, StatusCode> {
    if !state.feature_assignments {
        return Err(StatusCode::NOT_FOUND);
    }

    let user_id = user_from_headers(&state, &headers).await?;
    let rows = sqlx::query_as::<_, CategoryAssignmentDto>(
        "select pillid as id, budget_pillid as budget_id, category_pillid as category_id, to_char(month, 'YYYY-MM') as month, amount
         from category_assignments
         where user_id = $1 and deleted_at is null
         order by month desc, created_at desc",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(rows))
}

async fn create_category_assignment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SaveCategoryAssignment>,
) -> Result<Json<CategoryAssignmentDto>, StatusCode> {
    if !state.feature_assignments {
        return Err(StatusCode::NOT_FOUND);
    }

    let user_id = user_from_headers(&state, &headers).await?;
    let period = parse_projection_month(&payload.month)?;

    let row = sqlx::query_as::<_, CategoryAssignmentDto>(
        "insert into category_assignments (user_id, user_pillid, budget_id, budget_pillid, category_id, category_pillid, month, amount)
         select u.id, u.pillid, b.id, b.pillid, c.id, c.pillid, $4, $5
         from users u
         join budgets b on b.pillid = $2 and b.user_id = u.id and b.deleted_at is null
         join categories c on c.pillid = $3 and c.user_id = u.id and c.budget_id = b.id and c.deleted_at is null
         where u.id = $1
         returning pillid as id, budget_pillid as budget_id, category_pillid as category_id, to_char(month, 'YYYY-MM') as month, amount",
    )
    .bind(user_id)
    .bind(payload.budget_id)
    .bind(payload.category_id)
    .bind(period)
    .bind(payload.amount)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(row))
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
        sqlx::query("insert into user_emails (user_id,user_pillid,email,verified_at) select id,pillid,$2,now() from users where id=$1")
            .bind(uid).bind(email).execute(&mut *tx).await?;
        sqlx::query("insert into auth_methods (user_id,user_pillid,method_type,label) select id,pillid,'magic_link_email',$2 from users where id=$1")
            .bind(uid).bind(email).execute(&mut *tx).await?;
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
        sqlx::query_as::<_, (Uuid,)>("insert into budgets (user_id,user_pillid,name,currency_code,is_default) select id,pillid,'Seed Budget','USD',true from users where id=$1 returning id")
                .bind(user_id).fetch_one(&mut *tx).await?.0
    };

    sqlx::query("insert into accounts (user_id,user_pillid,budget_id,budget_pillid,name) select u.id,u.pillid,b.id,b.pillid,'Checking' from users u join budgets b on b.id=$2 where u.id=$1 on conflict do nothing")
        .bind(user_id).bind(budget_id).execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(())
}

async fn send_magic_link_email(state: &AppState, to_email: &str, body: &str) -> anyhow::Result<()> {
    let from: Mailbox = state.smtp_from.parse()?;
    let to: Mailbox = to_email.parse()?;
    let email = Message::builder()
        .from(from)
        .to(to)
        .subject("Your EnvelopeZero sign-in link")
        .body(body.to_string())?;

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(state.smtp_host.clone())
        .port(state.smtp_port)
        .build();
    mailer.send(email).await?;
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

    #[test]
    fn dashboard_projection_is_deterministic() {
        assert_eq!(project_available(4_500, 1_200), 3_300);
        assert_eq!(project_available(4_500, 1_200), 3_300);
    }

    #[test]
    fn split_validation_rejects_invalid_cases() {
        assert!(validate_splits(&[]).is_err());
        assert!(validate_splits(&[SplitInput {
            category_id: "c".into(),
            memo: None,
            inflow: -1,
            outflow: 0
        }])
        .is_err());
        assert!(validate_splits(&[SplitInput {
            category_id: "c".into(),
            memo: None,
            inflow: 1,
            outflow: 1
        }])
        .is_err());
        assert!(validate_splits(&[SplitInput {
            category_id: "c".into(),
            memo: None,
            inflow: 0,
            outflow: 0
        }])
        .is_err());
    }

    #[test]
    fn month_parse_accepts_iso_yyyy_mm() {
        let d = parse_projection_month("2026-02").unwrap();
        assert_eq!(d.format("%Y-%m-%d").to_string(), "2026-02-01");
        assert!(parse_projection_month("2026/02").is_err());
    }
}
