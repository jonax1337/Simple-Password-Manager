use crate::kdbx::{Database, GroupData, KdfInfo};
use crate::mutex_utils::safe_lock;
use crate::state::AppState;
use std::io::Read;
use std::path::PathBuf;
use tauri::State;
use std::process::Command;
use std::fs::File;

#[tauri::command]
pub fn get_initial_file_path(state: State<AppState>) -> Option<String> {
    let initial_path = safe_lock(&state.initial_file_path, "get_initial_file_path").ok()?;
    initial_path.clone()
}

#[tauri::command]
pub fn clear_initial_file_path(state: State<AppState>) -> Result<(), String> {
    let mut initial_path = safe_lock(&state.initial_file_path, "clear_initial_file_path")?;
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

    let mut database_lock = safe_lock(&state.database, "create_database")?;
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

    let mut database_lock = safe_lock(&state.database, "open_database")?;
    *database_lock = Some(db);

    Ok((root_group, path))
}

#[tauri::command]
pub fn save_database(state: State<AppState>) -> Result<(), String> {
    let mut database_lock = safe_lock(&state.database, "save_database")?;

    if let Some(db) = database_lock.as_mut() {
        db.save().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn close_database(state: State<AppState>) -> Result<(), String> {
    let mut database_lock = safe_lock(&state.database, "close_database")?;
    *database_lock = None;
    Ok(())
}

#[tauri::command]
pub fn get_kdf_info(state: State<AppState>) -> Result<KdfInfo, String> {
    let database_lock = safe_lock(&state.database, "get_kdf_info")?;
    if let Some(db) = database_lock.as_ref() {
        Ok(db.get_kdf_info())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn check_database_changes(state: State<AppState>) -> Result<bool, String> {
    let database_lock = safe_lock(&state.database, "check_database_changes")?;

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
pub fn merge_database(state: State<AppState>) -> Result<(), String> {
    let mut database_lock = safe_lock(&state.database, "merge_database")?;

    if let Some(db) = database_lock.as_mut() {
        db.merge_database().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn upgrade_kdf_parameters(state: State<AppState>) -> Result<(), String> {
    let mut database_lock = safe_lock(&state.database, "upgrade_kdf_parameters")?;
    if let Some(db) = database_lock.as_mut() {
        db.upgrade_kdf_parameters().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn get_groups(state: State<AppState>) -> Result<GroupData, String> {
    let database_lock = safe_lock(&state.database, "get_groups")?;

    if let Some(db) = database_lock.as_ref() {
        Ok(db.get_root_group())
    } else {
        Err("No database loaded".to_string())
    }
}
