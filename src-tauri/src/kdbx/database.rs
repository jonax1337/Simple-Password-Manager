use argon2::Version as Argon2Version;
use keepass::{
    config::{DatabaseConfig, KdfConfig},
    db::{EntryId, GroupId, Times},
    ChallengeResponseKey, Database as KeepassDatabase, DatabaseKey,
};
use secrecy::{ExposeSecret, SecretString};
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::time::SystemTime;

use super::error::DatabaseError;
use super::types::{ConflictChoice, EntryConflict, KdfInfo, YubikeyConfig, YubikeyInfo};

pub struct Database {
    pub db: KeepassDatabase,
    pub path: PathBuf,
    pub password: SecretString,
    /// If set, the database is encrypted with master password + a Yubikey
    /// HMAC-SHA1 challenge-response. We remember which key + slot the user
    /// chose so we can perform the same challenge on every save.
    pub yubikey: Option<YubikeyConfig>,
    pub last_modified: Option<SystemTime>,
}

/// List Yubikeys currently connected. Exposed via a Tauri command so the
/// frontend can populate a "pick a key" dropdown during Yubikey setup.
pub fn list_available_yubikeys() -> Result<Vec<YubikeyInfo>, DatabaseError> {
    match ChallengeResponseKey::get_available_yubikeys() {
        Ok(keys) => Ok(keys
            .iter()
            .map(|y| YubikeyInfo {
                serial_number: y.serial_number,
                name: y.name.clone(),
            })
            .collect()),
        Err(e) => {
            // The `ChallengeResponseKeyError` enum lives in a private
            // module so we can't match on its variants directly. The
            // crate's Display for NoKeys is stable enough to string-match
            // against — and we want the same "empty list" outcome for any
            // failure that means "no keys reachable", including HID
            // permission errors on Linux.
            let msg = e.to_string();
            if msg.to_lowercase().contains("no challenge") {
                Ok(Vec::new())
            } else {
                Err(DatabaseError::OpenError(format!(
                    "Failed to enumerate Yubikeys: {}",
                    msg
                )))
            }
        }
    }
}

// Build a DatabaseKey from a password + optional Yubikey. The Yubikey
// touch happens inside `keepass`'s code (it calls challenge_response
// when DatabaseKey::perform_challenge is invoked during open/save).
fn build_key(
    password: &SecretString,
    yubikey: Option<&YubikeyConfig>,
) -> Result<DatabaseKey, DatabaseError> {
    let mut key = DatabaseKey::new().with_password(password.expose_secret());

    if let Some(cfg) = yubikey {
        let yk = ChallengeResponseKey::get_yubikey(Some(cfg.serial_number)).map_err(|e| {
            DatabaseError::OpenError(format!(
                "Yubikey with serial {} not found: {}",
                cfg.serial_number, e
            ))
        })?;
        key = key.with_challenge_response_key(ChallengeResponseKey::YubikeyChallenge(
            yk,
            cfg.slot.clone(),
        ));
    }

    Ok(key)
}

impl Database {
    pub fn create(path: PathBuf, password: String) -> Result<Self, DatabaseError> {
        Self::create_with_yubikey(path, password, None)
    }

    pub fn create_with_yubikey(
        path: PathBuf,
        password: String,
        yubikey: Option<YubikeyConfig>,
    ) -> Result<Self, DatabaseError> {
        let secret_password = SecretString::new(password.into_boxed_str());

        let db_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Root")
            .to_string();

        let mut config = DatabaseConfig::default();
        config.kdf_config = default_kdf_config();

        let mut db = KeepassDatabase::with_config(config);
        db.meta.database_name = Some(db_name.clone());
        db.root_mut().name = db_name;

        let mut new_db = Self {
            db,
            path: path.clone(),
            password: secret_password,
            yubikey,
            last_modified: None,
        };

        new_db.save()?;
        new_db.last_modified = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());

        Ok(new_db)
    }

    pub fn open(path: PathBuf, password: String) -> Result<Self, DatabaseError> {
        Self::open_with_yubikey(path, password, None)
    }

    pub fn open_with_yubikey(
        path: PathBuf,
        password: String,
        yubikey: Option<YubikeyConfig>,
    ) -> Result<Self, DatabaseError> {
        let secret_password = SecretString::new(password.into_boxed_str());

        let mut file = File::open(&path)
            .map_err(|e| DatabaseError::OpenError(format!("Failed to open file: {}", e)))?;

        let key = build_key(&secret_password, yubikey.as_ref())?;

        let db = KeepassDatabase::open(&mut file, key).map_err(|e| {
            // keepass 0.13.x surfaces a wrong-password failure as "Incorrect key";
            // pre-0.13 used "Invalid credentials". We keep both so a future
            // upstream rename doesn't silently downgrade to a generic OpenError.
            let msg = e.to_string();
            if msg.contains("Incorrect key") || msg.contains("Invalid credentials") {
                DatabaseError::InvalidCredentials
            } else {
                DatabaseError::OpenError(msg)
            }
        })?;

        let last_modified = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());

        Ok(Self {
            db,
            path,
            password: secret_password,
            yubikey,
            last_modified,
        })
    }

    pub fn save(&mut self) -> Result<(), DatabaseError> {
        let key = build_key(&self.password, self.yubikey.as_ref())?;

        let mut file = File::create(&self.path)
            .map_err(|e| DatabaseError::SaveError(format!("Failed to create file: {}", e)))?;

        self.db
            .save(&mut file, key)
            .map_err(|e| DatabaseError::SaveError(e.to_string()))?;

        self.last_modified = std::fs::metadata(&self.path)
            .ok()
            .and_then(|m| m.modified().ok());

        Ok(())
    }

    /// Add (or replace) the Yubikey requirement on the currently-open
    /// database. The next `save()` re-encrypts the file using master
    /// password + Yubikey challenge-response.
    pub fn enable_yubikey(&mut self, config: YubikeyConfig) -> Result<(), DatabaseError> {
        // Verify the configured Yubikey is actually plugged in BEFORE
        // we commit. Otherwise the user would lose access to their DB
        // on the next save.
        let _ = ChallengeResponseKey::get_yubikey(Some(config.serial_number)).map_err(|e| {
            DatabaseError::OpenError(format!(
                "Yubikey with serial {} not detected: {}",
                config.serial_number, e
            ))
        })?;
        self.yubikey = Some(config);
        self.save()
    }

    /// Drop the Yubikey requirement. Saves the file with master-password
    /// only encryption afterwards.
    pub fn disable_yubikey(&mut self) -> Result<(), DatabaseError> {
        self.yubikey = None;
        self.save()
    }

    pub fn check_for_changes(&self) -> Result<bool, DatabaseError> {
        let current_modified = std::fs::metadata(&self.path)
            .ok()
            .and_then(|m| m.modified().ok());

        Ok(match (self.last_modified, current_modified) {
            (Some(last), Some(current)) => current > last,
            _ => false,
        })
    }

    pub fn merge_database(&mut self) -> Result<(), DatabaseError> {
        let mut file = File::open(&self.path)
            .map_err(|e| DatabaseError::OpenError(format!("Failed to open file: {}", e)))?;

        // Merge requires reading the on-disk version, which means a fresh
        // Yubikey touch if challenge-response is enabled. There's no way
        // around that — the file is encrypted with the live challenge
        // result, not a stored key.
        let key = build_key(&self.password, self.yubikey.as_ref())?;

        let disk_db = KeepassDatabase::open(&mut file, key)
            .map_err(|e| DatabaseError::OpenError(e.to_string()))?;

        let root_id = self.db.root().id();
        Self::merge_into(&mut self.db, &disk_db, root_id, root_id);

        self.last_modified = std::fs::metadata(&self.path)
            .ok()
            .and_then(|m| m.modified().ok());

        Ok(())
    }

    // Merge groups+entries from `source` (under source_group_id) into `target`
    // (under target_group_id). Same-UUID items keep the newer-modified version;
    // items only in source are copied across.
    fn merge_into(
        target: &mut KeepassDatabase,
        source: &KeepassDatabase,
        source_group_id: GroupId,
        target_group_id: GroupId,
    ) {
        let Some(source_group) = source.group(source_group_id) else {
            return;
        };

        let source_entry_ids: Vec<EntryId> = source_group.entry_ids().collect();
        let source_group_ids: Vec<GroupId> = source_group.group_ids().collect();

        // Snapshot existing children of the target group so we don't iterate
        // while we mutate.
        let target_entry_ids: Vec<EntryId> = match target.group(target_group_id) {
            Some(g) => g.entry_ids().collect(),
            None => return,
        };
        let target_group_ids: Vec<GroupId> = match target.group(target_group_id) {
            Some(g) => g.group_ids().collect(),
            None => return,
        };

        // Merge entries: same id → keep newer; missing → copy
        for src_entry_id in &source_entry_ids {
            let Some(src_entry_ref) = source.entry(*src_entry_id) else {
                continue;
            };
            let src_entry = src_entry_ref.clone();

            if target_entry_ids.contains(src_entry_id) {
                // Compute the comparison in a scope so the immutable EntryRef
                // borrow ends before we take the mutable entry_mut below.
                let src_is_newer = target
                    .entry(*src_entry_id)
                    .map(|target_entry_ref| {
                        src_entry_ref.times.last_modification
                            > target_entry_ref.times.last_modification
                    })
                    .unwrap_or(false);
                if src_is_newer {
                    if let Some(mut tgt_mut) = target.entry_mut(*src_entry_id) {
                        tgt_mut.fields = src_entry.fields.clone();
                        tgt_mut.tags = src_entry.tags.clone();
                        tgt_mut.times = src_entry.times.clone();
                        tgt_mut.custom_data = src_entry.custom_data.clone();
                        tgt_mut.history = src_entry.history.clone();
                    }
                }
            } else if let Some(mut tgt_group) = target.group_mut(target_group_id) {
                if let Ok(mut new_entry) = tgt_group.add_entry_with_id(*src_entry_id) {
                    new_entry.fields = src_entry.fields.clone();
                    new_entry.tags = src_entry.tags.clone();
                    new_entry.times = src_entry.times.clone();
                    new_entry.custom_data = src_entry.custom_data.clone();
                    new_entry.history = src_entry.history.clone();
                }
            }
        }

        // Merge sub-groups: same id → recurse; missing → create and recurse
        for src_group_id in &source_group_ids {
            let Some(src_subgroup) = source.group(*src_group_id) else {
                continue;
            };
            let src_name = src_subgroup.name.clone();

            if !target_group_ids.contains(src_group_id) {
                if let Some(mut tgt_parent) = target.group_mut(target_group_id) {
                    if let Ok(mut new_group) = tgt_parent.add_group_with_id(*src_group_id) {
                        new_group.name = src_name;
                    }
                }
            }
            Self::merge_into(target, source, *src_group_id, *src_group_id);
        }
    }

    /// Compare the in-memory DB against the on-disk version and return the
    /// list of entries that exist in both but have diverging user-visible
    /// fields. Used by the conflict-resolution dialog before a merge.
    ///
    /// Requires reading the on-disk file → a Yubikey touch when CR is
    /// enabled. Callers should gate this behind `check_for_changes()` so
    /// they don't burn a touch when nothing has changed.
    pub fn analyze_conflicts(&self) -> Result<Vec<EntryConflict>, DatabaseError> {
        let mut file = File::open(&self.path)
            .map_err(|e| DatabaseError::OpenError(format!("Failed to open file: {}", e)))?;
        let key = build_key(&self.password, self.yubikey.as_ref())?;
        let disk_db = KeepassDatabase::open(&mut file, key)
            .map_err(|e| DatabaseError::OpenError(e.to_string()))?;

        let mut conflicts = Vec::new();
        for local_ref in self.db.iter_all_entries() {
            let id = local_ref.id();
            let Some(remote_ref) = disk_db.entry(id) else {
                continue;
            };

            let local_group = local_ref.parent().id().uuid().to_string();
            let remote_group = remote_ref.parent().id().uuid().to_string();
            let local_data = Self::convert_entry(local_ref, &local_group);
            let remote_data = Self::convert_entry(remote_ref, &remote_group);

            if Self::entry_data_differs(&local_data, &remote_data) {
                conflicts.push(EntryConflict {
                    uuid: id.uuid().to_string(),
                    local: local_data,
                    remote: remote_data,
                });
            }
        }
        Ok(conflicts)
    }

    fn entry_data_differs(a: &super::types::EntryData, b: &super::types::EntryData) -> bool {
        a.title != b.title
            || a.username != b.username
            || a.password != b.password
            || a.url != b.url
            || a.notes != b.notes
            || a.tags != b.tags
            || a.is_favorite != b.is_favorite
            || a.icon_id != b.icon_id
            || a.group_uuid != b.group_uuid
            || a.custom_fields.len() != b.custom_fields.len()
            || a.custom_fields.iter().zip(b.custom_fields.iter()).any(|(x, y)| {
                x.name != y.name || x.value != y.value || x.protected != y.protected
            })
    }

    /// Apply per-entry user decisions, then run the standard newer-wins merge
    /// for everything else. Updates `last_modified` so subsequent change
    /// detection works correctly.
    pub fn resolve_and_merge(
        &mut self,
        decisions: HashMap<String, ConflictChoice>,
    ) -> Result<(), DatabaseError> {
        let mut file = File::open(&self.path)
            .map_err(|e| DatabaseError::OpenError(format!("Failed to open file: {}", e)))?;
        let key = build_key(&self.password, self.yubikey.as_ref())?;
        let disk_db = KeepassDatabase::open(&mut file, key)
            .map_err(|e| DatabaseError::OpenError(e.to_string()))?;

        // Apply per-entry choices BEFORE the generic merge. Touching
        // last_modification on a "keep local" entry guarantees merge_into's
        // newer-wins check leaves us alone. Copying remote fields onto a
        // "keep remote" entry means the subsequent merge_into is a no-op
        // for that entry (fields identical, times bumped to remote's).
        for (uuid_str, choice) in &decisions {
            let id = match Self::parse_entry_id(uuid_str) {
                Ok(id) => id,
                Err(_) => continue,
            };
            match choice {
                ConflictChoice::KeepLocal => {
                    if let Some(mut local) = self.db.entry_mut(id) {
                        local.times.last_modification = Some(Times::now());
                    }
                }
                ConflictChoice::KeepRemote => {
                    let Some(remote) = disk_db.entry(id) else {
                        continue;
                    };
                    let r = (*remote).clone();
                    if let Some(mut local) = self.db.entry_mut(id) {
                        local.fields = r.fields;
                        local.tags = r.tags;
                        local.times = r.times;
                        local.custom_data = r.custom_data;
                        local.history = r.history;
                    }
                }
            }
        }

        let root_id = self.db.root().id();
        Self::merge_into(&mut self.db, &disk_db, root_id, root_id);

        self.last_modified = std::fs::metadata(&self.path)
            .ok()
            .and_then(|m| m.modified().ok());
        Ok(())
    }

    pub fn get_kdf_info(&self) -> KdfInfo {
        match &self.db.config.kdf_config {
            KdfConfig::Aes { rounds } => KdfInfo {
                kdf_type: "AES".to_string(),
                is_weak: *rounds < 60000,
                iterations: Some(*rounds),
                memory: None,
                parallelism: None,
            },
            KdfConfig::Argon2 {
                iterations,
                memory,
                parallelism,
                ..
            } => {
                let memory_mb = memory / (1024 * 1024);
                let is_weak = *iterations < 2 || memory_mb < 64 || *parallelism < 2;
                KdfInfo {
                    kdf_type: "Argon2d".to_string(),
                    is_weak,
                    iterations: Some(*iterations),
                    memory: Some(*memory),
                    parallelism: Some(*parallelism),
                }
            }
            KdfConfig::Argon2id {
                iterations,
                memory,
                parallelism,
                ..
            } => {
                let memory_mb = memory / (1024 * 1024);
                let is_weak = *iterations < 2 || memory_mb < 64 || *parallelism < 2;
                KdfInfo {
                    kdf_type: "Argon2id".to_string(),
                    is_weak,
                    iterations: Some(*iterations),
                    memory: Some(*memory),
                    parallelism: Some(*parallelism),
                }
            }
            _ => KdfInfo {
                kdf_type: "Unknown".to_string(),
                is_weak: true,
                iterations: None,
                memory: None,
                parallelism: None,
            },
        }
    }

    pub fn upgrade_kdf_parameters(&mut self) -> Result<(), DatabaseError> {
        self.db.config.kdf_config = default_kdf_config();
        self.save()?;
        Ok(())
    }
}

// Production: OWASP-recommended Argon2id (64 MB, 2 iter, 2 lanes).
// Tests: fast settings so the full suite stays under a few seconds —
// we're testing format correctness, not KDF strength.
#[cfg(not(test))]
fn default_kdf_config() -> KdfConfig {
    KdfConfig::Argon2id {
        iterations: 2,
        memory: 64 * 1024 * 1024,
        parallelism: 2,
        version: Argon2Version::Version13,
    }
}

#[cfg(test)]
fn default_kdf_config() -> KdfConfig {
    KdfConfig::Argon2id {
        iterations: 1,
        memory: 1024 * 1024, // 1 MB
        parallelism: 1,
        version: Argon2Version::Version13,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fresh_db() -> (TempDir, Database) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.kdbx");
        let db = Database::create(path, "correct horse".into()).unwrap();
        (dir, db)
    }

    #[test]
    fn create_writes_file_to_disk() {
        let (dir, db) = fresh_db();
        assert!(db.path.exists());
        assert!(db.last_modified.is_some());
        drop(dir);
    }

    #[test]
    fn create_uses_filename_as_database_name() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("MyVault.kdbx");
        let db = Database::create(path, "pw".into()).unwrap();
        assert_eq!(db.db.meta.database_name.as_deref(), Some("MyVault"));
        assert_eq!(db.db.root().name, "MyVault");
    }

    #[test]
    fn open_with_correct_password_succeeds() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.kdbx");
        let _db = Database::create(path.clone(), "secret".into()).unwrap();
        let reopened = Database::open(path, "secret".into()).unwrap();
        assert_eq!(reopened.db.root().name, "vault");
    }

    #[test]
    fn open_with_wrong_password_returns_invalid_credentials() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.kdbx");
        let _ = Database::create(path.clone(), "secret".into()).unwrap();
        let err = Database::open(path, "wrong".into()).err().unwrap();
        assert!(
            matches!(err, DatabaseError::InvalidCredentials),
            "expected InvalidCredentials, got: {}",
            err
        );
    }

    #[test]
    fn open_with_missing_file_returns_open_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("does_not_exist.kdbx");
        let err = Database::open(path, "x".into()).err().unwrap();
        assert!(matches!(err, DatabaseError::OpenError(_)));
    }

    #[test]
    fn save_updates_last_modified() {
        let (_dir, mut db) = fresh_db();
        let first = db.last_modified;
        // Sleep to ensure mtime resolution sees a difference on Windows (100ns)
        // and Linux ext4 (1s). 1.1s covers both.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        db.save().unwrap();
        assert!(db.last_modified > first);
    }

    #[test]
    fn check_for_changes_false_when_only_we_wrote() {
        let (_dir, db) = fresh_db();
        assert!(!db.check_for_changes().unwrap());
    }

    #[test]
    fn check_for_changes_detects_external_write() {
        let (_dir, db) = fresh_db();
        // Sleep past the filesystem mtime resolution (Linux ext4 = 1s).
        std::thread::sleep(std::time::Duration::from_millis(1100));
        // Rewrite the file's bytes from "outside" the Database struct.
        let bytes = std::fs::read(&db.path).unwrap();
        std::fs::write(&db.path, bytes).unwrap();
        assert!(db.check_for_changes().unwrap());
    }

    #[test]
    fn kdf_info_reports_argon2id() {
        let (_dir, db) = fresh_db();
        let kdf = db.get_kdf_info();
        assert_eq!(kdf.kdf_type, "Argon2id");
        // In test mode KDF is intentionally weak.
        assert!(kdf.is_weak);
    }

    #[test]
    fn root_group_is_returned() {
        let (_dir, db) = fresh_db();
        let root = db.get_root_group();
        assert_eq!(root.name, "test");
        assert!(root.parent_uuid.is_none());
        assert!(root.children.is_empty());
    }
}
