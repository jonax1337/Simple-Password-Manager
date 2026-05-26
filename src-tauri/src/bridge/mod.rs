//! Local-loopback HTTP bridge that lets the browser extension query the
//! currently unlocked KeePass database in this Tauri process.
//!
//! Architecture is documented in `extension/ARCHITECTURE.md`. In short:
//! the server listens on `127.0.0.1:<random>` with bearer-token auth and
//! CORS pinned to `chrome-extension://` and `moz-extension://` origins.
//! Port + token are persisted to a per-user file (`bridge.json`) so the
//! native messaging host can hand them to the extension at startup.

mod auth;
mod handlers;
mod server;

pub use auth::{remove_bridge_file, write_bridge_file};
pub use server::start;

#[allow(unused_imports)]
pub use auth::{read_bridge_file, BridgeFile, BRIDGE_FILE_NAME};
#[allow(unused_imports)]
pub use server::BridgeState;

use std::path::PathBuf;

/// Where the bridge state file lives. Matches what the native messaging
/// host expects, so both binaries must agree on this path.
///
/// On Windows: `%LOCALAPPDATA%\digital.laux.passwordmanager\bridge`
/// On macOS:   `~/Library/Application Support/digital.laux.passwordmanager/bridge`
/// On Linux:   `~/.local/share/digital.laux.passwordmanager/bridge`
pub fn bridge_state_dir() -> Option<PathBuf> {
    let base = dirs_local_data_dir()?;
    Some(base.join("digital.laux.passwordmanager").join("bridge"))
}

#[cfg(target_os = "windows")]
fn dirs_local_data_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
}

#[cfg(target_os = "macos")]
fn dirs_local_data_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library").join("Application Support"))
}

#[cfg(target_os = "linux")]
fn dirs_local_data_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        return Some(PathBuf::from(xdg));
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".local").join("share"))
}
