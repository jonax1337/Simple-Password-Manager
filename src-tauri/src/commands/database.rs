use simple_password_manager::kdbx::{
    ConflictChoice, Database, EntryConflict, GroupData, KdfInfo, YubikeyConfig,
};
use std::collections::HashMap;
use simple_password_manager::lockfile::{self, LockError, LockInfo};
use simple_password_manager::state::AppState;
use notify::{event::ModifyKind, EventKind, RecursiveMode, Watcher};
use std::io::Read;
use std::path::PathBuf;
use tauri::{Emitter, State, AppHandle};
use std::process::Command;
use std::fs::File;
use std::time::{Duration, Instant};
use std::sync::Mutex as StdMutex;

/// Debounce window for raw FS events. Cloud sync clients (OneDrive, Dropbox)
/// often touch the file multiple times during a single "save" — a 250ms
/// quiet window collapses those into one frontend event.
const WATCHER_DEBOUNCE_MS: u64 = 250;

/// Start a `notify` watcher on `path`. Emits a Tauri event `database-external-change`
/// (debounced) to the frontend. Replaces any previously-installed watcher.
fn install_watcher(state: &AppState, app: AppHandle, path: PathBuf) -> Result<(), String> {
    let last_emit: StdMutex<Instant> = StdMutex::new(
        Instant::now() - Duration::from_secs(60),
    );
    let app_for_cb = app.clone();
    let watched_path = path.clone();

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(event) = res else { return };
        // Filter: only react to Modify/Create/Remove. Skip pure metadata noise.
        let interesting = matches!(
            event.kind,
            EventKind::Modify(ModifyKind::Data(_))
                | EventKind::Modify(ModifyKind::Any)
                | EventKind::Modify(ModifyKind::Name(_))
                | EventKind::Create(_)
                | EventKind::Remove(_)
        );
        if !interesting {
            return;
        }
        // Cloud sync clients often rename-into-place, so the watched path
        // sometimes shows up in event.paths and sometimes the parent does.
        // We re-check the file's mtime in the frontend anyway, so we don't
        // need to be picky here — only debounce.
        let mut last = match last_emit.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let now = Instant::now();
        if now.duration_since(*last) < Duration::from_millis(WATCHER_DEBOUNCE_MS) {
            return;
        }
        *last = now;
        drop(last);
        let _ = app_for_cb.emit(
            "database-external-change",
            watched_path.to_string_lossy().to_string(),
        );
    })
    .map_err(|e| format!("Failed to create file watcher: {}", e))?;

    // Watch the parent dir non-recursively. Some editors (and cloud clients)
    // replace the file rather than modifying it in place — watching the file
    // directly would lose events after the inode swap. Watching the parent
    // catches both the in-place and replace cases.
    let parent = path
        .parent()
        .ok_or_else(|| "Database path has no parent directory".to_string())?;
    watcher
        .watch(parent, RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch DB directory: {}", e))?;

    let mut slot = state
        .file_watcher
        .lock()
        .map_err(|_| "watcher slot poisoned".to_string())?;
    *slot = Some(watcher);
    Ok(())
}

fn uninstall_watcher(state: &AppState) {
    if let Ok(mut slot) = state.file_watcher.lock() {
        *slot = None;
    }
}

#[tauri::command]
pub fn get_initial_file_path(state: State<AppState>) -> Option<String> {
    let initial_path = state.initial_file_path.lock()
        .map_err(|e| {
            eprintln!("get_initial_file_path: Lock poisoned: {}", e);
            e
        })
        .ok()?;
    initial_path.clone()
}

#[tauri::command]
pub fn clear_initial_file_path(state: State<AppState>) -> Result<(), String> {
    let mut initial_path = state.initial_file_path.lock()
        .map_err(|e| {
            eprintln!("clear_initial_file_path: Lock poisoned: {}", e);
            "Failed to access state".to_string()
        })?;
    *initial_path = None;
    Ok(())
}

#[tauri::command]
pub fn create_database(
    app: AppHandle,
    state: State<AppState>,
    path: String,
    password: String,
    yubikey: Option<YubikeyConfig>,
) -> Result<GroupData, String> {
    let path_buf = PathBuf::from(&path);
    let db = Database::create_with_yubikey(path_buf.clone(), password, yubikey)
        .map_err(|e| e.to_string())?;

    let root_group = db.get_root_group();

    let mut database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("create_database: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;
    *database_lock = Some(db);
    drop(database_lock);

    // Best-effort watcher install. A missing watcher only degrades the
    // sync UX (frontend falls back to mtime polling), so we log and continue.
    if let Err(e) = install_watcher(&state, app, path_buf) {
        eprintln!("create_database: watcher install failed: {}", e);
    }

    Ok(root_group)
}

#[tauri::command]
pub fn open_database(
    app: AppHandle,
    state: State<AppState>,
    path: String,
    password: String,
    yubikey: Option<YubikeyConfig>,
) -> Result<(GroupData, String), String> {
    let path_buf = PathBuf::from(&path);
    let db = Database::open_with_yubikey(path_buf.clone(), password, yubikey)
        .map_err(|e| e.to_string())?;

    let root_group = db.get_root_group();

    let mut database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("open_database: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;
    *database_lock = Some(db);
    drop(database_lock);

    if let Err(e) = install_watcher(&state, app, path_buf) {
        eprintln!("open_database: watcher install failed: {}", e);
    }

    Ok((root_group, path))
}

#[tauri::command]
pub fn save_database(state: State<AppState>) -> Result<(), String> {
    let mut database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("save_database: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;

    if let Some(db) = database_lock.as_mut() {
        // Acquire a lock around the actual write. Foreign locks fail fast
        // with a peer hint so the frontend can show "Alice is editing".
        // Stale foreign locks (crashed peer) are stolen transparently.
        match lockfile::acquire_lock(&db.path) {
            Ok(_) => {}
            Err(LockError::HeldByPeer(host, pid)) => {
                return Err(format!("LOCK_HELD:{}:{}", host, pid));
            }
            Err(e) => return Err(e.to_string()),
        }
        let result = db.save().map_err(|e| e.to_string());
        let _ = lockfile::release_lock(&db.path);
        result
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn peek_lock_status(state: State<AppState>) -> Result<Option<LockInfo>, String> {
    let database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("peek_lock_status: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;
    if let Some(db) = database_lock.as_ref() {
        Ok(lockfile::peek_foreign_lock(&db.path))
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn close_database(state: State<AppState>) -> Result<(), String> {
    let mut database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("close_database: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;
    if let Some(db) = database_lock.as_ref() {
        let _ = simple_password_manager::lockfile::release_lock(&db.path);
    }
    *database_lock = None;
    drop(database_lock);
    uninstall_watcher(&state);
    // Drop cloud session too — a new vault should re-link explicitly.
    if let Ok(mut cloud) = state.cloud.lock() {
        *cloud = None;
    }
    Ok(())
}

#[tauri::command]
pub fn get_kdf_info(state: State<AppState>) -> Result<KdfInfo, String> {
    let database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("get_kdf_info: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;
    if let Some(db) = database_lock.as_ref() {
        Ok(db.get_kdf_info())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn check_database_changes(state: State<AppState>) -> Result<bool, String> {
    let database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("check_database_changes: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;

    if let Some(db) = database_lock.as_ref() {
        db.check_for_changes().map_err(|e| e.to_string())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn open_database_in_new_instance(db_path: String) -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?;
    
    Command::new(current_exe)
        .arg(&db_path)
        .spawn()
        .map_err(|e| format!("Failed to spawn new instance: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub fn validate_database_file(path: String) -> Result<bool, String> {
    let path_buf = PathBuf::from(&path);
    
    // Check if file exists
    if !path_buf.exists() {
        return Ok(false);
    }
    
    // Check if it's a file (not a directory)
    if !path_buf.is_file() {
        return Ok(false);
    }
    
    // Check .kdbx extension
    if let Some(ext) = path_buf.extension() {
        if ext.to_string_lossy().to_lowercase() != "kdbx" {
            return Ok(false);
        }
    } else {
        return Ok(false);
    }
    
    // Validate KDBX magic bytes (0x03D9A29A)
    // KDBX format starts with these 4 bytes after the base signature
    let mut file = File::open(&path_buf)
        .map_err(|_| "Failed to open file for validation".to_string())?;
    
    let mut magic_bytes = [0u8; 8];
    if file.read_exact(&mut magic_bytes).is_err() {
        // File is too small to be a valid KDBX file
        return Ok(false);
    }
    
    // Check for KDBX signature: first 4 bytes should be 0x03, 0xD9, 0xA2, 0x9A
    // followed by version bytes
    let valid = magic_bytes[0] == 0x03 
        && magic_bytes[1] == 0xD9 
        && magic_bytes[2] == 0xA2 
        && magic_bytes[3] == 0x9A;
    
    Ok(valid)
}

#[tauri::command]
pub fn analyze_conflicts(state: State<AppState>) -> Result<Vec<EntryConflict>, String> {
    let database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("analyze_conflicts: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;
    if let Some(db) = database_lock.as_ref() {
        db.analyze_conflicts().map_err(|e| e.to_string())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn resolve_conflicts(
    state: State<AppState>,
    decisions: HashMap<String, ConflictChoice>,
) -> Result<(), String> {
    let mut database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("resolve_conflicts: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;
    if let Some(db) = database_lock.as_mut() {
        db.resolve_and_merge(decisions).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn merge_database(state: State<AppState>) -> Result<(), String> {
    let mut database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("merge_database: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;

    if let Some(db) = database_lock.as_mut() {
        db.merge_database().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn upgrade_kdf_parameters(state: State<AppState>) -> Result<(), String> {
    let mut database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("upgrade_kdf_parameters: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;
    if let Some(db) = database_lock.as_mut() {
        db.upgrade_kdf_parameters().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn get_groups(state: State<AppState>) -> Result<GroupData, String> {
    let database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("get_groups: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;

    if let Some(db) = database_lock.as_ref() {
        Ok(db.get_root_group())
    } else {
        Err("No database loaded".to_string())
    }
}
