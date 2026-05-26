use simple_password_manager::kdbx::{DashboardStats, EntryData};
use simple_password_manager::state::AppState;
use tauri::State;

#[tauri::command]
pub fn search_entries(state: State<AppState>, query: String) -> Result<Vec<EntryData>, String> {
    let database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("search_entries: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;

    if let Some(db) = database_lock.as_ref() {
        Ok(db.search_entries(&query))
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn search_entries_in_group(state: State<AppState>, query: String, group_uuid: String) -> Result<Vec<EntryData>, String> {
    let database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("search_entries_in_group: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;

    if let Some(db) = database_lock.as_ref() {
        Ok(db.search_entries_in_group(&query, &group_uuid))
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn get_dashboard_stats(state: State<AppState>) -> Result<DashboardStats, String> {
    let database_lock = state.database.lock()
        .map_err(|e| {
            eprintln!("get_dashboard_stats: Lock poisoned: {}", e);
            "Failed to access database state".to_string()
        })?;
    
    if let Some(db) = database_lock.as_ref() {
        Ok(db.get_dashboard_stats())
    } else {
        Err("No database loaded".to_string())
    }
}
