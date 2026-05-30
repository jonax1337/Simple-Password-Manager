//! Vault read/write endpoints.
//!
//! The server treats the vault as an opaque base64-encoded blob. Conflict
//! detection is etag-based: client `GET /vault` to learn the current etag,
//! then `PUT /vault` with `If-Match: <etag>` to atomically swap. A second
//! writer with a stale etag gets 409 and must merge client-side.

use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::{make_etag_for_blob, AuthState, AuthenticatedUserId};
use crate::error::{ApiError, ApiResult};

#[derive(Debug, Serialize)]
pub struct GetVaultResp {
    pub etag: String,
    pub ciphertext_b64: String,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct PutVaultReq {
    pub ciphertext_b64: String,
    /// `None` means "create vault that does not yet exist". `Some(etag)`
    /// means "atomically replace the vault that currently has this etag".
    /// A mismatch returns 409.
    pub expected_etag: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PutVaultResp {
    pub etag: String,
    pub updated_at: i64,
}

pub async fn get_vault(
    State(state): State<AuthState>,
    Extension(user): Extension<AuthenticatedUserId>,
) -> ApiResult<(HeaderMap, Json<GetVaultResp>)> {
    let row = state
        .storage
        .get_vault(&user.0)?
        .ok_or(ApiError::NotFound)?;
    let mut headers = HeaderMap::new();
    headers.insert("ETag", row.etag.parse().unwrap());
    Ok((
        headers,
        Json(GetVaultResp {
            etag: row.etag,
            ciphertext_b64: row.ciphertext_b64,
            updated_at: row.updated_at,
        }),
    ))
}

pub async fn put_vault(
    State(state): State<AuthState>,
    Extension(user): Extension<AuthenticatedUserId>,
    Json(req): Json<PutVaultReq>,
) -> ApiResult<Json<PutVaultResp>> {
    if req.ciphertext_b64.is_empty() {
        return Err(ApiError::BadRequest("empty vault blob".into()));
    }
    let new_etag = make_etag_for_blob(&req.ciphertext_b64);
    let row = state.storage.upsert_vault(
        &user.0,
        &req.ciphertext_b64,
        &new_etag,
        req.expected_etag.as_deref(),
    )?;
    Ok(Json(PutVaultResp {
        etag: row.etag,
        updated_at: row.updated_at,
    }))
}
