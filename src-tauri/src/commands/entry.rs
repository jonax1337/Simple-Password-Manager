use simple_password_manager::kdbx::EntryData;
use simple_password_manager::state::AppState;
use tauri::State;

#[tauri::command]
pub fn get_entries(state: State<AppState>, group_uuid: String) -> Result<Vec<EntryData>, String> {
    let database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("get_entries: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;
    let db = database_lock
        .as_ref()
        .ok_or("Database not loaded".to_string())?;

    db.get_entries_in_group(&group_uuid).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_favorite_entries(state: State<AppState>) -> Result<Vec<EntryData>, String> {
    let database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("get_favorite_entries: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;
    let db = database_lock
        .as_ref()
        .ok_or("Database not loaded".to_string())?;

    let all_entries = db.get_all_entries();
    let favorites: Vec<EntryData> = all_entries
        .into_iter()
        .filter(|entry| entry.is_favorite)
        .collect();

    Ok(favorites)
}

#[tauri::command]
pub fn get_entry(state: State<AppState>, entry_uuid: String) -> Result<EntryData, String> {
    let database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("get_entry: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;

    if let Some(db) = database_lock.as_ref() {
        db.get_entry(&entry_uuid).map_err(|e| e.to_string())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn create_entry(state: State<AppState>, entry: EntryData) -> Result<(), String> {
    let mut database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("create_entry: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;

    if let Some(db) = database_lock.as_mut() {
        db.create_entry(entry).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn update_entry(state: State<AppState>, entry: EntryData) -> Result<(), String> {
    let mut database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("update_entry: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;

    if let Some(db) = database_lock.as_mut() {
        db.update_entry(entry).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn delete_entry(state: State<AppState>, entry_uuid: String) -> Result<(), String> {
    let mut database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("delete_entry: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;

    if let Some(db) = database_lock.as_mut() {
        db.delete_entry(&entry_uuid).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn move_entry(state: State<AppState>, entry_uuid: String, new_group_uuid: String) -> Result<(), String> {
    let mut database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("move_entry: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;

    if let Some(db) = database_lock.as_mut() {
        db.move_entry(&entry_uuid, &new_group_uuid).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}
