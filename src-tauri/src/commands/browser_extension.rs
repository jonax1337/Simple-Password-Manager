//! Tauri commands for the browser-extension setup workflow.
//!
//! The user pastes the extension ID from chrome://extensions; we detect all
//! installed Chromium-based browsers (plus Firefox) and write the native
//! messaging host manifest + registry entry for each one. This is the same
//! logic as `extension/scripts/install-host.mjs` ported to Rust so the user
//! can drive it from inside the app.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct Browser {
    id: &'static str,
    label: &'static str,
    kind: BrowserKind,
    manifest_path: PathBuf,
    /// Windows-only: HKCU\Software\<vendor>\NativeMessagingHosts\<host>.
    /// Unused on macOS / Linux because those platforms locate the host
    /// manifest by file path instead of a registry vendor key.
    #[cfg_attr(not(windows), allow(dead_code))]
    win_vendor: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserKind {
    Chromium,
    Firefox,
}

#[derive(Serialize)]
pub struct BrowserInfo {
    pub id: String,
    pub label: String,
    pub kind: String,
}

#[derive(Serialize)]
pub struct InstallReport {
    pub registered: Vec<String>,
    pub failed: Vec<FailedBrowser>,
}

#[derive(Serialize)]
pub struct FailedBrowser {
    pub label: String,
    pub reason: String,
}

const HOST_NAME: &str = "digital.laux.simple_password_manager";

#[tauri::command]
pub fn detect_browsers() -> Vec<BrowserInfo> {
    detect()
        .into_iter()
        .map(|b| BrowserInfo {
            id: b.id.to_string(),
            label: b.label.to_string(),
            kind: match b.kind {
                BrowserKind::Chromium => "chromium",
                BrowserKind::Firefox => "firefox",
            }
            .to_string(),
        })
        .collect()
}

#[tauri::command]
pub fn install_native_host(extension_id: String) -> Result<InstallReport, String> {
    let host_path = locate_host_binary()
        .ok_or_else(|| "Could not locate browser-bridge-host binary".to_string())?;

    let mut registered = Vec::new();
    let mut failed = Vec::new();

    for browser in detect() {
        match install_for(&browser, &extension_id, &host_path) {
            Ok(()) => registered.push(browser.label.to_string()),
            Err(e) => failed.push(FailedBrowser {
                label: browser.label.to_string(),
                reason: e,
            }),
        }
    }

    if registered.is_empty() && failed.is_empty() {
        return Err("No supported browsers detected on this machine".to_string());
    }

    Ok(InstallReport { registered, failed })
}

#[tauri::command]
pub fn uninstall_native_host() -> Result<Vec<String>, String> {
    let mut removed = Vec::new();
    for browser in detect() {
        if browser.manifest_path.exists() {
            if let Err(e) = fs::remove_file(&browser.manifest_path) {
                eprintln!(
                    "[browser_extension] failed to remove {}: {}",
                    browser.manifest_path.display(),
                    e
                );
                continue;
            }
            removed.push(browser.label.to_string());
        }
        #[cfg(target_os = "windows")]
        if let Some(vendor) = browser.win_vendor {
            let _ = std::process::Command::new("reg")
                .args([
                    "delete",
                    &format!("HKCU\\Software\\{}\\NativeMessagingHosts\\{}", vendor, HOST_NAME),
                    "/f",
                ])
                .output();
        }
    }
    Ok(removed)
}

// ---------- detection ----------

fn detect() -> Vec<Browser> {
    #[cfg(target_os = "windows")]
    return detect_windows();
    #[cfg(target_os = "macos")]
    return detect_macos();
    #[cfg(target_os = "linux")]
    return detect_linux();
}

#[cfg(target_os = "windows")]
fn detect_windows() -> Vec<Browser> {
    let lad = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
        let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
        format!("{}\\AppData\\Local", home)
    });
    let rd = std::env::var("APPDATA").unwrap_or_else(|_| lad.clone());
    let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".into());
    let pf86 = std::env::var("ProgramFiles(x86)")
        .unwrap_or_else(|_| "C:\\Program Files (x86)".into());

    let manifest_dir = PathBuf::from(&lad)
        .join("digital.laux.passwordmanager")
        .join("bridge")
        .join("native-hosts");

    let candidates: Vec<(Browser, Vec<PathBuf>)> = vec![
        (
            Browser {
                id: "chrome",
                label: "Google Chrome",
                kind: BrowserKind::Chromium,
                manifest_path: manifest_dir.join(format!("{}.chrome.json", HOST_NAME)),
                win_vendor: Some("Google\\Chrome"),
            },
            vec![
                PathBuf::from(&pf).join("Google\\Chrome\\Application\\chrome.exe"),
                PathBuf::from(&pf86).join("Google\\Chrome\\Application\\chrome.exe"),
                PathBuf::from(&lad).join("Google\\Chrome\\Application\\chrome.exe"),
            ],
        ),
        (
            Browser {
                id: "edge",
                label: "Microsoft Edge",
                kind: BrowserKind::Chromium,
                manifest_path: manifest_dir.join(format!("{}.edge.json", HOST_NAME)),
                win_vendor: Some("Microsoft\\Edge"),
            },
            vec![
                PathBuf::from(&pf).join("Microsoft\\Edge\\Application\\msedge.exe"),
                PathBuf::from(&pf86).join("Microsoft\\Edge\\Application\\msedge.exe"),
            ],
        ),
        (
            Browser {
                id: "brave",
                label: "Brave",
                kind: BrowserKind::Chromium,
                manifest_path: manifest_dir.join(format!("{}.brave.json", HOST_NAME)),
                win_vendor: Some("BraveSoftware\\Brave-Browser"),
            },
            vec![
                PathBuf::from(&pf).join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"),
                PathBuf::from(&pf86).join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"),
                PathBuf::from(&lad).join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"),
            ],
        ),
        (
            Browser {
                id: "vivaldi",
                label: "Vivaldi",
                kind: BrowserKind::Chromium,
                manifest_path: manifest_dir.join(format!("{}.vivaldi.json", HOST_NAME)),
                win_vendor: Some("Vivaldi"),
            },
            vec![
                PathBuf::from(&lad).join("Vivaldi\\Application\\vivaldi.exe"),
                PathBuf::from(&pf).join("Vivaldi\\Application\\vivaldi.exe"),
            ],
        ),
        (
            Browser {
                id: "opera",
                label: "Opera",
                kind: BrowserKind::Chromium,
                manifest_path: manifest_dir.join(format!("{}.opera.json", HOST_NAME)),
                win_vendor: Some("Opera Software\\Opera Stable"),
            },
            vec![
                PathBuf::from(&lad).join("Programs\\Opera\\launcher.exe"),
                PathBuf::from(&pf).join("Opera\\launcher.exe"),
            ],
        ),
        (
            Browser {
                id: "chromium",
                label: "Chromium",
                kind: BrowserKind::Chromium,
                manifest_path: manifest_dir.join(format!("{}.chromium.json", HOST_NAME)),
                win_vendor: Some("Chromium"),
            },
            vec![PathBuf::from(&lad).join("Chromium\\Application\\chrome.exe")],
        ),
        (
            Browser {
                id: "arc",
                label: "Arc",
                kind: BrowserKind::Chromium,
                manifest_path: manifest_dir.join(format!("{}.arc.json", HOST_NAME)),
                win_vendor: Some("TheBrowserCompany\\Arc"),
            },
            vec![
                PathBuf::from(&lad).join("Arc\\Application\\Arc.exe"),
                PathBuf::from(&rd).join("Arc\\Application\\Arc.exe"),
            ],
        ),
        (
            Browser {
                id: "firefox",
                label: "Firefox",
                kind: BrowserKind::Firefox,
                manifest_path: manifest_dir.join(format!("{}.firefox.json", HOST_NAME)),
                win_vendor: Some("Mozilla"),
            },
            vec![
                PathBuf::from(&pf).join("Mozilla Firefox\\firefox.exe"),
                PathBuf::from(&pf86).join("Mozilla Firefox\\firefox.exe"),
            ],
        ),
    ];

    candidates
        .into_iter()
        .filter(|(_, probes)| probes.iter().any(|p| p.exists()))
        .map(|(b, _)| b)
        .collect()
}

#[cfg(target_os = "macos")]
fn detect_macos() -> Vec<Browser> {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()));
    let support = home.join("Library/Application Support");

    let candidates: Vec<(Browser, Vec<PathBuf>)> = vec![
        (
            Browser {
                id: "chrome",
                label: "Google Chrome",
                kind: BrowserKind::Chromium,
                manifest_path: support
                    .join("Google/Chrome/NativeMessagingHosts")
                    .join(format!("{}.json", HOST_NAME)),
                win_vendor: None,
            },
            vec![PathBuf::from("/Applications/Google Chrome.app")],
        ),
        (
            Browser {
                id: "edge",
                label: "Microsoft Edge",
                kind: BrowserKind::Chromium,
                manifest_path: support
                    .join("Microsoft Edge/NativeMessagingHosts")
                    .join(format!("{}.json", HOST_NAME)),
                win_vendor: None,
            },
            vec![PathBuf::from("/Applications/Microsoft Edge.app")],
        ),
        (
            Browser {
                id: "brave",
                label: "Brave",
                kind: BrowserKind::Chromium,
                manifest_path: support
                    .join("BraveSoftware/Brave-Browser/NativeMessagingHosts")
                    .join(format!("{}.json", HOST_NAME)),
                win_vendor: None,
            },
            vec![PathBuf::from("/Applications/Brave Browser.app")],
        ),
        (
            Browser {
                id: "vivaldi",
                label: "Vivaldi",
                kind: BrowserKind::Chromium,
                manifest_path: support
                    .join("Vivaldi/NativeMessagingHosts")
                    .join(format!("{}.json", HOST_NAME)),
                win_vendor: None,
            },
            vec![PathBuf::from("/Applications/Vivaldi.app")],
        ),
        (
            Browser {
                id: "arc",
                label: "Arc",
                kind: BrowserKind::Chromium,
                manifest_path: support
                    .join("Arc/User Data/NativeMessagingHosts")
                    .join(format!("{}.json", HOST_NAME)),
                win_vendor: None,
            },
            vec![PathBuf::from("/Applications/Arc.app")],
        ),
        (
            Browser {
                id: "firefox",
                label: "Firefox",
                kind: BrowserKind::Firefox,
                manifest_path: support
                    .join("Mozilla/NativeMessagingHosts")
                    .join(format!("{}.json", HOST_NAME)),
                win_vendor: None,
            },
            vec![PathBuf::from("/Applications/Firefox.app")],
        ),
    ];

    candidates
        .into_iter()
        .filter(|(_, probes)| probes.iter().any(|p| p.exists()))
        .map(|(b, _)| b)
        .collect()
}

#[cfg(target_os = "linux")]
fn detect_linux() -> Vec<Browser> {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()));
    let cfg = home.join(".config");

    let candidates: Vec<(Browser, Vec<PathBuf>)> = vec![
        (
            Browser {
                id: "chrome",
                label: "Google Chrome",
                kind: BrowserKind::Chromium,
                manifest_path: cfg
                    .join("google-chrome/NativeMessagingHosts")
                    .join(format!("{}.json", HOST_NAME)),
                win_vendor: None,
            },
            vec![
                PathBuf::from("/usr/bin/google-chrome"),
                PathBuf::from("/usr/bin/google-chrome-stable"),
            ],
        ),
        (
            Browser {
                id: "edge",
                label: "Microsoft Edge",
                kind: BrowserKind::Chromium,
                manifest_path: cfg
                    .join("microsoft-edge/NativeMessagingHosts")
                    .join(format!("{}.json", HOST_NAME)),
                win_vendor: None,
            },
            vec![
                PathBuf::from("/usr/bin/microsoft-edge"),
                PathBuf::from("/usr/bin/microsoft-edge-stable"),
            ],
        ),
        (
            Browser {
                id: "brave",
                label: "Brave",
                kind: BrowserKind::Chromium,
                manifest_path: cfg
                    .join("BraveSoftware/Brave-Browser/NativeMessagingHosts")
                    .join(format!("{}.json", HOST_NAME)),
                win_vendor: None,
            },
            vec![PathBuf::from("/usr/bin/brave-browser"), PathBuf::from("/usr/bin/brave")],
        ),
        (
            Browser {
                id: "vivaldi",
                label: "Vivaldi",
                kind: BrowserKind::Chromium,
                manifest_path: cfg
                    .join("vivaldi/NativeMessagingHosts")
                    .join(format!("{}.json", HOST_NAME)),
                win_vendor: None,
            },
            vec![PathBuf::from("/usr/bin/vivaldi"), PathBuf::from("/usr/bin/vivaldi-stable")],
        ),
        (
            Browser {
                id: "chromium",
                label: "Chromium",
                kind: BrowserKind::Chromium,
                manifest_path: cfg
                    .join("chromium/NativeMessagingHosts")
                    .join(format!("{}.json", HOST_NAME)),
                win_vendor: None,
            },
            vec![
                PathBuf::from("/usr/bin/chromium"),
                PathBuf::from("/usr/bin/chromium-browser"),
            ],
        ),
        (
            Browser {
                id: "firefox",
                label: "Firefox",
                kind: BrowserKind::Firefox,
                manifest_path: home
                    .join(".mozilla/native-messaging-hosts")
                    .join(format!("{}.json", HOST_NAME)),
                win_vendor: None,
            },
            vec![PathBuf::from("/usr/bin/firefox"), PathBuf::from("/usr/bin/firefox-esr")],
        ),
    ];

    candidates
        .into_iter()
        .filter(|(_, probes)| probes.iter().any(|p| p.exists()))
        .map(|(b, _)| b)
        .collect()
}

// ---------- install ----------

fn install_for(browser: &Browser, extension_id: &str, host_path: &Path) -> Result<(), String> {
    if extension_id.is_empty() {
        return Err("extension ID is empty".into());
    }

    let allowed_field = match browser.kind {
        BrowserKind::Firefox => format!(
            r#""allowed_extensions": ["{}@simple-password-manager"]"#,
            escape_json(extension_id)
        ),
        BrowserKind::Chromium => format!(
            r#""allowed_origins": ["chrome-extension://{}/"]"#,
            escape_json(extension_id)
        ),
    };

    let manifest = format!(
        r#"{{
  "name": "{name}",
  "description": "Simple Password Manager native messaging host",
  "path": {path_json},
  "type": "stdio",
  {allowed}
}}
"#,
        name = HOST_NAME,
        path_json = serde_json::to_string(host_path).map_err(|e| e.to_string())?,
        allowed = allowed_field,
    );

    if let Some(parent) = browser.manifest_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create dir: {}", e))?;
    }
    fs::write(&browser.manifest_path, manifest)
        .map_err(|e| format!("write manifest: {}", e))?;

    #[cfg(target_os = "windows")]
    if let Some(vendor) = browser.win_vendor {
        let key = format!(
            "HKCU\\Software\\{}\\NativeMessagingHosts\\{}",
            vendor, HOST_NAME
        );
        let path_str = browser.manifest_path.to_string_lossy().to_string();
        let status = std::process::Command::new("reg")
            .args(["add", &key, "/ve", "/t", "REG_SZ", "/d", &path_str, "/f"])
            .status()
            .map_err(|e| format!("invoke reg: {}", e))?;
        if !status.success() {
            return Err(format!("reg add returned {}", status));
        }
    }

    Ok(())
}

// ---------- helpers ----------

fn locate_host_binary() -> Option<PathBuf> {
    let exe_name = if cfg!(windows) {
        "browser-bridge-host.exe"
    } else {
        "browser-bridge-host"
    };

    // 1. Same directory as the main executable (packaged install case).
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            let p = dir.join(exe_name);
            if p.exists() {
                return Some(p);
            }
        }
    }

    // 2. cargo target dirs (dev case).
    let cwd = std::env::current_dir().ok()?;
    for profile in &["release", "debug"] {
        // when launched from repo root
        let p = cwd.join("src-tauri").join("target").join(profile).join(exe_name);
        if p.exists() {
            return Some(p);
        }
        // when launched from src-tauri
        let p = cwd.join("target").join(profile).join(exe_name);
        if p.exists() {
            return Some(p);
        }
    }

    None
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
