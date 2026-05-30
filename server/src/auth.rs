//! Account endpoints + JWT middleware.
//!
//! Trust model:
//!   - Client derives `master_key = Argon2id(master_password, kdf_salt)` locally.
//!   - Client computes `client_auth_hash = SHA256(master_key || "auth")` and
//!     sends *only* `client_auth_hash` over the wire — the master password
//!     never leaves the device.
//!   - Server stores Argon2(client_auth_hash) in `users.server_auth_hash`.
//!     If the server DB leaks, an attacker still needs to brute-force the
//!     master password through two layers of Argon2 (client + server).
//!   - Vault decryption keys are derived client-side from the same master_key
//!     and never touch the server.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::storage::Storage;

const JWT_VALID_SECS: i64 = 60 * 60 * 24 * 7; // 7 days

#[derive(Clone)]
pub struct AuthState {
    pub storage: Storage,
    pub jwt_encode: Arc<EncodingKey>,
    pub jwt_decode: Arc<DecodingKey>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// User id.
    pub sub: String,
    /// Unix seconds expiry.
    pub exp: i64,
}

#[derive(Debug, Deserialize)]
pub struct SignupReq {
    pub email: String,
    /// Salt the client used to derive its master_key (base64). Public.
    pub kdf_salt_b64: String,
    /// Hex/b64 client-derived auth proof. Server Argon2s this again and
    /// stores the result; the raw value is short-lived in RAM.
    pub client_auth_hash: String,
    /// Initial vault blob (opaque ciphertext base64) — typically the
    /// freshly-created kdbx file bytes after first signup.
    pub initial_vault_blob_b64: String,
}

#[derive(Debug, Serialize)]
pub struct SignupResp {
    pub token: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginReq {
    pub email: String,
    pub client_auth_hash: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResp {
    pub token: String,
    pub user_id: String,
    pub kdf_salt_b64: String,
}

#[derive(Debug, Serialize)]
pub struct KdfParamsResp {
    pub kdf_salt_b64: String,
}

pub async fn signup(
    State(state): State<AuthState>,
    Json(req): Json<SignupReq>,
) -> ApiResult<Json<SignupResp>> {
    if req.email.is_empty() || !req.email.contains('@') {
        return Err(ApiError::BadRequest("invalid email".into()));
    }
    if req.client_auth_hash.len() < 16 {
        return Err(ApiError::BadRequest("client_auth_hash too short".into()));
    }

    let server_hash = hash_password(&req.client_auth_hash)?;
    let user_id = Uuid::new_v4().to_string();
    state
        .storage
        .create_user(&user_id, &req.email, &server_hash, &req.kdf_salt_b64)?;

    // Initial vault upload — etag derived from blob length + a random suffix.
    // The client uses ETag-based If-Match later; the value's only contract is
    // "changes whenever the blob changes."
    let etag = make_etag(&req.initial_vault_blob_b64);
    state
        .storage
        .upsert_vault(&user_id, &req.initial_vault_blob_b64, &etag, None)?;

    let token = make_token(&state, &user_id)?;
    Ok(Json(SignupResp { token, user_id }))
}

pub async fn login(
    State(state): State<AuthState>,
    Json(req): Json<LoginReq>,
) -> ApiResult<Json<LoginResp>> {
    // Timing-side-channel note: rejecting "user not found" early reveals
    // which emails exist. We accept the leak for MVP simplicity — a
    // self-hosted single-tenant server typically only has one user anyway.
    let user = state
        .storage
        .find_user_by_email(&req.email)?
        .ok_or(ApiError::Unauthorized)?;
    verify_password(&req.client_auth_hash, &user.server_auth_hash)?;
    let token = make_token(&state, &user.id)?;
    Ok(Json(LoginResp {
        token,
        user_id: user.id,
        kdf_salt_b64: user.kdf_salt_b64,
    }))
}

pub async fn kdf_params(
    State(state): State<AuthState>,
    Json(req): Json<LoginReq>, // reuse: we only need .email
) -> ApiResult<Json<KdfParamsResp>> {
    // Public endpoint — the salt isn't a secret, it's only there to make
    // the client's KDF resist rainbow tables.
    let user = state
        .storage
        .find_user_by_email(&req.email)?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(KdfParamsResp {
        kdf_salt_b64: user.kdf_salt_b64,
    }))
}

/// Extract + validate a bearer token. Injects the decoded user id into
/// request extensions so downstream handlers can read it.
pub async fn require_auth(
    State(state): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let header_value = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;
    let token = header_value
        .strip_prefix("Bearer ")
        .ok_or(ApiError::Unauthorized)?;
    let claims = decode::<Claims>(token, &state.jwt_decode, &Validation::default())?.claims;
    let user_id: AuthenticatedUserId = AuthenticatedUserId(claims.sub);
    req.extensions_mut().insert(user_id);
    Ok(next.run(req).await)
}

/// Wrapper newtype so handlers can `Extension<AuthenticatedUserId>` cleanly
/// without colliding with other String extensions.
#[derive(Clone, Debug)]
pub struct AuthenticatedUserId(pub String);

fn hash_password(plain: &str) -> ApiResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(plain.as_bytes(), &salt)?
        .to_string();
    Ok(hash)
}

fn verify_password(plain: &str, hash_str: &str) -> ApiResult<()> {
    let parsed = PasswordHash::new(hash_str).map_err(|_| ApiError::Unauthorized)?;
    Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .map_err(|_| ApiError::Unauthorized)?;
    Ok(())
}

fn make_token(state: &AuthState, user_id: &str) -> ApiResult<String> {
    let exp = now_secs() + JWT_VALID_SECS;
    let claims = Claims {
        sub: user_id.to_string(),
        exp,
    };
    Ok(encode(&Header::default(), &claims, &state.jwt_encode)?)
}

fn make_etag(blob: &str) -> String {
    use rand::RngCore;
    let mut buf = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut buf);
    // Length-prefixing makes accidental-collision astronomically unlikely
    // and keeps the etag readable when grepping server logs.
    format!("{:x}-{:x}", blob.len(), u64::from_le_bytes(buf))
}

pub fn make_etag_for_blob(blob: &str) -> String {
    make_etag(blob)
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
