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
        generate_account_keypair, generate_master_key, generate_recovery_code, new_kdf_salt,
        open_vault, seal_vault, sealed_open, sealed_seal, unwrap_key, wrap_key, AccountKeypair,
        SecretKey,
    },
    session::ActiveVault,
    CloudSession,
};
use simple_password_manager::kdbx::Database;
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
    lock.as_ref()
        .ok_or_else(|| "No database loaded".to_string())?
        .path
        .clone()
        .ok_or_else(|| "Cloud-only vault: no local file".to_string())
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

    // 3. Curve25519 account keypair for future vault sharing. Wrap the
    //    private half with master_key so any device can reproduce it after
    //    login. Public half goes to the server in the clear.
    let keypair = generate_account_keypair();
    let privkey_secret =
        SecretKey::from_bytes(*keypair.secret_bytes());
    let wrapped_privkey = wrap_key(&master_key, &privkey_secret).map_err(map_crypto_err)?;

    // 4. First vault: random vault_key, wrap with master_key.
    let vault_key = generate_master_key(); // same primitive — 32 random bytes
    let wrapped_vault_key = wrap_key(&master_key, &vault_key).map_err(map_crypto_err)?;

    // 4. Source the kdbx bytes from in-memory state (cloud-only) or
    //    fall back to the file (legacy local mode).
    let kdbx_bytes = {
        let db_lock = state
            .database
            .lock()
            .map_err(|_| "database state poisoned".to_string())?;
        let db = db_lock
            .as_ref()
            .ok_or_else(|| "No database loaded".to_string())?;
        if db.is_cloud_only() {
            db.save_to_bytes().map_err(|e| e.to_string())?
        } else {
            let path = db
                .path
                .as_ref()
                .ok_or_else(|| "No DB path".to_string())?;
            fs::read(path).map_err(|e| format!("Failed to read kdbx: {}", e))?
        }
    };
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
            account_pubkey_b64: &b64_encode(&keypair.public_key),
            wrapped_account_privkey_b64: &b64_encode(&wrapped_privkey),
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
    session.account_keypair = Some(keypair);
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

    // Rehydrate the sharing keypair if the account has one. Pre-Stage-C
    // accounts return empty strings here and just don't get keypair
    // capability until they re-key.
    let keypair = if !resp.wrapped_account_privkey_b64.is_empty()
        && !resp.account_pubkey_b64.is_empty()
    {
        let wrapped = b64_decode(&resp.wrapped_account_privkey_b64).map_err(map_crypto_err)?;
        let pubkey_bytes = b64_decode(&resp.account_pubkey_b64).map_err(map_crypto_err)?;
        if pubkey_bytes.len() == 32 {
            let privkey_secret = unwrap_key(&master_key, &wrapped).map_err(map_crypto_err)?;
            let mut pk = [0u8; 32];
            pk.copy_from_slice(&pubkey_bytes);
            Some(AccountKeypair::from_bytes(pk, *privkey_secret.as_bytes()))
        } else {
            None
        }
    } else {
        None
    };

    let mut session = CloudSession::new(
        server_url.clone(),
        email.clone(),
        resp.user_id,
        resp.token,
        master_key,
    );
    session.account_keypair = keypair;
    store_session(&state, session)?;

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
/// open the KDBX in memory (cloud-only mode — no file on disk), and
/// store it as the active database. Subsequent push/pull operate on
/// the in-memory bytes.
///
/// `kdbx_password` is the KeePass-layer master password — separate from
/// the cloud master_password. The cloud crypto only undoes the outer
/// AES-GCM wrap; the KDBX layer underneath still needs its own key.
#[tauri::command]
pub async fn cloud_open_vault(
    state: State<'_, AppState>,
    vault_id: String,
    kdbx_password: String,
) -> Result<(), String> {
    let (server_url, token, master_bytes) = snapshot_token_and_master(&state)?;

    let client = CloudClient::new(server_url);
    let full = client
        .get_vault(&token, &vault_id)
        .await
        .map_err(map_cloud_err)?;

    // Choose the right unwrap path: owners use master_key (AES-GCM wrap);
    // shared members use the account_keypair (sealed_box). The server
    // tells us which by the membership row's `role`.
    let master_key = SecretKey::from_bytes(master_bytes);
    let wrapped_vk = b64_decode(&full.wrapped_vault_key_b64).map_err(map_crypto_err)?;
    let vault_key = if full.role == "owner" {
        unwrap_key(&master_key, &wrapped_vk).map_err(map_crypto_err)?
    } else {
        let cloud = state
            .cloud
            .lock()
            .map_err(|_| "cloud slot poisoned".to_string())?;
        let s = cloud
            .as_ref()
            .ok_or_else(|| "Cloud account not linked".to_string())?;
        let keypair = s.account_keypair.as_ref().ok_or_else(|| {
            "Shared vault cannot be opened: this account has no sharing keypair. \
             Sign up again or wait for the account-rekey flow."
                .to_string()
        })?;
        let plaintext = sealed_open(keypair, &wrapped_vk).map_err(map_crypto_err)?;
        if plaintext.len() != 32 {
            return Err("unsealed vault_key is not 32 bytes".to_string());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&plaintext);
        SecretKey::from_bytes(arr)
    };

    let sealed = b64_decode(&full.ciphertext_b64).map_err(map_crypto_err)?;
    let kdbx_bytes = open_vault(&vault_key, &sealed).map_err(map_crypto_err)?;

    // Open the kdbx straight from RAM — no fs::write. The vault never
    // touches disk on this machine.
    let db = Database::open_from_bytes(&kdbx_bytes, kdbx_password, None)
        .map_err(|e| e.to_string())?;

    // Atomic swap of in-memory DB + active_vault. Locks acquired in this
    // tight order: database first, then cloud. Any previous open DB is
    // replaced.
    {
        let mut db_lock = state
            .database
            .lock()
            .map_err(|_| "database state poisoned".to_string())?;
        *db_lock = Some(db);
    }
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
///
/// For cloud-only vaults we re-serialize the in-memory KDBX to bytes via
/// `save_to_bytes` — no fs::read, no file ever exists on disk for these.
/// Legacy local-file vaults still read from disk for backward-compat with
/// users who linked their existing local kdbx to the cloud.
#[tauri::command]
pub async fn cloud_push(state: State<'_, AppState>) -> Result<(), String> {
    let kdbx_bytes = {
        let db_lock = state
            .database
            .lock()
            .map_err(|_| "database state poisoned".to_string())?;
        let db = db_lock
            .as_ref()
            .ok_or_else(|| "No database loaded".to_string())?;
        if db.is_cloud_only() {
            db.save_to_bytes().map_err(|e| e.to_string())?
        } else {
            let path = db
                .path
                .as_ref()
                .ok_or_else(|| "No DB path for legacy push".to_string())?;
            fs::read(path).map_err(|e| format!("Failed to read kdbx for push: {}", e))?
        }
    };

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
    let (server_url, token, vault_id, vault_key_bytes, _) = snapshot_active_vault(&state)?;
    let vault_key = SecretKey::from_bytes(vault_key_bytes);

    let client = CloudClient::new(server_url);
    let full = client
        .get_vault(&token, &vault_id)
        .await
        .map_err(map_cloud_err)?;

    let sealed = b64_decode(&full.ciphertext_b64).map_err(map_crypto_err)?;
    let kdbx_bytes = open_vault(&vault_key, &sealed).map_err(map_crypto_err)?;

    // For cloud-only vaults: rebuild the in-memory DB from the freshly
    // pulled bytes, preserving the KDBX-layer password. For legacy
    // local-file vaults: write to disk and let the file watcher trigger
    // a reload via the existing merge path.
    let is_cloud_only = {
        let db_lock = state
            .database
            .lock()
            .map_err(|_| "database state poisoned".to_string())?;
        db_lock
            .as_ref()
            .map(|d| d.is_cloud_only())
            .unwrap_or(false)
    };

    if is_cloud_only {
        // Re-open via Database::open_from_bytes. We need the kdbx
        // password from the existing Database instance.
        use secrecy::ExposeSecret;
        let (pw_str, yubikey) = {
            let db_lock = state
                .database
                .lock()
                .map_err(|_| "database state poisoned".to_string())?;
            let db = db_lock
                .as_ref()
                .ok_or_else(|| "No database loaded".to_string())?;
            (db.password.expose_secret().to_string(), db.yubikey.clone())
        };
        let fresh = Database::open_from_bytes(&kdbx_bytes, pw_str, yubikey)
            .map_err(|e| e.to_string())?;
        let mut db_lock = state
            .database
            .lock()
            .map_err(|_| "database state poisoned".to_string())?;
        *db_lock = Some(fresh);
    } else {
        let db_path = current_db_path(&state)?;
        fs::write(&db_path, &kdbx_bytes)
            .map_err(|e| format!("Failed to write pulled kdbx: {}", e))?;
    }

    if let Ok(mut cloud) = state.cloud.lock() {
        if let Some(s) = cloud.as_mut() {
            if let Some(av) = s.active_vault.as_mut() {
                av.last_known_etag = Some(full.etag);
            }
        }
    }
    Ok(())
}

/// Create a brand-new empty vault on the server. The KDBX is minted in
/// memory with the user's chosen vault password — no local file ever
/// exists for the new vault. After upload the new vault becomes active.
#[tauri::command]
pub async fn cloud_create_vault(
    state: State<'_, AppState>,
    name: String,
    kdbx_password: String,
) -> Result<CloudCreateVaultResp, String> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err("Vault name required".into());
    }
    if kdbx_password.is_empty() {
        return Err("KDBX password required".into());
    }

    // Mint a fresh empty KDBX in RAM, seal it for the cloud.
    let new_db = Database::create_in_memory(&trimmed, kdbx_password.clone())
        .map_err(|e| e.to_string())?;
    let kdbx_bytes = new_db.save_to_bytes().map_err(|e| e.to_string())?;

    let (server_url, token, master_bytes) = snapshot_token_and_master(&state)?;
    let master_key = SecretKey::from_bytes(master_bytes);

    let vault_key = generate_master_key();
    let wrapped_vk = wrap_key(&master_key, &vault_key).map_err(map_crypto_err)?;
    let sealed = seal_vault(&vault_key, &kdbx_bytes).map_err(map_crypto_err)?;

    let client = CloudClient::new(server_url);
    let resp = client
        .create_vault(
            &token,
            &trimmed,
            &b64_encode(&sealed),
            &b64_encode(&wrapped_vk),
        )
        .await
        .map_err(map_cloud_err)?;

    // Make the new vault the active one — swap the in-memory DB so the
    // app's group/entry views land in the new (empty) vault immediately.
    {
        let mut db_lock = state
            .database
            .lock()
            .map_err(|_| "database state poisoned".to_string())?;
        *db_lock = Some(new_db);
    }
    if let Ok(mut cloud) = state.cloud.lock() {
        if let Some(s) = cloud.as_mut() {
            s.active_vault = Some(ActiveVault {
                id: resp.id.clone(),
                name: trimmed,
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

#[derive(Debug, Serialize)]
pub struct CloudUserLookupResp {
    pub user_id: String,
    pub account_pubkey_b64: String,
}

#[tauri::command]
pub async fn cloud_lookup_user(
    state: State<'_, AppState>,
    email: String,
) -> Result<CloudUserLookupResp, String> {
    let (server_url, token, _) = snapshot_token_and_master(&state)?;
    let client = CloudClient::new(server_url);
    let resp = client
        .user_lookup(&token, &email)
        .await
        .map_err(map_cloud_err)?;
    Ok(CloudUserLookupResp {
        user_id: resp.user_id,
        account_pubkey_b64: resp.account_pubkey_b64,
    })
}

/// Share a vault with another account. The caller must have the vault
/// open (active) or at least have unwrapped its vault_key — we re-fetch
/// it from the server and unwrap in-place. The wrapped_vault_key for the
/// recipient is a sealed_box keyed by their account_pubkey.
#[tauri::command]
pub async fn cloud_share_vault(
    state: State<'_, AppState>,
    vault_id: String,
    recipient_email: String,
    role: String, // "editor" | "reader"
) -> Result<(), String> {
    let (server_url, token, master_bytes) = snapshot_token_and_master(&state)?;
    let client = CloudClient::new(server_url);

    // Look up recipient's pubkey first — fail fast if they don't exist
    // or haven't enrolled for sharing.
    let target = client
        .user_lookup(&token, &recipient_email)
        .await
        .map_err(map_cloud_err)?;
    if target.account_pubkey_b64.is_empty() {
        return Err("Recipient has no sharing keypair — ask them to re-sign-in.".to_string());
    }
    let pubkey_bytes = b64_decode(&target.account_pubkey_b64).map_err(map_crypto_err)?;
    if pubkey_bytes.len() != 32 {
        return Err("Recipient pubkey is malformed".to_string());
    }
    let mut recipient_pk = [0u8; 32];
    recipient_pk.copy_from_slice(&pubkey_bytes);

    // Pull the vault to learn the wrapping for the *caller*. We have to
    // unwrap to plaintext before re-sealing for the recipient.
    let full = client
        .get_vault(&token, &vault_id)
        .await
        .map_err(map_cloud_err)?;
    if full.role != "owner" {
        return Err("Only the owner can share a vault.".to_string());
    }
    let master_key = SecretKey::from_bytes(master_bytes);
    let wrapped_vk = b64_decode(&full.wrapped_vault_key_b64).map_err(map_crypto_err)?;
    let vault_key = unwrap_key(&master_key, &wrapped_vk).map_err(map_crypto_err)?;

    let sealed = sealed_seal(&recipient_pk, vault_key.as_bytes()).map_err(map_crypto_err)?;

    client
        .share_vault(
            &token,
            &vault_id,
            &target.user_id,
            &b64_encode(&sealed),
            role.as_str(),
        )
        .await
        .map_err(map_cloud_err)?;
    Ok(())
}

#[tauri::command]
pub async fn cloud_unshare_vault(
    state: State<'_, AppState>,
    vault_id: String,
    user_id: String,
) -> Result<(), String> {
    let (server_url, token, _) = snapshot_token_and_master(&state)?;
    let client = CloudClient::new(server_url);
    client
        .unshare_vault(&token, &vault_id, &user_id)
        .await
        .map_err(map_cloud_err)?;
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
