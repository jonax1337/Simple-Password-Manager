//! Tauri commands wiring the cloud-sync session into the UI.
//!
//! Flow per command:
//!   - `cloud_signup`:  generate master_key + recovery code → wrap both → upload → return recovery code (shown once)
//!   - `cloud_login`:   download {salt, wrapped_master_key} → unwrap with password → POST /auth/login
//!   - `cloud_recover`: download recovery_blob → unwrap with recovery code → re-wrap with new password → /auth/reset
//!   - `cloud_push`:    seal current kdbx with vault_key → PUT /vault with expected_etag
//!   - `cloud_pull`:    GET /vault → open with vault_key → write kdbx to disk
//!   - `cloud_status`:  read-only snapshot for the settings UI
//!   - `cloud_disconnect`: drop session (forget vault_key + token)

use serde::Serialize;
use simple_password_manager::cloud::{
    client::{CloudClient, CloudError},
    crypto::{
        self, b64_decode, b64_encode, derive_auth_hash, derive_password_key, derive_recovery_key,
        derive_vault_key, generate_master_key, generate_recovery_code, new_kdf_salt, open_vault,
        seal_vault, unwrap_key, wrap_key, SecretKey,
    },
    CloudSession,
};
use simple_password_manager::state::AppState;
use std::fs;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct CloudStatusResp {
    pub linked: bool,
    pub server_url: Option<String>,
    pub email: Option<String>,
    pub has_remote_vault: bool,
}

#[derive(Debug, Serialize)]
pub struct CloudSignupResp {
    pub email: String,
    pub server_url: String,
    /// Shown to the user ONCE on signup. Lost = no password recovery.
    pub recovery_code: String,
}

#[derive(Debug, Serialize)]
pub struct CloudActionResp {
    pub email: String,
    pub server_url: String,
}

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

#[tauri::command]
pub async fn cloud_signup(
    state: State<'_, AppState>,
    server_url: String,
    email: String,
    master_password: String,
) -> Result<CloudSignupResp, String> {
    let db_path = current_db_path(&state)?;

    // 1. Generate the canonical secrets. master_key is the long-lived
    //    encryption root; password_key is just a wrapper for it.
    let master_key = generate_master_key();
    let salt = new_kdf_salt();
    let password_key =
        derive_password_key(&master_password, &salt).map_err(map_crypto_err)?;
    let wrapped_master = wrap_key(&password_key, &master_key).map_err(map_crypto_err)?;

    // 2. Recovery: deterministic per-email salt means the printed code is
    //    the only thing the user has to keep.
    let recovery_code = generate_recovery_code();
    let recovery_key = derive_recovery_key(&recovery_code, &email).map_err(map_crypto_err)?;
    let recovery_blob = wrap_key(&recovery_key, &master_key).map_err(map_crypto_err)?;

    // 3. Auth + vault derivations from master_key (NOT password_key).
    let auth_hash = derive_auth_hash(&master_key);
    let vault_key = derive_vault_key(&master_key);

    // 4. Encrypt the on-disk kdbx for upload. KeePass's own AES is intact
    //    underneath — this is a second layer keyed by master_key.
    let kdbx_bytes =
        fs::read(&db_path).map_err(|e| format!("Failed to read kdbx for upload: {}", e))?;
    let sealed_vault = seal_vault(&vault_key, &kdbx_bytes).map_err(map_crypto_err)?;

    let client = CloudClient::new(server_url.clone());
    let resp = client
        .signup(
            &email,
            &b64_encode(&salt),
            &b64_encode(auth_hash.as_bytes()),
            &b64_encode(&wrapped_master),
            &b64_encode(&recovery_blob),
            &b64_encode(&sealed_vault),
        )
        .await
        .map_err(map_cloud_err)?;

    store_session(
        &state,
        CloudSession::new(
            server_url.clone(),
            email.clone(),
            resp.user_id,
            resp.token,
            vault_key,
            None,
        ),
    )?;

    Ok(CloudSignupResp {
        email,
        server_url,
        recovery_code,
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

    // Public — anyone can request kdf params. Wrong password → unwrap fails
    // a few lines down with a generic "decryption failed", so the server
    // never sees a failed login attempt unless the password was right.
    let params = client.kdf_params(&email).await.map_err(map_cloud_err)?;
    let salt = b64_decode(&params.kdf_salt_b64).map_err(map_crypto_err)?;
    let wrapped_master = b64_decode(&params.wrapped_master_key_b64).map_err(map_crypto_err)?;

    let password_key =
        derive_password_key(&master_password, &salt).map_err(map_crypto_err)?;
    let master_key = unwrap_key(&password_key, &wrapped_master).map_err(map_crypto_err)?;

    let auth_hash = derive_auth_hash(&master_key);
    let vault_key = derive_vault_key(&master_key);

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
            vault_key,
            None,
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

    // Re-wrap with the new password.
    let new_salt = new_kdf_salt();
    let new_password_key =
        derive_password_key(&new_master_password, &new_salt).map_err(map_crypto_err)?;
    let new_wrapped = wrap_key(&new_password_key, &master_key).map_err(map_crypto_err)?;

    let auth_hash = derive_auth_hash(&master_key);
    let vault_key = derive_vault_key(&master_key);

    let resp = client
        .reset_password(
            &email,
            &b64_encode(auth_hash.as_bytes()),
            &b64_encode(&new_salt),
            &b64_encode(&new_wrapped),
        )
        .await
        .map_err(map_cloud_err)?;

    // We don't know the user_id from the reset response (we could plumb it
    // through but it doesn't gate anything in the client). Reuse the
    // session shape with an empty id.
    store_session(
        &state,
        CloudSession::new(
            server_url.clone(),
            email.clone(),
            String::new(),
            resp.token,
            vault_key,
            None,
        ),
    )?;

    Ok(CloudActionResp { email, server_url })
}

#[tauri::command]
pub async fn cloud_push(state: State<'_, AppState>) -> Result<(), String> {
    let db_path = current_db_path(&state)?;
    let kdbx_bytes = fs::read(&db_path)
        .map_err(|e| format!("Failed to read kdbx for push: {}", e))?;

    let (server_url, token, expected_etag, sealed_b64) = {
        let cloud = state
            .cloud
            .lock()
            .map_err(|_| "cloud slot poisoned".to_string())?;
        let s = cloud
            .as_ref()
            .ok_or_else(|| "Cloud account not linked".to_string())?;
        let sealed = seal_vault(&s.vault_key, &kdbx_bytes).map_err(map_crypto_err)?;
        (
            s.server_url.clone(),
            s.token.clone(),
            s.last_known_etag.clone(),
            b64_encode(&sealed),
        )
    };

    let client = CloudClient::new(server_url);
    let resp = client
        .put_vault(&token, &sealed_b64, expected_etag.as_deref())
        .await
        .map_err(map_cloud_err)?;

    if let Ok(mut cloud) = state.cloud.lock() {
        if let Some(s) = cloud.as_mut() {
            s.last_known_etag = Some(resp.etag);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn cloud_pull(state: State<'_, AppState>) -> Result<(), String> {
    let db_path = current_db_path(&state)?;

    let (server_url, token, vault_key_bytes) = {
        let cloud = state
            .cloud
            .lock()
            .map_err(|_| "cloud slot poisoned".to_string())?;
        let s = cloud
            .as_ref()
            .ok_or_else(|| "Cloud account not linked".to_string())?;
        (
            s.server_url.clone(),
            s.token.clone(),
            *s.vault_key.as_bytes(),
        )
    };

    let client = CloudClient::new(server_url);
    let resp = client.get_vault(&token).await.map_err(map_cloud_err)?;

    let sealed = b64_decode(&resp.ciphertext_b64).map_err(map_crypto_err)?;
    let vault_key = SecretKey::from_bytes(vault_key_bytes);
    let plaintext = open_vault(&vault_key, &sealed).map_err(map_crypto_err)?;

    fs::write(&db_path, &plaintext)
        .map_err(|e| format!("Failed to write pulled kdbx: {}", e))?;

    if let Ok(mut cloud) = state.cloud.lock() {
        if let Some(s) = cloud.as_mut() {
            s.last_known_etag = Some(resp.etag);
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
            has_remote_vault: s.last_known_etag.is_some(),
        },
        None => CloudStatusResp {
            linked: false,
            server_url: None,
            email: None,
            has_remote_vault: false,
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
