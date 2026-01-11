use crate::state::AppState;
use crate::mutex_utils::safe_lock;
use tauri::State;

#[tauri::command]
pub fn create_group(
    state: State<AppState>,
    name: String,
    parent_uuid: Option<String>,
    icon_id: Option<u32>,
) -> Result<(), String> {
    let mut database_lock = safe_lock(&state.database, "create_group")?;

    if let Some(db) = database_lock.as_mut() {
        db.create_group(name, parent_uuid, icon_id)
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn rename_group(
    state: State<AppState>,
    group_uuid: String,
    new_name: String,
    icon_id: Option<u32>,
) -> Result<(), String> {
    let mut database_lock = safe_lock(&state.database, "rename_group")?;

    if let Some(db) = database_lock.as_mut() {
        db.rename_group(&group_uuid, new_name, icon_id)
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn move_group(
    state: State<AppState>,
    group_uuid: String,
    new_parent_uuid: String,
) -> Result<(), String> {
    let mut database_lock = safe_lock(&state.database, "move_group")?;

    if let Some(db) = database_lock.as_mut() {
        db.move_group(&group_uuid, &new_parent_uuid)
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn reorder_group(
    state: State<AppState>,
    group_uuid: String,
    target_index: usize,
) -> Result<(), String> {
    let mut database_lock = safe_lock(&state.database, "reorder_group")?;

    if let Some(db) = database_lock.as_mut() {
        db.reorder_group(&group_uuid, target_index)
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn delete_group(state: State<AppState>, group_uuid: String) -> Result<(), String> {
    let mut database_lock = safe_lock(&state.database, "delete_group")?;

    if let Some(db) = database_lock.as_mut() {
        db.delete_group(&group_uuid).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No database loaded".to_string())
    }
}
