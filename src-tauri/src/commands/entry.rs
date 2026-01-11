use crate::kdbx::EntryData;
use crate::state::AppState;
use crate::mutex_utils::safe_lock;
use tauri::State;

#[tauri::command]
pub fn get_entries(state: State<AppState>, group_uuid: String) -> Result<Vec<EntryData>, String> {
    let database_lock = safe_lock(&state.database, "get_entries")?;
    let db = database_lock
        .as_ref()
        .ok_or("Database not loaded".to_string())?;

    db.get_entries_in_group(&group_uuid).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_favorite_entries(state: State<AppState>) -> Result<Vec<EntryData>, String> {
    let database_lock = safe_lock(&state.database, "get_favorite_entries")?;
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
    let database_lock = safe_lock(&state.database, "get_entry")?;

    if let Some(db) = database_lock.as_ref() {
        db.get_entry(&entry_uuid).map_err(|e| e.to_string())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn create_entry(state: State<AppState>, entry: EntryData) -> Result<(), String> {
    let mut database_lock = safe_lock(&state.database, "create_entry")?;

    if let Some(db) = database_lock.as_mut() {
        db.create_entry(entry).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn update_entry(state: State<AppState>, entry: EntryData) -> Result<(), String> {
    let mut database_lock = safe_lock(&state.database, "update_entry")?;

    if let Some(db) = database_lock.as_mut() {
        db.update_entry(entry).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn delete_entry(state: State<AppState>, entry_uuid: String) -> Result<(), String> {
    let mut database_lock = safe_lock(&state.database, "delete_entry")?;

    if let Some(db) = database_lock.as_mut() {
        db.delete_entry(&entry_uuid).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn move_entry(state: State<AppState>, entry_uuid: String, new_group_uuid: String) -> Result<(), String> {
    let mut database_lock = safe_lock(&state.database, "move_entry")?;

    if let Some(db) = database_lock.as_mut() {
        db.move_entry(&entry_uuid, &new_group_uuid).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}
