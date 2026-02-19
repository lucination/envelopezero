use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::routing::post;
use axum::Json;
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Duration;
use chrono::Utc;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/auth/magic-link/request", post(request_magic_link))
        .route("/auth/magic-link/verify", post(verify_magic_link))
        .route("/auth/passkey/register/start", post(passkey_start))
        .route("/auth/passkey/register/finish", post(passkey_finish))
        .route("/auth/methods/:user_id", get(list_methods))
        .route("/auth/methods/:user_id/remove", post(remove_method))
        .with_state(state)
}

#[derive(Serialize)]
struct Health {
    ok: bool,
    service: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health {
        ok: true,
        service: "envelopezero-api",
    })
}

#[derive(Deserialize)]
struct MagicLinkRequest {
    email: String,
}

#[derive(Serialize)]
struct MagicLinkRequestResponse {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    debug_token: Option<String>,
}

#[derive(Deserialize)]
struct MagicLinkVerifyRequest {
    token: String,
}

#[derive(Serialize)]
struct SessionResponse {
    session_token: String,
    user_id: Uuid,
}

#[derive(Deserialize)]
struct PasskeyStartRequest {
    user_id: Uuid,
}

#[derive(Serialize)]
struct PasskeyStartResponse {
    challenge_id: Uuid,
    challenge: String,
    rp_id: String,
}

#[derive(Deserialize)]
struct PasskeyFinishRequest {
    challenge_id: Uuid,
    credential_id: String,
    public_key: String,
    label: Option<String>,
}

#[derive(Serialize)]
struct AuthMethodDto {
    id: Uuid,
    method_type: String,
    label: Option<String>,
}

#[derive(Deserialize)]
struct RemoveMethodRequest {
    method_id: Uuid,
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

    sqlx::query(
        "insert into magic_link_tokens (email, token_hash, expires_at) values ($1, $2, now() + interval '15 minutes')",
    )
    .bind(&email)
    .bind(&token_hash)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("insert into email_outbox (to_email, subject, body) values ($1, $2, $3)")
        .bind(&email)
        .bind("Your EnvelopeZero sign-in link")
        .bind(format!("Use this token to sign in: {token}"))
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

    let row: Option<(Uuid, String)> = sqlx::query_as(
        "select id, email from magic_link_tokens where token_hash = $1 and consumed_at is null and expires_at > now() order by created_at desc limit 1",
    )
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

    let user_id: Uuid = if let Some((uid,)) = sqlx::query_as::<_, (Uuid,)>(
        "select user_id from user_emails where email = $1 and verified_at is not null limit 1",
    )
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
        session_token,
        user_id,
    }))
}

async fn passkey_start(
    State(state): State<AppState>,
    Json(payload): Json<PasskeyStartRequest>,
) -> Result<Json<PasskeyStartResponse>, StatusCode> {
    let challenge = random_token(32);
    let challenge_id = Uuid::now_v7();

    sqlx::query("insert into passkey_challenges (id, user_id, challenge, purpose, expires_at) values ($1, $2, $3, 'register', now() + interval '10 minutes')")
        .bind(challenge_id)
        .bind(payload.user_id)
        .bind(&challenge)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(PasskeyStartResponse {
        challenge_id,
        challenge,
        rp_id: "localhost".into(),
    }))
}

async fn passkey_finish(
    State(state): State<AppState>,
    Json(payload): Json<PasskeyFinishRequest>,
) -> Result<Json<AuthMethodDto>, StatusCode> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "select user_id from passkey_challenges where id = $1 and used_at is null and expires_at > now() and purpose = 'register'",
    )
    .bind(payload.challenge_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (user_id,) = row.ok_or(StatusCode::UNAUTHORIZED)?;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let method_id = Uuid::now_v7();
    let label = payload.label.unwrap_or_else(|| "Passkey".into());

    sqlx::query(
        "insert into auth_methods (id, user_id, method_type, label) values ($1, $2, 'passkey', $3)",
    )
    .bind(method_id)
    .bind(user_id)
    .bind(&label)
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "insert into passkey_credentials (user_id, credential_id, public_key) values ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(&payload.credential_id)
    .bind(&payload.public_key)
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("update passkey_challenges set used_at = now() where id = $1")
        .bind(payload.challenge_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AuthMethodDto {
        id: method_id,
        method_type: "passkey".into(),
        label: Some(label),
    }))
}

async fn list_methods(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<Vec<AuthMethodDto>>, StatusCode> {
    let rows = sqlx::query_as::<_, (Uuid, String, Option<String>)>(
        "select id, method_type, label from auth_methods where user_id = $1 and disabled_at is null order by created_at asc",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(
        rows.into_iter()
            .map(|(id, method_type, label)| AuthMethodDto {
                id,
                method_type,
                label,
            })
            .collect(),
    ))
}

async fn remove_method(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<RemoveMethodRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("update auth_methods set disabled_at = now() where id = $1 and user_id = $2 and disabled_at is null")
        .bind(payload.method_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let res = sqlx::query("select assert_user_has_auth_method($1)")
        .bind(user_id)
        .execute(&mut *tx)
        .await;

    if res.is_err() {
        tx.rollback()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Err(StatusCode::CONFLICT);
    }

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
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
mod tests {
    use super::*;

    #[test]
    fn token_has_entropy() {
        let a = random_token(32);
        let b = random_token(32);
        assert_ne!(a, b);
        assert!(a.len() > 20);
    }

    #[test]
    fn hash_is_stable() {
        let h1 = sha256_hex("abc");
        let h2 = sha256_hex("abc");
        assert_eq!(h1, h2);
    }
}
