//! KeePass-style lock-file plumbing for shared DB files.
//!
//! When two devices share a `.kdbx` (Dropbox/OneDrive/Nextcloud), the cloud
//! client may flush both writes in close succession. The lock file is a
//! courtesy signal — it doesn't *prevent* the other side from writing, but it
//! lets the UI surface "someone else is editing" before we clobber their
//! save.
//!
//! Format: `<db>.lock`, JSON. Compatible-ish with what KeePass classic writes
//! (text owner line), but extended so we can detect *our* lock vs theirs and
//! age them out after a crash.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Foreign locks older than this are treated as stale (process probably
/// crashed). A real-world save shouldn't take more than a few seconds even
/// on cold KDF; 10 minutes is generous but bounded.
const STALE_AFTER: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LockInfo {
    pub pid: u32,
    pub host: String,
    /// Wall-clock UNIX timestamp (seconds since epoch). Cross-machine clocks
    /// can drift, so this is best-effort — we only use it to age out stale
    /// locks from crashed peers.
    pub acquired_at: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("Database is locked by {0} (pid {1})")]
    HeldByPeer(String, u32),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Lock file corrupt; cannot determine owner")]
    Corrupt,
}

fn lock_path(db_path: &Path) -> PathBuf {
    let mut p = db_path.as_os_str().to_os_string();
    p.push(".lock");
    PathBuf::from(p)
}

fn current_host() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn read_lock(path: &Path) -> Result<LockInfo, LockError> {
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(|_| LockError::Corrupt)
}

/// Try to acquire a lock on `db_path`. Idempotent for our own PID. Returns
/// `HeldByPeer` if a fresh foreign lock exists. Foreign locks older than
/// `STALE_AFTER` are stolen — the original holder crashed.
pub fn acquire_lock(db_path: &Path) -> Result<LockInfo, LockError> {
    let path = lock_path(db_path);
    let me = LockInfo {
        pid: std::process::id(),
        host: current_host(),
        acquired_at: now_secs(),
    };

    if path.exists() {
        match read_lock(&path) {
            Ok(existing) => {
                let ours = existing.pid == me.pid && existing.host == me.host;
                let stale = now_secs().saturating_sub(existing.acquired_at)
                    > STALE_AFTER.as_secs();
                if !ours && !stale {
                    return Err(LockError::HeldByPeer(existing.host, existing.pid));
                }
                // Either our own lock (re-acquire after restart) or a stale
                // peer lock — both safe to overwrite.
            }
            Err(_) => {
                // Corrupt lock from a partial write. Treat as no lock so
                // the user isn't stuck behind a malformed file.
            }
        }
    }

    let json = serde_json::to_vec_pretty(&me).map_err(|_| LockError::Corrupt)?;
    let mut f = fs::File::create(&path)?;
    f.write_all(&json)?;
    f.sync_all().ok();
    Ok(me)
}

/// Drop our lock. Silently no-ops if the lock is missing or owned by someone
/// else (we don't steal locks on release).
pub fn release_lock(db_path: &Path) -> io::Result<()> {
    let path = lock_path(db_path);
    if !path.exists() {
        return Ok(());
    }
    match read_lock(&path) {
        Ok(info)
            if info.pid == std::process::id() && info.host == current_host() =>
        {
            fs::remove_file(&path)
        }
        // Foreign or corrupt lock — leave it alone.
        _ => Ok(()),
    }
}

/// Inspect the current lock state without modifying anything. Returns `None`
/// if no fresh foreign lock exists (no file, our own, stale, or corrupt).
pub fn peek_foreign_lock(db_path: &Path) -> Option<LockInfo> {
    let path = lock_path(db_path);
    let info = read_lock(&path).ok()?;
    if info.pid == std::process::id() && info.host == current_host() {
        return None;
    }
    if now_secs().saturating_sub(info.acquired_at) > STALE_AFTER.as_secs() {
        return None;
    }
    Some(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fake_db(dir: &TempDir) -> PathBuf {
        let p = dir.path().join("vault.kdbx");
        fs::write(&p, b"not a real kdbx").unwrap();
        p
    }

    #[test]
    fn acquire_then_release_round_trips() {
        let dir = TempDir::new().unwrap();
        let db = fake_db(&dir);
        let info = acquire_lock(&db).unwrap();
        assert_eq!(info.pid, std::process::id());
        assert!(lock_path(&db).exists());
        release_lock(&db).unwrap();
        assert!(!lock_path(&db).exists());
    }

    #[test]
    fn re_acquire_by_same_pid_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let db = fake_db(&dir);
        acquire_lock(&db).unwrap();
        // Second call must not fail — restart-without-clean-release is a
        // real scenario (debugger stop, OS sleep, etc.).
        acquire_lock(&db).unwrap();
        release_lock(&db).unwrap();
    }

    #[test]
    fn foreign_fresh_lock_blocks_acquire() {
        let dir = TempDir::new().unwrap();
        let db = fake_db(&dir);
        let foreign = LockInfo {
            pid: u32::MAX, // not us
            host: "other-machine".to_string(),
            acquired_at: now_secs(),
        };
        fs::write(lock_path(&db), serde_json::to_vec(&foreign).unwrap()).unwrap();
        match acquire_lock(&db) {
            Err(LockError::HeldByPeer(host, _)) => assert_eq!(host, "other-machine"),
            other => panic!("expected HeldByPeer, got {:?}", other),
        }
    }

    #[test]
    fn stale_foreign_lock_is_stolen() {
        let dir = TempDir::new().unwrap();
        let db = fake_db(&dir);
        let stale = LockInfo {
            pid: u32::MAX,
            host: "crashed-machine".to_string(),
            acquired_at: now_secs() - STALE_AFTER.as_secs() - 60,
        };
        fs::write(lock_path(&db), serde_json::to_vec(&stale).unwrap()).unwrap();
        let mine = acquire_lock(&db).unwrap();
        assert_eq!(mine.pid, std::process::id());
        release_lock(&db).unwrap();
    }

    #[test]
    fn release_leaves_foreign_lock_alone() {
        let dir = TempDir::new().unwrap();
        let db = fake_db(&dir);
        let foreign = LockInfo {
            pid: u32::MAX,
            host: "other".to_string(),
            acquired_at: now_secs(),
        };
        let raw = serde_json::to_vec(&foreign).unwrap();
        fs::write(lock_path(&db), &raw).unwrap();
        release_lock(&db).unwrap();
        assert!(lock_path(&db).exists());
        let still_there = read_lock(&lock_path(&db)).unwrap();
        assert_eq!(still_there.host, "other");
    }

    #[test]
    fn peek_foreign_lock_filters_self_and_stale() {
        let dir = TempDir::new().unwrap();
        let db = fake_db(&dir);

        // Our own lock — peek returns None.
        acquire_lock(&db).unwrap();
        assert!(peek_foreign_lock(&db).is_none());
        release_lock(&db).unwrap();

        // Fresh foreign — peek returns Some.
        let fresh = LockInfo {
            pid: 99,
            host: "alice".to_string(),
            acquired_at: now_secs(),
        };
        fs::write(lock_path(&db), serde_json::to_vec(&fresh).unwrap()).unwrap();
        assert_eq!(peek_foreign_lock(&db).unwrap().host, "alice");

        // Stale foreign — peek returns None.
        let stale = LockInfo {
            pid: 99,
            host: "bob".to_string(),
            acquired_at: now_secs() - STALE_AFTER.as_secs() - 1,
        };
        fs::write(lock_path(&db), serde_json::to_vec(&stale).unwrap()).unwrap();
        assert!(peek_foreign_lock(&db).is_none());
    }
}
