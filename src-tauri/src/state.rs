use crate::cloud::CloudSession;
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
    /// Active `notify` watcher for the currently-open DB file. We hold it
    /// only to control its lifetime — dropping it stops the OS watch.
    pub file_watcher: Mutex<Option<notify::RecommendedWatcher>>,
    /// Active cloud-sync session, if the user linked an account this run.
    /// Cleared on `close_database` so a new vault doesn't inherit the
    /// previous vault's credentials.
    pub cloud: Mutex<Option<CloudSession>>,
}
