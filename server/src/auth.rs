//! Account endpoints + JWT middleware.
//!
//! Trust model (v2 — wrapped master key):
//!   - `master_key` is a random 32-byte secret generated client-side at signup.
//!     It is the canonical input to all later derivations (`vault_key`,
//!     `client_auth_hash`).
//!   - `password_key = Argon2id(master_password, kdf_salt)`. The client uses
//!     it to AES-GCM-wrap `master_key`. The wrapped blob lives on the server.
//!   - On login the client downloads `kdf_salt + wrapped_master_key`, derives
//!     `password_key`, unwraps `master_key`, then proves possession of it via
//!     `SHA256(master_key || "auth")` → the server's `server_auth_hash` row.
//!   - On signup the client also wraps `master_key` with a high-entropy
//!     recovery code (shown once). Forgotten passwords are recoverable; the
//!     server cannot help without the recovery code.
//!   - Password change = client re-wraps `master_key` with a new
//!     `password_key`. The vault stays put — no re-encryption, no re-upload.

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
    /// Salt the client used for Argon2id(password) → password_key.
    pub kdf_salt_b64: String,
    /// `SHA256(master_key || "auth")` — proves possession of master_key
    /// without revealing it. Argon2-hashed again on the server.
    pub client_auth_hash: String,
    /// `master_key` AES-GCM-wrapped with `password_key`.
    pub wrapped_master_key_b64: String,
    /// `master_key` AES-GCM-wrapped with the recovery code. Empty string =
    /// user opted out (we still accept it but warn in the UI).
    pub recovery_blob_b64: String,
    /// Initial encrypted vault blob (typically the freshly-created kdbx
    /// file bytes after first signup).
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
    pub wrapped_master_key_b64: String,
}

#[derive(Debug, Serialize)]
pub struct KdfParamsResp {
    pub kdf_salt_b64: String,
    /// Returned alongside the salt so the client can unwrap master_key in
    /// one round-trip after the user types their password.
    pub wrapped_master_key_b64: String,
}

#[derive(Debug, Deserialize)]
pub struct RecoveryInitReq {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct RecoveryInitResp {
    /// `master_key` wrapped with `recovery_key`. Client unwraps locally to
    /// regain access; the server still doesn't see master_key.
    pub recovery_blob_b64: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetReq {
    pub email: String,
    /// `SHA256(master_key || "auth")` — must match the stored hash. Proves
    /// the client successfully unwrapped via recovery code before we let
    /// them replace credentials.
    pub client_auth_hash: String,
    pub new_kdf_salt_b64: String,
    pub new_wrapped_master_key_b64: String,
}

#[derive(Debug, Serialize)]
pub struct ResetResp {
    pub token: String,
}

pub async fn signup(
    State(state): State<AuthState>,
    Json(req): Json<SignupReq>,
) -> ApiResult<Json<SignupResp>> {
    // Treat email as a free-form username — self-hosted instances often
    // don't have working SMTP, so a "@" requirement just adds friction. We
    // still keep the field named `email` for backwards compatibility with
    // the original protocol.
    if req.email.trim().is_empty() {
        return Err(ApiError::BadRequest("username required".into()));
    }
    if req.client_auth_hash.len() < 16 {
        return Err(ApiError::BadRequest("client_auth_hash too short".into()));
    }
    if req.wrapped_master_key_b64.len() < 24 {
        return Err(ApiError::BadRequest("wrapped_master_key missing".into()));
    }

    let server_hash = hash_password(&req.client_auth_hash)?;
    let user_id = Uuid::new_v4().to_string();
    state.storage.create_user(
        &user_id,
        &req.email,
        &server_hash,
        &req.kdf_salt_b64,
        &req.wrapped_master_key_b64,
        &req.recovery_blob_b64,
    )?;

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
        wrapped_master_key_b64: user.wrapped_master_key_b64,
    }))
}

pub async fn kdf_params(
    State(state): State<AuthState>,
    Json(req): Json<LoginReq>, // reuse: we only need .email
) -> ApiResult<Json<KdfParamsResp>> {
    // Public endpoint — salt + wrapped_master_key aren't secrets. The
    // wrap is AES-GCM with a key derived from the password the client
    // about to type; an offline attacker still has to brute-force the
    // password through Argon2id (64 MiB / 3 iterations).
    let user = state
        .storage
        .find_user_by_email(&req.email)?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(KdfParamsResp {
        kdf_salt_b64: user.kdf_salt_b64,
        wrapped_master_key_b64: user.wrapped_master_key_b64,
    }))
}

/// First half of the recovery flow: client supplies email; server returns
/// `recovery_blob`. Rate-limited like other /auth endpoints.
pub async fn recovery_init(
    State(state): State<AuthState>,
    Json(req): Json<RecoveryInitReq>,
) -> ApiResult<Json<RecoveryInitResp>> {
    let user = state
        .storage
        .find_user_by_email(&req.email)?
        .ok_or(ApiError::NotFound)?;
    if user.recovery_blob_b64.is_empty() {
        return Err(ApiError::NotFound);
    }
    Ok(Json(RecoveryInitResp {
        recovery_blob_b64: user.recovery_blob_b64,
    }))
}

/// Second half: client proves it unwrapped `master_key` via the recovery
/// code (auth hash matches), then ships a new password-wrap. We swap the
/// stored salt + wrapped_master_key in one go and return a fresh token so
/// the user doesn't have to log in again.
pub async fn reset_password(
    State(state): State<AuthState>,
    Json(req): Json<ResetReq>,
) -> ApiResult<Json<ResetResp>> {
    let user = state
        .storage
        .find_user_by_email(&req.email)?
        .ok_or(ApiError::Unauthorized)?;
    // The auth hash is derived from master_key, so a matching hash =
    // matching master_key = the recovery actually worked.
    verify_password(&req.client_auth_hash, &user.server_auth_hash)?;
    // Re-hash the (unchanged) auth proof under a fresh server-side salt to
    // keep the stored value rotating with every credential reset.
    let new_server_auth = hash_password(&req.client_auth_hash)?;
    state.storage.reset_credentials(
        &user.id,
        &new_server_auth,
        &req.new_kdf_salt_b64,
        &req.new_wrapped_master_key_b64,
    )?;
    let token = make_token(&state, &user.id)?;
    Ok(Json(ResetResp { token }))
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
