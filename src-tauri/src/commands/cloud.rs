//! Tauri commands for the multi-vault cloud sync.
//!
//! Flow:
//!   - signup   → generate master_key + first vault_key, wrap both, upload kdbx + recovery
//!   - login    → unwrap master_key (no active vault yet)
//!   - recover  → unwrap via recovery_code → re-wrap with new password
//!   - list_vaults                          → vault picker rows
//!   - open_vault(vault_id, target_path)    → download, decrypt, write to target_path,
//!                                            mark vault active for push/pull
//!   - push     → uses active vault's key + etag
//!   - pull     → uses active vault, replaces local kdbx
//!   - create_vault(name)                   → seal current kdbx as a new server vault
//!   - rename_vault / delete_vault          → metadata ops (owner-only)
//!   - disconnect                           → drop session

use serde::Serialize;
use simple_password_manager::cloud::{
    client::{CloudClient, CloudError, SignupArgs, VaultSummary},
    crypto::{
        self, b64_decode, b64_encode, derive_auth_hash, derive_password_key, derive_recovery_key,
        generate_master_key, generate_recovery_code, new_kdf_salt, open_vault, seal_vault,
        unwrap_key, wrap_key, SecretKey,
    },
    session::ActiveVault,
    CloudSession,
};
use simple_password_manager::state::AppState;
use std::fs;
use tauri::State;

// ----- response payloads -----

#[derive(Debug, Serialize)]
pub struct CloudStatusResp {
    pub linked: bool,
    pub server_url: Option<String>,
    pub email: Option<String>,
    pub active_vault_id: Option<String>,
    pub active_vault_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CloudSignupResp {
    pub email: String,
    pub server_url: String,
    pub recovery_code: String,
    pub initial_vault_id: String,
}

#[derive(Debug, Serialize)]
pub struct CloudActionResp {
    pub email: String,
    pub server_url: String,
}

#[derive(Debug, Serialize)]
pub struct CloudVaultEntry {
    pub id: String,
    pub name: String,
    pub role: String,
    pub owner_user_id: String,
    pub etag: String,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct CloudCreateVaultResp {
    pub id: String,
}

// ----- helpers -----

fn map_cloud_err(e: CloudError) -> String {
    e.to_string()
}

fn map_crypto_err(e: crypto::CryptoError) -> String {
    e.to_string()
}

fn current_db_path(state: &AppState) -> Result<std::path::PathBuf, String> {
    let lock = state
        .database
        .lock()
        .map_err(|_| "database state poisoned".to_string())?;
    let path = lock
        .as_ref()
        .ok_or_else(|| "No database loaded".to_string())?
        .path
        .clone();
    Ok(path)
}

fn store_session(state: &AppState, session: CloudSession) -> Result<(), String> {
    *state
        .cloud
        .lock()
        .map_err(|_| "cloud slot poisoned".to_string())? = Some(session);
    Ok(())
}

/// Read master_key bytes without holding the lock across the entire command —
/// callers do their network IO afterwards.
fn snapshot_token_and_master(
    state: &AppState,
) -> Result<(String, String, [u8; 32]), String> {
    let cloud = state
        .cloud
        .lock()
        .map_err(|_| "cloud slot poisoned".to_string())?;
    let s = cloud
        .as_ref()
        .ok_or_else(|| "Cloud account not linked".to_string())?;
    Ok((
        s.server_url.clone(),
        s.token.clone(),
        *s.master_key.as_bytes(),
    ))
}

fn snapshot_active_vault(
    state: &AppState,
) -> Result<(String, String, String, [u8; 32], Option<String>), String> {
    let cloud = state
        .cloud
        .lock()
        .map_err(|_| "cloud slot poisoned".to_string())?;
    let s = cloud
        .as_ref()
        .ok_or_else(|| "Cloud account not linked".to_string())?;
    let v = s
        .active_vault
        .as_ref()
        .ok_or_else(|| "No active vault — pick one from the cloud picker".to_string())?;
    Ok((
        s.server_url.clone(),
        s.token.clone(),
        v.id.clone(),
        *v.vault_key.as_bytes(),
        v.last_known_etag.clone(),
    ))
}

// ----- commands -----

#[tauri::command]
pub async fn cloud_signup(
    state: State<'_, AppState>,
    server_url: String,
    email: String,
    master_password: String,
    initial_vault_name: Option<String>,
) -> Result<CloudSignupResp, String> {
    let db_path = current_db_path(&state)?;

    // 1. Generate the canonical secrets.
    let master_key = generate_master_key();
    let salt = new_kdf_salt();
    let password_key =
        derive_password_key(&master_password, &salt).map_err(map_crypto_err)?;
    let wrapped_master = wrap_key(&password_key, &master_key).map_err(map_crypto_err)?;

    // 2. Recovery code.
    let recovery_code = generate_recovery_code();
    let recovery_key = derive_recovery_key(&recovery_code, &email).map_err(map_crypto_err)?;
    let recovery_blob = wrap_key(&recovery_key, &master_key).map_err(map_crypto_err)?;

    // 3. First vault: random vault_key, wrap with master_key.
    let vault_key = generate_master_key(); // same primitive — 32 random bytes
    let wrapped_vault_key = wrap_key(&master_key, &vault_key).map_err(map_crypto_err)?;

    // 4. Seal the on-disk kdbx with vault_key for upload.
    let kdbx_bytes = fs::read(&db_path)
        .map_err(|e| format!("Failed to read kdbx for upload: {}", e))?;
    let sealed_vault = seal_vault(&vault_key, &kdbx_bytes).map_err(map_crypto_err)?;

    // 5. Auth proof from master_key (NOT password — server can verify
    //    possession of master_key without ever seeing the password).
    let auth_hash = derive_auth_hash(&master_key);

    let vault_name = initial_vault_name
        .as_deref()
        .map(|n| n.trim())
        .filter(|n| !n.is_empty())
        .unwrap_or("Personal");

    let client = CloudClient::new(server_url.clone());
    let resp = client
        .signup(SignupArgs {
            email: &email,
            kdf_salt_b64: &b64_encode(&salt),
            client_auth_hash: &b64_encode(auth_hash.as_bytes()),
            wrapped_master_key_b64: &b64_encode(&wrapped_master),
            recovery_blob_b64: &b64_encode(&recovery_blob),
            // account_pubkey + privkey stay empty until sharing lands.
            account_pubkey_b64: "",
            wrapped_account_privkey_b64: "",
            initial_vault_name: vault_name,
            initial_vault_blob_b64: &b64_encode(&sealed_vault),
            initial_wrapped_vault_key_b64: &b64_encode(&wrapped_vault_key),
        })
        .await
        .map_err(map_cloud_err)?;

    // Re-fetch to discover the assigned vault id (signup response gives us
    // user_id + token only — vault id lives in the listing).
    let vaults = client
        .list_vaults(&resp.token)
        .await
        .map_err(map_cloud_err)?;
    let first_vault = vaults
        .vaults
        .into_iter()
        .next()
        .ok_or_else(|| "Server accepted signup but returned no vaults".to_string())?;

    let mut session = CloudSession::new(
        server_url.clone(),
        email.clone(),
        resp.user_id,
        resp.token,
        master_key,
    );
    session.active_vault = Some(ActiveVault {
        id: first_vault.id.clone(),
        name: first_vault.name,
        vault_key,
        last_known_etag: Some(first_vault.etag),
    });
    store_session(&state, session)?;

    Ok(CloudSignupResp {
        email,
        server_url,
        recovery_code,
        initial_vault_id: first_vault.id,
    })
}

#[tauri::command]
pub async fn cloud_login(
    state: State<'_, AppState>,
    server_url: String,
    email: String,
    master_password: String,
) -> Result<CloudActionResp, String> {
    let client = CloudClient::new(server_url.clone());

    let params = client.kdf_params(&email).await.map_err(map_cloud_err)?;
    let salt = b64_decode(&params.kdf_salt_b64).map_err(map_crypto_err)?;
    let wrapped_master = b64_decode(&params.wrapped_master_key_b64).map_err(map_crypto_err)?;

    let password_key =
        derive_password_key(&master_password, &salt).map_err(map_crypto_err)?;
    let master_key = unwrap_key(&password_key, &wrapped_master).map_err(map_crypto_err)?;

    let auth_hash = derive_auth_hash(&master_key);
    let resp = client
        .login(&email, &b64_encode(auth_hash.as_bytes()))
        .await
        .map_err(map_cloud_err)?;

    store_session(
        &state,
        CloudSession::new(
            server_url.clone(),
            email.clone(),
            resp.user_id,
            resp.token,
            master_key,
        ),
    )?;

    Ok(CloudActionResp { email, server_url })
}

#[tauri::command]
pub async fn cloud_recover(
    state: State<'_, AppState>,
    server_url: String,
    email: String,
    recovery_code: String,
    new_master_password: String,
) -> Result<CloudActionResp, String> {
    let client = CloudClient::new(server_url.clone());

    let init = client
        .recovery_init(&email)
        .await
        .map_err(map_cloud_err)?;
    let recovery_blob = b64_decode(&init.recovery_blob_b64).map_err(map_crypto_err)?;

    let recovery_key = derive_recovery_key(&recovery_code, &email).map_err(map_crypto_err)?;
    let master_key = unwrap_key(&recovery_key, &recovery_blob).map_err(map_crypto_err)?;

    let new_salt = new_kdf_salt();
    let new_password_key =
        derive_password_key(&new_master_password, &new_salt).map_err(map_crypto_err)?;
    let new_wrapped = wrap_key(&new_password_key, &master_key).map_err(map_crypto_err)?;

    let auth_hash = derive_auth_hash(&master_key);
    let resp = client
        .reset_password(
            &email,
            &b64_encode(auth_hash.as_bytes()),
            &b64_encode(&new_salt),
            &b64_encode(&new_wrapped),
        )
        .await
        .map_err(map_cloud_err)?;

    store_session(
        &state,
        CloudSession::new(
            server_url.clone(),
            email.clone(),
            String::new(),
            resp.token,
            master_key,
        ),
    )?;

    Ok(CloudActionResp { email, server_url })
}

#[tauri::command]
pub async fn cloud_list_vaults(
    state: State<'_, AppState>,
) -> Result<Vec<CloudVaultEntry>, String> {
    let (server_url, token, _) = snapshot_token_and_master(&state)?;
    let client = CloudClient::new(server_url);
    let resp = client.list_vaults(&token).await.map_err(map_cloud_err)?;
    Ok(resp
        .vaults
        .into_iter()
        .map(|v: VaultSummary| CloudVaultEntry {
            id: v.id,
            name: v.name,
            role: v.role,
            owner_user_id: v.owner_user_id,
            etag: v.etag,
            updated_at: v.updated_at,
        })
        .collect())
}

/// Download a vault, decrypt with master_key → vault_key → kdbx bytes,
/// write to `target_path`. Sets active_vault so subsequent push/pull
/// flows through this vault.
#[tauri::command]
pub async fn cloud_open_vault(
    state: State<'_, AppState>,
    vault_id: String,
    target_path: String,
) -> Result<(), String> {
    let (server_url, token, master_bytes) = snapshot_token_and_master(&state)?;

    let client = CloudClient::new(server_url);
    let full = client
        .get_vault(&token, &vault_id)
        .await
        .map_err(map_cloud_err)?;

    // Reconstruct master_key briefly so we can unwrap the vault_key.
    let master_key = SecretKey::from_bytes(master_bytes);
    let wrapped_vk = b64_decode(&full.wrapped_vault_key_b64).map_err(map_crypto_err)?;
    let vault_key = unwrap_key(&master_key, &wrapped_vk).map_err(map_crypto_err)?;

    let sealed = b64_decode(&full.ciphertext_b64).map_err(map_crypto_err)?;
    let plaintext = open_vault(&vault_key, &sealed).map_err(map_crypto_err)?;

    if let Some(parent) = std::path::Path::new(&target_path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&target_path, &plaintext)
        .map_err(|e| format!("Failed to write vault to {}: {}", target_path, e))?;

    let mut cloud = state
        .cloud
        .lock()
        .map_err(|_| "cloud slot poisoned".to_string())?;
    if let Some(s) = cloud.as_mut() {
        s.active_vault = Some(ActiveVault {
            id: full.id,
            name: full.name,
            vault_key,
            last_known_etag: Some(full.etag),
        });
    }
    Ok(())
}

/// Upload the in-app database to the active vault. CAS on expected_etag —
/// surface 409s so the UI can pull first and merge.
#[tauri::command]
pub async fn cloud_push(state: State<'_, AppState>) -> Result<(), String> {
    let db_path = current_db_path(&state)?;
    let kdbx_bytes = fs::read(&db_path)
        .map_err(|e| format!("Failed to read kdbx for push: {}", e))?;

    let (server_url, token, vault_id, vault_key_bytes, expected_etag) =
        snapshot_active_vault(&state)?;
    let vault_key = SecretKey::from_bytes(vault_key_bytes);
    let sealed = seal_vault(&vault_key, &kdbx_bytes).map_err(map_crypto_err)?;
    let sealed_b64 = b64_encode(&sealed);

    let client = CloudClient::new(server_url);
    let resp = client
        .put_vault(&token, &vault_id, &sealed_b64, expected_etag.as_deref())
        .await
        .map_err(map_cloud_err)?;

    if let Ok(mut cloud) = state.cloud.lock() {
        if let Some(s) = cloud.as_mut() {
            if let Some(av) = s.active_vault.as_mut() {
                av.last_known_etag = Some(resp.etag);
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn cloud_pull(state: State<'_, AppState>) -> Result<(), String> {
    let db_path = current_db_path(&state)?;
    let (server_url, token, vault_id, vault_key_bytes, _) = snapshot_active_vault(&state)?;
    let vault_key = SecretKey::from_bytes(vault_key_bytes);

    let client = CloudClient::new(server_url);
    let full = client
        .get_vault(&token, &vault_id)
        .await
        .map_err(map_cloud_err)?;

    let sealed = b64_decode(&full.ciphertext_b64).map_err(map_crypto_err)?;
    let plaintext = open_vault(&vault_key, &sealed).map_err(map_crypto_err)?;
    fs::write(&db_path, &plaintext)
        .map_err(|e| format!("Failed to write pulled kdbx: {}", e))?;

    if let Ok(mut cloud) = state.cloud.lock() {
        if let Some(s) = cloud.as_mut() {
            if let Some(av) = s.active_vault.as_mut() {
                av.last_known_etag = Some(full.etag);
            }
        }
    }
    Ok(())
}

/// Create a new vault from the currently-open local DB. Mints a fresh
/// vault_key, wraps with master_key, seals + uploads kdbx. The new vault
/// becomes active.
#[tauri::command]
pub async fn cloud_create_vault(
    state: State<'_, AppState>,
    name: String,
) -> Result<CloudCreateVaultResp, String> {
    let db_path = current_db_path(&state)?;
    let kdbx_bytes = fs::read(&db_path)
        .map_err(|e| format!("Failed to read kdbx: {}", e))?;

    let (server_url, token, master_bytes) = snapshot_token_and_master(&state)?;
    let master_key = SecretKey::from_bytes(master_bytes);

    let vault_key = generate_master_key();
    let wrapped_vk = wrap_key(&master_key, &vault_key).map_err(map_crypto_err)?;
    let sealed = seal_vault(&vault_key, &kdbx_bytes).map_err(map_crypto_err)?;

    let client = CloudClient::new(server_url);
    let resp = client
        .create_vault(
            &token,
            name.trim(),
            &b64_encode(&sealed),
            &b64_encode(&wrapped_vk),
        )
        .await
        .map_err(map_cloud_err)?;

    if let Ok(mut cloud) = state.cloud.lock() {
        if let Some(s) = cloud.as_mut() {
            s.active_vault = Some(ActiveVault {
                id: resp.id.clone(),
                name: name.trim().to_string(),
                vault_key,
                last_known_etag: Some(resp.etag),
            });
        }
    }
    Ok(CloudCreateVaultResp { id: resp.id })
}

#[tauri::command]
pub async fn cloud_rename_vault(
    state: State<'_, AppState>,
    vault_id: String,
    name: String,
) -> Result<(), String> {
    let (server_url, token, _) = snapshot_token_and_master(&state)?;
    let client = CloudClient::new(server_url);
    client
        .rename_vault(&token, &vault_id, name.trim())
        .await
        .map_err(map_cloud_err)?;
    Ok(())
}

#[tauri::command]
pub async fn cloud_delete_vault(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<(), String> {
    let (server_url, token, _) = snapshot_token_and_master(&state)?;
    let client = CloudClient::new(server_url);
    client
        .delete_vault(&token, &vault_id)
        .await
        .map_err(map_cloud_err)?;
    // Drop active_vault if we just deleted it.
    if let Ok(mut cloud) = state.cloud.lock() {
        if let Some(s) = cloud.as_mut() {
            if s.active_vault.as_ref().map(|v| v.id == vault_id).unwrap_or(false) {
                s.active_vault = None;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn cloud_status(state: State<'_, AppState>) -> Result<CloudStatusResp, String> {
    let cloud = state
        .cloud
        .lock()
        .map_err(|_| "cloud slot poisoned".to_string())?;
    Ok(match cloud.as_ref() {
        Some(s) => CloudStatusResp {
            linked: true,
            server_url: Some(s.server_url.clone()),
            email: Some(s.email.clone()),
            active_vault_id: s.active_vault.as_ref().map(|v| v.id.clone()),
            active_vault_name: s.active_vault.as_ref().map(|v| v.name.clone()),
        },
        None => CloudStatusResp {
            linked: false,
            server_url: None,
            email: None,
            active_vault_id: None,
            active_vault_name: None,
        },
    })
}

#[tauri::command]
pub fn cloud_disconnect(state: State<'_, AppState>) -> Result<(), String> {
    *state
        .cloud
        .lock()
        .map_err(|_| "cloud slot poisoned".to_string())? = None;
    Ok(())
}
