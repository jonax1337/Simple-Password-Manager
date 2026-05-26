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
