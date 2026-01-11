use crate::kdbx::{DashboardStats, EntryData};
use crate::state::AppState;
use crate::mutex_utils::safe_lock;
use tauri::State;

#[tauri::command]
pub fn search_entries(state: State<AppState>, query: String) -> Result<Vec<EntryData>, String> {
    let database_lock = safe_lock(&state.database, "search_entries")?;

    if let Some(db) = database_lock.as_ref() {
        Ok(db.search_entries(&query))
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn search_entries_in_group(state: State<AppState>, query: String, group_uuid: String) -> Result<Vec<EntryData>, String> {
    let database_lock = safe_lock(&state.database, "search_entries_in_group")?;

    if let Some(db) = database_lock.as_ref() {
        Ok(db.search_entries_in_group(&query, &group_uuid))
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn get_dashboard_stats(state: State<AppState>) -> Result<DashboardStats, String> {
    let database_lock = safe_lock(&state.database, "get_dashboard_stats")?;
    
    if let Some(db) = database_lock.as_ref() {
        Ok(db.get_dashboard_stats())
    } else {
        Err("No database loaded".to_string())
    }
}
