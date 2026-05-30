//! Multi-vault REST surface.
//!
//! Routes:
//!   POST   /vaults              create new vault
//!   GET    /vaults              list vaults visible to caller
//!   GET    /vaults/{id}         single vault (ciphertext + caller's wrapped key)
//!   PUT    /vaults/{id}         update ciphertext (CAS via expected_etag)
//!   PATCH  /vaults/{id}         rename (owner only)
//!   DELETE /vaults/{id}         delete (owner only)
//!
//! All endpoints require Bearer auth → user_id arrives via Extension.

use axum::{
    extract::{Extension, Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::{make_etag_for_blob, AuthState, AuthenticatedUserId};
use crate::error::{ApiError, ApiResult};
use crate::storage::VaultRole;

#[derive(Debug, Serialize)]
pub struct VaultSummary {
    pub id: String,
    pub name: String,
    pub role: VaultRole,
    pub owner_user_id: String,
    pub etag: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub wrapped_vault_key_b64: String,
}

#[derive(Debug, Serialize)]
pub struct VaultFull {
    pub id: String,
    pub name: String,
    pub role: VaultRole,
    pub owner_user_id: String,
    pub etag: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub wrapped_vault_key_b64: String,
    pub ciphertext_b64: String,
}

#[derive(Debug, Serialize)]
pub struct ListVaultsResp {
    pub vaults: Vec<VaultSummary>,
}

#[derive(Debug, Deserialize)]
pub struct CreateVaultReq {
    pub name: String,
    pub ciphertext_b64: String,
    pub wrapped_vault_key_b64: String,
}

#[derive(Debug, Serialize)]
pub struct CreateVaultResp {
    pub id: String,
    pub etag: String,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct PutVaultReq {
    pub ciphertext_b64: String,
    pub expected_etag: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PutVaultResp {
    pub etag: String,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct RenameVaultReq {
    pub name: String,
}

pub async fn create_vault(
    State(state): State<AuthState>,
    Extension(user): Extension<AuthenticatedUserId>,
    Json(req): Json<CreateVaultReq>,
) -> ApiResult<Json<CreateVaultResp>> {
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("vault name required".into()));
    }
    if req.ciphertext_b64.is_empty() {
        return Err(ApiError::BadRequest("empty vault blob".into()));
    }
    if req.wrapped_vault_key_b64.len() < 24 {
        return Err(ApiError::BadRequest("wrapped_vault_key missing".into()));
    }
    let etag = make_etag_for_blob(&req.ciphertext_b64);
    let vault = state.storage.create_vault(
        &user.0,
        req.name.trim(),
        &req.ciphertext_b64,
        &etag,
        &req.wrapped_vault_key_b64,
    )?;
    Ok(Json(CreateVaultResp {
        id: vault.id,
        etag: vault.etag,
        updated_at: vault.updated_at,
    }))
}

pub async fn list_vaults(
    State(state): State<AuthState>,
    Extension(user): Extension<AuthenticatedUserId>,
) -> ApiResult<Json<ListVaultsResp>> {
    let entries = state.storage.list_vaults_for_user(&user.0)?;
    let vaults = entries
        .into_iter()
        .map(|e| VaultSummary {
            id: e.vault.id,
            name: e.vault.name,
            role: e.role,
            owner_user_id: e.vault.owner_user_id,
            etag: e.vault.etag,
            created_at: e.vault.created_at,
            updated_at: e.vault.updated_at,
            wrapped_vault_key_b64: e.wrapped_vault_key_b64,
        })
        .collect();
    Ok(Json(ListVaultsResp { vaults }))
}

pub async fn get_vault(
    State(state): State<AuthState>,
    Extension(user): Extension<AuthenticatedUserId>,
    Path(vault_id): Path<String>,
) -> ApiResult<(HeaderMap, Json<VaultFull>)> {
    let entry = state
        .storage
        .get_vault_for_user(&user.0, &vault_id)?
        .ok_or(ApiError::NotFound)?;
    let mut headers = HeaderMap::new();
    headers.insert("ETag", entry.vault.etag.parse().unwrap());
    Ok((
        headers,
        Json(VaultFull {
            id: entry.vault.id,
            name: entry.vault.name,
            role: entry.role,
            owner_user_id: entry.vault.owner_user_id,
            etag: entry.vault.etag,
            created_at: entry.vault.created_at,
            updated_at: entry.vault.updated_at,
            wrapped_vault_key_b64: entry.wrapped_vault_key_b64,
            ciphertext_b64: entry.vault.ciphertext_b64,
        }),
    ))
}

pub async fn put_vault(
    State(state): State<AuthState>,
    Extension(user): Extension<AuthenticatedUserId>,
    Path(vault_id): Path<String>,
    Json(req): Json<PutVaultReq>,
) -> ApiResult<Json<PutVaultResp>> {
    if req.ciphertext_b64.is_empty() {
        return Err(ApiError::BadRequest("empty vault blob".into()));
    }
    let new_etag = make_etag_for_blob(&req.ciphertext_b64);
    let updated = state.storage.update_vault_ciphertext(
        &user.0,
        &vault_id,
        &req.ciphertext_b64,
        &new_etag,
        req.expected_etag.as_deref(),
    )?;
    Ok(Json(PutVaultResp {
        etag: updated.etag,
        updated_at: updated.updated_at,
    }))
}

pub async fn rename_vault(
    State(state): State<AuthState>,
    Extension(user): Extension<AuthenticatedUserId>,
    Path(vault_id): Path<String>,
    Json(req): Json<RenameVaultReq>,
) -> ApiResult<()> {
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("vault name required".into()));
    }
    state
        .storage
        .rename_vault(&user.0, &vault_id, req.name.trim())?;
    Ok(())
}

pub async fn delete_vault(
    State(state): State<AuthState>,
    Extension(user): Extension<AuthenticatedUserId>,
    Path(vault_id): Path<String>,
) -> ApiResult<()> {
    state.storage.delete_vault(&user.0, &vault_id)?;
    Ok(())
}
