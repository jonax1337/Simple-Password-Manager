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

/// The cloud session is keyed per server URL — a user with multiple servers
/// (e.g. local dev + hosted) gets one persisted session each. If no URL is
/// known yet (rehydrate before login), we look up the well-known "default"
/// target.
#[cfg_attr(not(windows), allow(dead_code))]
fn target_for_server(server_url: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(server_url.as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
    format!("{}server:{}", TARGET_PREFIX, &hex[..32])
}

/// Well-known target used by `cloud_rehydrate_session` when no other hint
/// is available. The latest successful Remember-Me writes to BOTH this
/// well-known slot and the per-server slot — so a startup rehydrate hits
/// the most recent account without us having to round-trip the server URL
/// through localStorage first.
#[cfg_attr(not(windows), allow(dead_code))]
const DEFAULT_TARGET: &str = "digital.laux.simple_password_manager:cloud:default";

#[derive(Serialize, Deserialize)]
struct PersistedSession {
    server_url: String,
    email: String,
    user_id: String,
    token: String,
    /// Account-level secret. With multi-vault, every per-vault key is
    /// derived/unwrapped from this — so the persisted session can spin
    /// back up any vault the user re-opens.
    master_key_b64: String,
    /// Last-opened vault id, so the unlock screen can highlight it in the
    /// picker after rehydrate. None on first save.
    #[serde(default)]
    last_vault_id: Option<String>,
}

// ----------------------------- Windows impl -----------------------------

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::ERROR_NOT_FOUND;
    use windows::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_FLAGS,
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    };

    pub fn store(target: &str, json: &str) -> Result<(), String> {
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

    pub fn read(target: &str) -> Result<Option<String>, String> {
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

    pub fn clear(target: &str) -> Result<(), String> {
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
    pub fn store(_target: &str, _json: &str) -> Result<(), String> {
        // Linux/macOS need libsecret/Keychain bindings — out of scope for
        // this iteration. The frontend feature flag means non-Windows users
        // just don't see the "Remember me" toggle.
        Err("persistent cloud session is Windows-only for now".into())
    }
    pub fn read(_target: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
    pub fn clear(_target: &str) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Serialize)]
pub struct CloudRehydrateResp {
    pub linked: bool,
    pub server_url: Option<String>,
    pub email: Option<String>,
    pub last_vault_id: Option<String>,
}

/// Snapshot the current cloud session into the OS credential store. Writes
/// both the well-known DEFAULT_TARGET (for zero-config rehydrate at startup)
/// and a per-server slot (so a user with multiple servers doesn't lose
/// older sessions). No-op if no session is linked.
#[tauri::command]
pub fn cloud_persist_session(state: State<'_, AppState>) -> Result<bool, String> {
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
        master_key_b64: B64.encode(s.master_key.as_bytes()),
        last_vault_id: s.active_vault.as_ref().map(|v| v.id.clone()),
    };
    let json = serde_json::to_string(&payload)
        .map_err(|e| format!("serialize session: {}", e))?;
    let per_server = target_for_server(&s.server_url);
    imp::store(&per_server, &json)?;
    imp::store(DEFAULT_TARGET, &json)?;
    Ok(true)
}

/// Try to repopulate the in-memory cloud session from the credential store.
/// Called at app startup. Returns metadata so the UI can decide whether to
/// auto-jump to the vault picker.
#[tauri::command]
pub fn cloud_rehydrate_session(
    state: State<'_, AppState>,
) -> Result<CloudRehydrateResp, String> {
    let Some(json) = imp::read(DEFAULT_TARGET)? else {
        return Ok(CloudRehydrateResp {
            linked: false,
            server_url: None,
            email: None,
            last_vault_id: None,
        });
    };
    let payload: PersistedSession = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(_) => {
            // Corrupt entry — wipe it so we don't loop on bad data.
            let _ = imp::clear(DEFAULT_TARGET);
            return Ok(CloudRehydrateResp {
                linked: false,
                server_url: None,
                email: None,
                last_vault_id: None,
            });
        }
    };
    let master_bytes = B64
        .decode(&payload.master_key_b64)
        .map_err(|_| "stored master_key not base64".to_string())?;
    if master_bytes.len() != 32 {
        let _ = imp::clear(DEFAULT_TARGET);
        return Ok(CloudRehydrateResp {
            linked: false,
            server_url: None,
            email: None,
            last_vault_id: None,
        });
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&master_bytes);

    let resp = CloudRehydrateResp {
        linked: true,
        server_url: Some(payload.server_url.clone()),
        email: Some(payload.email.clone()),
        last_vault_id: payload.last_vault_id.clone(),
    };
    *state
        .cloud
        .lock()
        .map_err(|_| "cloud slot poisoned".to_string())? = Some(CloudSession::new(
        payload.server_url,
        payload.email,
        payload.user_id,
        payload.token,
        SecretKey::from_bytes(arr),
    ));
    Ok(resp)
}

/// Erase the persisted session from both the per-server slot and the default.
#[tauri::command]
pub fn cloud_forget_session(state: State<'_, AppState>) -> Result<(), String> {
    // If we still have a live session, clear its per-server slot too.
    if let Ok(cloud) = state.cloud.lock() {
        if let Some(s) = cloud.as_ref() {
            let per_server = target_for_server(&s.server_url);
            let _ = imp::clear(&per_server);
        }
    }
    imp::clear(DEFAULT_TARGET)?;
    Ok(())
}
