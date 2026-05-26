//! Native Messaging Host for the browser extension.
//!
//! Browsers (Chrome, Edge, Firefox) launch this small binary when the
//! extension calls `chrome.runtime.connectNative(...)`. The host's only job
//! is to hand the extension the current HTTP bridge port + token (and PID),
//! both of which the main Tauri app writes to a per-user `bridge.json` on
//! startup.
//!
//! Protocol (native messaging):
//! - stdin/stdout, each frame prefixed with a 4-byte little-endian length
//! - JSON body
//!
//! Requests this host accepts:
//!   { "action": "discover" }     → returns { port, token, pid, status: "ok" }
//!                                  or { status: "no-app" } if bridge.json
//!                                  is missing or stale.
//!   { "action": "ping" }         → returns { status: "ok" }
//!
//! No other actions are recognized — the host deliberately does not proxy
//! REST calls. Once the extension has the port+token it talks to the bridge
//! HTTP server directly over HTTPS-on-localhost.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, Read, Write};
use std::path::PathBuf;

const BRIDGE_FILE_NAME: &str = "bridge.json";

#[derive(Deserialize)]
struct Request {
    #[serde(default)]
    action: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    status: &'static str,
    reason: String,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
enum DiscoverResponse {
    Ok {
        port: u16,
        token: String,
        pid: u32,
    },
    NoApp,
}

#[derive(Deserialize)]
struct BridgeFile {
    port: u16,
    token: String,
    pid: u32,
}

fn main() -> io::Result<()> {
    // Read native messaging frames until EOF.
    loop {
        let mut len_buf = [0u8; 4];
        if let Err(e) = io::stdin().read_exact(&mut len_buf) {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                return Ok(());
            }
            return Err(e);
        }
        let len = u32::from_le_bytes(len_buf) as usize;
        if len == 0 || len > 1024 * 1024 {
            return write_error("invalid message length");
        }

        let mut body = vec![0u8; len];
        io::stdin().read_exact(&mut body)?;

        let parsed: Result<Request, _> = serde_json::from_slice(&body);
        let response_json = match parsed {
            Ok(req) => handle(&req.action),
            Err(_) => serde_json::to_value(ErrorResponse {
                status: "error",
                reason: "invalid JSON".to_string(),
            })
            .unwrap_or(Value::Null),
        };

        write_frame(&response_json)?;
    }
}

fn handle(action: &str) -> Value {
    match action {
        "ping" => serde_json::json!({ "status": "ok" }),
        "discover" => match read_bridge_file() {
            Some(b) if process_alive(b.pid) => {
                serde_json::to_value(DiscoverResponse::Ok {
                    port: b.port,
                    token: b.token,
                    pid: b.pid,
                })
                .unwrap_or(Value::Null)
            }
            _ => serde_json::to_value(DiscoverResponse::NoApp).unwrap_or(Value::Null),
        },
        other => serde_json::to_value(ErrorResponse {
            status: "error",
            reason: format!("unknown action: {}", other),
        })
        .unwrap_or(Value::Null),
    }
}

fn write_frame(value: &Value) -> io::Result<()> {
    let bytes = serde_json::to_vec(value)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let len = bytes.len() as u32;
    let mut stdout = io::stdout().lock();
    stdout.write_all(&len.to_le_bytes())?;
    stdout.write_all(&bytes)?;
    stdout.flush()?;
    Ok(())
}

fn write_error(reason: &str) -> io::Result<()> {
    let v = serde_json::to_value(ErrorResponse {
        status: "error",
        reason: reason.to_string(),
    })
    .unwrap_or(Value::Null);
    write_frame(&v)
}

fn bridge_state_dir() -> Option<PathBuf> {
    let base = local_data_dir()?;
    Some(base.join("digital.laux.passwordmanager").join("bridge"))
}

#[cfg(target_os = "windows")]
fn local_data_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
}

#[cfg(target_os = "macos")]
fn local_data_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library").join("Application Support"))
}

#[cfg(target_os = "linux")]
fn local_data_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        return Some(PathBuf::from(xdg));
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".local").join("share"))
}

fn read_bridge_file() -> Option<BridgeFile> {
    let path = bridge_state_dir()?.join(BRIDGE_FILE_NAME);
    let raw = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

// Best-effort check that the PID recorded in bridge.json is still a live
// process. If the app crashed, the file may linger; we don't want to hand
// the extension a stale token.
#[cfg(target_os = "windows")]
fn process_alive(pid: u32) -> bool {
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
        fn GetExitCodeProcess(handle: *mut std::ffi::c_void, code: *mut u32) -> i32;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    }
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut code: u32 = 0;
        let ok = GetExitCodeProcess(handle, &mut code) != 0;
        CloseHandle(handle);
        ok && code == STILL_ACTIVE
    }
}

#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    // kill(pid, 0) returns 0 if the process exists and we have permission to
    // signal it, otherwise it errors. We don't care which error — both
    // ESRCH and EPERM mean "don't trust this token".
    unsafe { libc_kill(pid as i32, 0) == 0 }
}

#[cfg(unix)]
extern "C" {
    #[link_name = "kill"]
    fn libc_kill(pid: i32, sig: i32) -> i32;
}
