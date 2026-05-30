//! Tauri commands wiring the cloud-sync session into the UI.
//!
//! Flow per command:
//!   - `cloud_signup`:  derive keys → POST /auth/signup with sealed kdbx → store session
//!   - `cloud_login`:   GET salt → derive keys → POST /auth/login → store session
//!   - `cloud_push`:    seal current kdbx with vault_key → PUT /vault with expected_etag
//!   - `cloud_pull`:    GET /vault → open with vault_key → write kdbx to disk
//!   - `cloud_status`:  read-only snapshot of linked-ness for the settings UI
//!   - `cloud_disconnect`: drop session (forget vault_key + token)

use serde::Serialize;
use simple_password_manager::cloud::{
    client::{CloudClient, CloudError},
    crypto::{
        self, b64_decode, b64_encode, derive_auth_hash, derive_master_key, derive_vault_key,
        new_kdf_salt, open_vault, seal_vault,
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

#[tauri::command]
pub async fn cloud_signup(
    state: State<'_, AppState>,
    server_url: String,
    email: String,
    master_password: String,
) -> Result<CloudActionResp, String> {
    let db_path = current_db_path(&state)?;

    // Derive keys client-side. Master key never leaves this function.
    let salt = new_kdf_salt();
    let master = derive_master_key(&master_password, &salt).map_err(map_crypto_err)?;
    let auth_hash = derive_auth_hash(&master);
    let vault_key = derive_vault_key(&master);

    // Seal the on-disk kdbx as the initial blob. The file is already AES-
    // encrypted by KeePass — wrapping it again with vault_key means the
    // server holds *two* layers of encryption, both derived from the
    // master password.
    let kdbx_bytes =
        fs::read(&db_path).map_err(|e| format!("Failed to read kdbx for upload: {}", e))?;
    let sealed = seal_vault(&vault_key, &kdbx_bytes).map_err(map_crypto_err)?;

    let client = CloudClient::new(server_url.clone());
    let resp = client
        .signup(
            &email,
            &b64_encode(&salt),
            &b64_encode(auth_hash.as_bytes()),
            &b64_encode(&sealed),
        )
        .await
        .map_err(map_cloud_err)?;

    let session = CloudSession::new(
        server_url.clone(),
        email.clone(),
        resp.user_id,
        resp.token,
        vault_key,
        None, // initial put gets its own etag we'll learn on next push
    );
    *state
        .cloud
        .lock()
        .map_err(|_| "cloud slot poisoned".to_string())? = Some(session);

    Ok(CloudActionResp {
        email,
        server_url,
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

    // The salt is public — we pull it before computing anything. This means
    // login on a fresh device only needs email + master_password.
    let params = client.kdf_params(&email).await.map_err(map_cloud_err)?;
    let salt = b64_decode(&params.kdf_salt_b64).map_err(map_crypto_err)?;

    let master = derive_master_key(&master_password, &salt).map_err(map_crypto_err)?;
    let auth_hash = derive_auth_hash(&master);
    let vault_key = derive_vault_key(&master);

    let resp = client
        .login(&email, &b64_encode(auth_hash.as_bytes()))
        .await
        .map_err(map_cloud_err)?;

    let session = CloudSession::new(
        server_url.clone(),
        email.clone(),
        resp.user_id,
        resp.token,
        vault_key,
        None,
    );
    *state
        .cloud
        .lock()
        .map_err(|_| "cloud slot poisoned".to_string())? = Some(session);

    Ok(CloudActionResp {
        email,
        server_url,
    })
}

#[tauri::command]
pub async fn cloud_push(state: State<'_, AppState>) -> Result<(), String> {
    let db_path = current_db_path(&state)?;
    let kdbx_bytes = fs::read(&db_path)
        .map_err(|e| format!("Failed to read kdbx for push: {}", e))?;

    // Snapshot the values we need so we can release the lock before doing
    // network IO. Re-acquire afterwards to write the new etag back.
    let (server_url, token, expected_etag, sealed_b64) = {
        let cloud = state
            .cloud
            .lock()
            .map_err(|_| "cloud slot poisoned".to_string())?;
        let s = cloud.as_ref().ok_or_else(|| "Cloud account not linked".to_string())?;
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
        let s = cloud.as_ref().ok_or_else(|| "Cloud account not linked".to_string())?;
        (
            s.server_url.clone(),
            s.token.clone(),
            *s.vault_key.as_bytes(),
        )
    };

    let client = CloudClient::new(server_url);
    let resp = client.get_vault(&token).await.map_err(map_cloud_err)?;

    let sealed = b64_decode(&resp.ciphertext_b64).map_err(map_crypto_err)?;
    // Re-wrap raw bytes into a SecretKey just for the open call. Drops at
    // end of scope and zeroes.
    let key_owned = {
        struct OwnedKey([u8; 32]);
        impl Drop for OwnedKey {
            fn drop(&mut self) {
                use zeroize::Zeroize;
                self.0.zeroize();
            }
        }
        OwnedKey(vault_key_bytes)
    };
    // We need a SecretKey for open_vault; reconstruct via a small helper.
    let vault_key = crypto_from_bytes(key_owned.0);

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

// Re-borrow helper: SecretKey is a private wrapper; we need to reconstruct
// one from raw bytes to call open_vault. Done via the public constructor in
// crypto.rs. If the API ever closes off direct construction, this becomes a
// `pub fn from_bytes` in crypto.rs.
fn crypto_from_bytes(bytes: [u8; 32]) -> simple_password_manager::cloud::crypto::SecretKey {
    simple_password_manager::cloud::crypto::SecretKey::from_bytes(bytes)
}
