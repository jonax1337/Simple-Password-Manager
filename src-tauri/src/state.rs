use crate::kdbx::Database;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

// `database` lives behind an Arc so the HTTP bridge for the browser
// extension can hold its own handle without duplicating the Mutex.
pub type DatabaseHandle = Arc<Mutex<Option<Database>>>;

pub struct AppState {
    pub database: DatabaseHandle,
    pub initial_file_path: Mutex<Option<String>>,
    pub dismissed_breaches: Mutex<HashMap<String, HashSet<String>>>,
}
