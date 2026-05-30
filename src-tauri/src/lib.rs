//! Library facade for the desktop app.
//!
//! Holds everything testable in isolation from Tauri's runtime: the KDBX
//! layer, TOTP, AppState plumbing, and the browser-extension HTTP bridge.
//! `commands/` deliberately lives in `main.rs` only — those are thin Tauri
//! wrappers and pulling them into the lib would force every integration
//! test binary to link against the WebView runtime.

pub mod bridge;
pub mod cloud;
pub mod kdbx;
pub mod lockfile;
pub mod state;
pub mod totp;
