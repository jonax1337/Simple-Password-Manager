//! Tauri commands for the Yubikey unlock feature.
//!
//! The HMAC-SHA1 challenge-response flow happens inside the keepass crate
//! during `Database::open`/`save`; these commands are the thin Tauri-side
//! wrappers that the frontend calls to enumerate keys and toggle the
//! feature on/off for the currently-open database.

use simple_password_manager::kdbx::{list_available_yubikeys, YubikeyConfig, YubikeyInfo};
use simple_password_manager::state::AppState;
use tauri::State;

/// Enumerate Yubikeys currently plugged in. Returns an empty list if no
/// keys are detected — that's the "no Yubikey support available" case
/// the frontend renders as a disabled toggle.
#[tauri::command]
pub fn list_yubikeys() -> Result<Vec<YubikeyInfo>, String> {
    list_available_yubikeys().map_err(|e| e.to_string())
}

/// Turn the Yubikey requirement on for the database that's currently
/// open in this process. Verifies the device is reachable, then re-saves
/// the file using master password + challenge-response so the on-disk
/// encryption matches the new key set.
#[tauri::command]
pub fn enable_yubikey(
    state: State<AppState>,
    serial_number: u32,
    slot: String,
) -> Result<(), String> {
    let mut guard = state
        .database
        .lock()
        .map_err(|_| "Failed to access database state".to_string())?;
    let db = guard.as_mut().ok_or("No database is open".to_string())?;

    db.enable_yubikey(YubikeyConfig {
        serial_number,
        slot,
    })
    .map_err(|e| e.to_string())
}

/// Drop the Yubikey requirement from the open database and re-save it
/// using only the master password. The frontend should also clear its
/// per-database "Yubikey enabled" flag so the next unlock doesn't try
/// to ask for the key.
#[tauri::command]
pub fn disable_yubikey(state: State<AppState>) -> Result<(), String> {
    let mut guard = state
        .database
        .lock()
        .map_err(|_| "Failed to access database state".to_string())?;
    let db = guard.as_mut().ok_or("No database is open".to_string())?;

    db.disable_yubikey().map_err(|e| e.to_string())
}

/// Report whether the currently-open database is protected by a Yubikey.
/// Used by the popup-style "Database details" screen to show the current
/// configuration without exposing the serial number itself.
#[tauri::command]
pub fn yubikey_enabled_for_open_db(state: State<AppState>) -> Result<bool, String> {
    let guard = state
        .database
        .lock()
        .map_err(|_| "Failed to access database state".to_string())?;
    Ok(guard.as_ref().map(|db| db.yubikey.is_some()).unwrap_or(false))
}
