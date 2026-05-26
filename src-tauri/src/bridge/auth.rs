use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const TOKEN_BYTES: usize = 32; // 256-bit
pub const BRIDGE_FILE_NAME: &str = "bridge.json";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BridgeFile {
    pub port: u16,
    pub token: String,
    pub pid: u32,
}

pub fn generate_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn write_bridge_file(dir: &Path, file: &BridgeFile) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(BRIDGE_FILE_NAME);

    let json = serde_json::to_string_pretty(file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    // Write atomically: write to <name>.tmp then rename.
    let tmp = dir.join(format!("{}.tmp", BRIDGE_FILE_NAME));
    std::fs::write(&tmp, json)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }

    std::fs::rename(&tmp, &path)?;
    Ok(path)
}

#[allow(dead_code)]
pub fn read_bridge_file(dir: &Path) -> std::io::Result<BridgeFile> {
    let path = dir.join(BRIDGE_FILE_NAME);
    let raw = std::fs::read_to_string(path)?;
    serde_json::from_str(&raw)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

pub fn remove_bridge_file(dir: &Path) {
    let _ = std::fs::remove_file(dir.join(BRIDGE_FILE_NAME));
}

// Constant-time token comparison to avoid timing attacks on the auth header.
pub fn token_matches(expected: &str, candidate: &str) -> bool {
    if expected.len() != candidate.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (a, b) in expected.bytes().zip(candidate.bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn token_is_64_hex_chars() {
        let t = generate_token();
        assert_eq!(t.len(), 64);
        assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn tokens_are_unique() {
        let a = generate_token();
        let b = generate_token();
        assert_ne!(a, b);
    }

    #[test]
    fn token_matches_equal_strings() {
        assert!(token_matches("abc123", "abc123"));
    }

    #[test]
    fn token_matches_rejects_different_strings_of_same_length() {
        assert!(!token_matches("abc123", "abc124"));
    }

    #[test]
    fn token_matches_rejects_length_mismatch() {
        assert!(!token_matches("abc", "abc123"));
        assert!(!token_matches("abc123", "abc"));
    }

    #[test]
    fn token_matches_empty_strings() {
        assert!(token_matches("", ""));
    }

    #[test]
    fn bridge_file_roundtrip() {
        let dir = TempDir::new().unwrap();
        let original = BridgeFile {
            port: 54321,
            token: "deadbeef".to_string(),
            pid: 4242,
        };
        write_bridge_file(dir.path(), &original).unwrap();

        let loaded = read_bridge_file(dir.path()).unwrap();
        assert_eq!(loaded.port, original.port);
        assert_eq!(loaded.token, original.token);
        assert_eq!(loaded.pid, original.pid);
    }

    #[test]
    fn bridge_file_is_atomic_no_tmp_left_over() {
        let dir = TempDir::new().unwrap();
        let f = BridgeFile {
            port: 1,
            token: "x".into(),
            pid: 1,
        };
        write_bridge_file(dir.path(), &f).unwrap();

        let tmp_path = dir.path().join(format!("{}.tmp", BRIDGE_FILE_NAME));
        assert!(!tmp_path.exists(), "temp file should be renamed away");
    }

    #[test]
    fn remove_bridge_file_is_idempotent() {
        let dir = TempDir::new().unwrap();
        remove_bridge_file(dir.path());
        remove_bridge_file(dir.path());

        let f = BridgeFile { port: 1, token: "x".into(), pid: 1 };
        write_bridge_file(dir.path(), &f).unwrap();
        remove_bridge_file(dir.path());
        assert!(!dir.path().join(BRIDGE_FILE_NAME).exists());
    }
}
