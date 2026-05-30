//! Persist the active cloud-sync session in the OS credential store so the
//! user doesn't re-type their master password every time they unlock.
//!
//! The blob holds: server URL, username, user_id, JWT, base64(vault_key),
//! last_known_etag. All of this is per-`db_path` so a user with multiple
//! vaults gets independent sessions.
//!
//! Threat model: anything that can read the current Windows user's
//! credential store can hijack the session — DPAPI binds the blob to the
//! account but doesn't gate reads behind biometrics. The user opts in
//! explicitly via the "Remember me" toggle; we don't auto-persist.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use simple_password_manager::cloud::{crypto::SecretKey, CloudSession};
use simple_password_manager::state::AppState;
use tauri::State;

#[cfg_attr(not(windows), allow(dead_code))]
const TARGET_PREFIX: &str = "digital.laux.simple_password_manager:cloud:";

#[cfg_attr(not(windows), allow(dead_code))]
fn target_for(db_path: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(db_path.as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
    format!("{}{}", TARGET_PREFIX, &hex[..32])
}

#[derive(Serialize, Deserialize)]
struct PersistedSession {
    server_url: String,
    email: String,
    user_id: String,
    token: String,
    vault_key_b64: String,
    last_known_etag: Option<String>,
}

// ----------------------------- Windows impl -----------------------------

#[cfg(windows)]
mod imp {
    use super::target_for;
    use windows::Win32::Foundation::ERROR_NOT_FOUND;
    use windows::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_FLAGS,
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    };

    pub fn store(db_path: &str, json: &str) -> Result<(), String> {
        let target = target_for(db_path);
        let target_w: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
        let mut secret_bytes: Vec<u8> = json.as_bytes().to_vec();

        let cred = CREDENTIALW {
            Flags: CRED_FLAGS(0),
            Type: CRED_TYPE_GENERIC,
            TargetName: windows::core::PWSTR(target_w.as_ptr() as *mut _),
            Comment: windows::core::PWSTR::null(),
            LastWritten: windows::Win32::Foundation::FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
            CredentialBlobSize: secret_bytes.len() as u32,
            CredentialBlob: secret_bytes.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: windows::core::PWSTR::null(),
            UserName: windows::core::PWSTR::null(),
        };
        unsafe {
            CredWriteW(&cred, 0).map_err(|e| format!("CredWriteW failed: {}", e))?;
        }
        Ok(())
    }

    pub fn read(db_path: &str) -> Result<Option<String>, String> {
        let target = target_for(db_path);
        let target_w: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let mut cred_ptr: *mut CREDENTIALW = std::ptr::null_mut();
            match CredReadW(
                windows::core::PCWSTR(target_w.as_ptr()),
                CRED_TYPE_GENERIC,
                None,
                &mut cred_ptr,
            ) {
                Ok(()) => {
                    let cred = &*cred_ptr;
                    let len = cred.CredentialBlobSize as usize;
                    let blob = std::slice::from_raw_parts(cred.CredentialBlob, len).to_vec();
                    CredFree(cred_ptr as *mut _);
                    let s = String::from_utf8(blob).map_err(|_| "blob not UTF-8".to_string())?;
                    Ok(Some(s))
                }
                Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(None),
                Err(e) => Err(format!("CredReadW failed: {}", e)),
            }
        }
    }

    pub fn clear(db_path: &str) -> Result<(), String> {
        let target = target_for(db_path);
        let target_w: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            match CredDeleteW(
                windows::core::PCWSTR(target_w.as_ptr()),
                CRED_TYPE_GENERIC,
                None,
            ) {
                Ok(()) => Ok(()),
                Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
                Err(e) => Err(format!("CredDeleteW failed: {}", e)),
            }
        }
    }
}

#[cfg(not(windows))]
#[allow(dead_code)]
mod imp {
    pub fn store(_db_path: &str, _json: &str) -> Result<(), String> {
        // Linux/macOS need libsecret/Keychain bindings — out of scope for
        // this iteration. The frontend feature flag means non-Windows users
        // just don't see the "Remember me" toggle.
        Err("persistent cloud session is Windows-only for now".into())
    }
    pub fn read(_db_path: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
    pub fn clear(_db_path: &str) -> Result<(), String> {
        Ok(())
    }
}

fn current_db_path(state: &AppState) -> Result<String, String> {
    let lock = state
        .database
        .lock()
        .map_err(|_| "database state poisoned".to_string())?;
    let path = lock
        .as_ref()
        .ok_or_else(|| "No database loaded".to_string())?
        .path
        .to_string_lossy()
        .to_string();
    Ok(path)
}

/// Snapshot the current cloud session into the OS credential store. No-op
/// if no session is linked.
#[tauri::command]
pub fn cloud_persist_session(state: State<'_, AppState>) -> Result<bool, String> {
    let db_path = current_db_path(&state)?;
    let cloud = state
        .cloud
        .lock()
        .map_err(|_| "cloud slot poisoned".to_string())?;
    let Some(s) = cloud.as_ref() else {
        return Ok(false);
    };
    let payload = PersistedSession {
        server_url: s.server_url.clone(),
        email: s.email.clone(),
        user_id: s.user_id.clone(),
        token: s.token.clone(),
        vault_key_b64: B64.encode(s.vault_key.as_bytes()),
        last_known_etag: s.last_known_etag.clone(),
    };
    let json = serde_json::to_string(&payload)
        .map_err(|e| format!("serialize session: {}", e))?;
    imp::store(&db_path, &json)?;
    Ok(true)
}

/// Try to repopulate the in-memory cloud session from the credential store.
/// Returns true on hit. The desktop app calls this right after a successful
/// open_database so cloud-linked vaults light up the Cloud Sync UI without
/// the user re-entering their password.
#[tauri::command]
pub fn cloud_rehydrate_session(state: State<'_, AppState>) -> Result<bool, String> {
    let db_path = current_db_path(&state)?;
    let Some(json) = imp::read(&db_path)? else {
        return Ok(false);
    };
    let payload: PersistedSession = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(_) => {
            // Corrupt entry — wipe it so we don't loop on bad data.
            let _ = imp::clear(&db_path);
            return Ok(false);
        }
    };
    let vault_key_bytes = B64
        .decode(&payload.vault_key_b64)
        .map_err(|_| "stored vault_key not base64".to_string())?;
    if vault_key_bytes.len() != 32 {
        let _ = imp::clear(&db_path);
        return Ok(false);
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&vault_key_bytes);

    *state
        .cloud
        .lock()
        .map_err(|_| "cloud slot poisoned".to_string())? = Some(CloudSession::new(
        payload.server_url,
        payload.email,
        payload.user_id,
        payload.token,
        SecretKey::from_bytes(arr),
        payload.last_known_etag,
    ));
    Ok(true)
}

/// Erase the persisted session. Called by cloud_disconnect and also as a
/// recovery hook when an API call returns 401 (stale token).
#[tauri::command]
pub fn cloud_forget_session(state: State<'_, AppState>) -> Result<(), String> {
    let db_path = current_db_path(&state)?;
    imp::clear(&db_path)?;
    Ok(())
}
