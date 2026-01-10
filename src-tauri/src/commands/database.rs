use crate::kdbx::{Database, GroupData, KdfInfo};
use crate::state::AppState;
use std::path::{Path, PathBuf};
use tauri::State;
use std::process::Command;

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
    state: State<AppState>,
    path: String,
    password: String,
) -> Result<GroupData, String> {
    let db = Database::create(PathBuf::from(&path), password).map_err(|e| e.to_string())?;

    let root_group = db.get_root_group();

    let mut database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("create_database: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;
    *database_lock = Some(db);

    Ok(root_group)
}

#[tauri::command]
pub fn open_database(
    state: State<AppState>,
    path: String,
    password: String,
) -> Result<(GroupData, String), String> {
    let path_buf = PathBuf::from(&path);
    let db = Database::open(path_buf.clone(), password).map_err(|e| e.to_string())?;

    let root_group = db.get_root_group();

    let mut database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("open_database: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;
    *database_lock = Some(db);

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
        db.save().map_err(|e| e.to_string())?;
        Ok(())
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
    *database_lock = None;
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
    
    // Check file extension
    if let Some(ext) = path_buf.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if ext_str != "kdbx" {
            return Ok(false);
        }
    } else {
        return Ok(false);
    }
    
    // Try to read just the file header to validate it's a KDBX file
    // We don't actually open it (which would require a password),
    // just check if it has the KDBX magic bytes
    use std::io::Read;
    match std::fs::File::open(&path_buf) {
        Ok(mut file) => {
            let mut header = [0u8; 8];
            match file.read_exact(&mut header) {
                Ok(_) => {
                    // KDBX files start with the magic signature: 0x03D9A29A (KeePass 2.x)
                    // Primary signature: 0x03, 0xD9, 0xA2, 0x9A
                    if header[0] == 0x03 && header[1] == 0xD9 && header[2] == 0xA2 && header[3] == 0x9A {
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
                Err(_) => {
                    // File is too short to be a valid KDBX file
                    Ok(false)
                }
            }
        }
        Err(_) => {
            // File exists but can't be opened (permissions issue, etc.)
            Ok(false)
        }
    }
}
