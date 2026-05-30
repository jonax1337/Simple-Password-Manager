use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub name: String,
    pub value: String,
    pub protected: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub title: String,
    pub username: String,
    pub password: String,
    pub url: String,
    pub notes: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EntryData {
    pub uuid: String,
    pub title: String,
    pub username: String,
    pub password: String,
    pub url: String,
    pub notes: String,
    pub tags: String,
    pub group_uuid: String,
    pub icon_id: Option<usize>,
    pub is_favorite: bool,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub last_accessed: Option<String>,
    pub expiry_time: Option<String>,
    pub expires: bool,
    pub usage_count: usize,
    pub custom_fields: Vec<CustomField>,
    pub history: Vec<HistoryEntry>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GroupData {
    pub uuid: String,
    pub name: String,
    pub parent_uuid: Option<String>,
    pub children: Vec<GroupData>,
    pub icon_id: Option<usize>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KdfInfo {
    pub kdf_type: String,
    pub is_weak: bool,
    pub iterations: Option<u64>,
    pub memory: Option<u64>,
    pub parallelism: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct YubikeyConfig {
    /// 32-bit serial number printed on the back of the YubiKey. Used to
    /// reconnect to the same physical device on subsequent unlocks even
    /// when multiple keys are plugged in.
    pub serial_number: u32,
    /// "1" or "2" — which Yubikey slot is programmed for HMAC-SHA1
    /// challenge-response. Kept as a string because that's what the
    /// keepass crate's API takes.
    pub slot: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct YubikeyInfo {
    pub serial_number: u32,
    pub name: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EntryConflict {
    pub uuid: String,
    pub local: EntryData,
    pub remote: EntryData,
}

/// Per-entry user decision when resolving a sync conflict. `KeepLocal` forces
/// the in-memory version to win the next merge; `KeepRemote` copies the
/// on-disk fields onto the in-memory entry before merging.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictChoice {
    KeepLocal,
    KeepRemote,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_entries: usize,
    pub total_groups: usize,
    pub weak_passwords: usize,
    pub reused_passwords: usize,
    pub old_passwords: usize,
    pub expired_entries: usize,
    pub favorite_entries: usize,
    pub average_password_strength: f64,
}
