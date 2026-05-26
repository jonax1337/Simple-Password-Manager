use chrono::NaiveDateTime;
use keepass::db::{fields, Entry, EntryId, EntryRef, History, Icon, Times};
use uuid::Uuid;

use super::database::Database;
use super::error::DatabaseError;
use super::types::{CustomField, EntryData, HistoryEntry};

const FAVORITE_FIELD: &str = "_Favorite";
const TAGS_FIELD: &str = "Tags";
const STANDARD_FIELDS: &[&str] = &[
    fields::TITLE,
    fields::USERNAME,
    fields::PASSWORD,
    fields::URL,
    fields::NOTES,
    TAGS_FIELD,
    FAVORITE_FIELD,
];

impl Database {
    pub fn get_entries_in_group(&self, group_uuid: &str) -> Result<Vec<EntryData>, DatabaseError> {
        let group_id = Self::parse_group_id(group_uuid)?;
        let group = self.db.group(group_id).ok_or(DatabaseError::GroupNotFound)?;

        let entries = group
            .entries()
            .map(|entry| Self::convert_entry(entry, group_uuid))
            .collect();

        Ok(entries)
    }

    pub fn get_all_entries(&self) -> Vec<EntryData> {
        self.db
            .iter_all_entries()
            .map(|entry| {
                let parent_uuid = entry.parent().id().uuid().to_string();
                Self::convert_entry(entry, &parent_uuid)
            })
            .collect()
    }

    pub(super) fn convert_entry(entry: EntryRef<'_>, group_uuid: &str) -> EntryData {
        let uuid = entry.id().uuid().to_string();
        let is_favorite = entry.get(FAVORITE_FIELD).unwrap_or("") == "true";

        let created = entry
            .times
            .creation
            .map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string());
        let modified = entry
            .times
            .last_modification
            .map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string());
        let last_accessed = entry
            .times
            .last_access
            .map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string());
        // Add 1 hour to compensate for keepass-rs timezone conversion when reading
        let expiry_time = entry.times.expiry.map(|t| {
            let adjusted = t + chrono::Duration::hours(1);
            adjusted.format("%Y-%m-%dT%H:%M").to_string()
        });

        let custom_fields: Vec<CustomField> = entry
            .fields
            .iter()
            .filter(|(key, _)| !STANDARD_FIELDS.contains(&key.as_str()))
            .map(|(key, value)| {
                let val = value.get().clone();
                let protected = value.is_protected();
                CustomField {
                    name: key.clone(),
                    value: val,
                    protected,
                }
            })
            .collect();

        let history: Vec<HistoryEntry> = entry
            .history
            .as_ref()
            .map(|hist| {
                hist.get_entries()
                    .iter()
                    .map(|h| HistoryEntry {
                        timestamp: h
                            .times
                            .last_modification
                            .map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string())
                            .unwrap_or_default(),
                        title: h.get_title().unwrap_or("").to_string(),
                        username: h.get_username().unwrap_or("").to_string(),
                        password: h.get_password().unwrap_or("").to_string(),
                        url: h.get(fields::URL).unwrap_or("").to_string(),
                        notes: h.get(fields::NOTES).unwrap_or("").to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let icon_id = match entry.icon() {
            Some(Icon::BuiltIn(id)) => Some(*id),
            _ => None,
        };

        EntryData {
            uuid,
            title: entry.get_title().unwrap_or("").to_string(),
            username: entry.get_username().unwrap_or("").to_string(),
            password: entry.get_password().unwrap_or("").to_string(),
            url: entry.get(fields::URL).unwrap_or("").to_string(),
            notes: entry.get(fields::NOTES).unwrap_or("").to_string(),
            tags: entry.get(TAGS_FIELD).unwrap_or("").to_string(),
            group_uuid: group_uuid.to_string(),
            icon_id,
            is_favorite,
            created,
            modified,
            last_accessed,
            expiry_time,
            expires: entry.times.expires.unwrap_or(false),
            usage_count: entry.times.usage_count.unwrap_or(0),
            custom_fields,
            history,
        }
    }

    pub fn get_entry(&self, entry_uuid: &str) -> Result<EntryData, DatabaseError> {
        let entry_id = Self::parse_entry_id(entry_uuid)?;
        let entry = self.db.entry(entry_id).ok_or(DatabaseError::EntryNotFound)?;
        let group_uuid = entry.parent().id().uuid().to_string();
        Ok(Self::convert_entry(entry, &group_uuid))
    }

    pub fn create_entry(&mut self, entry_data: EntryData) -> Result<(), DatabaseError> {
        let group_id = Self::parse_group_id(&entry_data.group_uuid)?;

        let entry_id = if entry_data.uuid.is_empty() {
            EntryId::from_uuid(Uuid::new_v4())
        } else {
            Self::parse_entry_id(&entry_data.uuid)?
        };

        let mut group = self
            .db
            .group_mut(group_id)
            .ok_or(DatabaseError::GroupNotFound)?;

        let mut entry = group
            .add_entry_with_id(entry_id)
            .map_err(|_| DatabaseError::EntryNotFound)?;

        let now = Times::now();
        entry.times.creation = Some(now);
        entry.times.last_modification = Some(now);
        entry.times.last_access = Some(now);

        entry.set_unprotected(fields::TITLE, entry_data.title);
        entry.set_unprotected(fields::USERNAME, entry_data.username);
        entry.set_protected(fields::PASSWORD, entry_data.password);
        if !entry_data.url.is_empty() {
            entry.set_unprotected(fields::URL, entry_data.url);
        }
        if !entry_data.notes.is_empty() {
            entry.set_unprotected(fields::NOTES, entry_data.notes);
        }
        if !entry_data.tags.is_empty() {
            entry.set_unprotected(TAGS_FIELD, entry_data.tags);
        }
        if entry_data.is_favorite {
            entry.set_unprotected(FAVORITE_FIELD, "true");
        }

        for field in entry_data.custom_fields {
            if field.protected {
                entry.set_protected(field.name, field.value);
            } else {
                entry.set_unprotected(field.name, field.value);
            }
        }

        if let Some(icon) = entry_data.icon_id {
            entry.set_icon_builtin(icon);
        }

        entry.times.expires = Some(entry_data.expires);
        if entry_data.expires {
            if let Some(expiry_str) = entry_data.expiry_time {
                entry.times.expiry = parse_expiry(&expiry_str);
            }
        }

        Ok(())
    }

    pub fn update_entry(&mut self, entry_data: EntryData) -> Result<(), DatabaseError> {
        let entry_id = Self::parse_entry_id(&entry_data.uuid)?;

        // Snapshot current state for the history entry, if any user-visible
        // field is actually changing.
        let history_snapshot: Option<Entry> = {
            let current = self
                .db
                .entry(entry_id)
                .ok_or(DatabaseError::EntryNotFound)?;
            let any_change = current.get_title().unwrap_or("") != entry_data.title
                || current.get_username().unwrap_or("") != entry_data.username
                || current.get_password().unwrap_or("") != entry_data.password
                || current.get(fields::URL).unwrap_or("") != entry_data.url
                || current.get(fields::NOTES).unwrap_or("") != entry_data.notes
                || current.get(TAGS_FIELD).unwrap_or("") != entry_data.tags;

            if any_change {
                let mut clone = (*current).clone();
                clone.history = None;
                Some(clone)
            } else {
                None
            }
        };

        let mut entry = self
            .db
            .entry_mut(entry_id)
            .ok_or(DatabaseError::EntryNotFound)?;

        if let Some(snapshot) = history_snapshot {
            if let Some(ref mut hist) = entry.history {
                hist.add_entry(snapshot);
            } else {
                let mut new_history = History::default();
                new_history.add_entry(snapshot);
                entry.history = Some(new_history);
            }
        }

        let now = Times::now();
        entry.times.last_modification = Some(now);
        entry.times.last_access = Some(now);

        // Drop existing non-standard fields, then write the desired state.
        entry
            .fields
            .retain(|key, _| STANDARD_FIELDS.contains(&key.as_str()));

        entry.set_unprotected(fields::TITLE, entry_data.title);
        entry.set_unprotected(fields::USERNAME, entry_data.username);
        entry.set_protected(fields::PASSWORD, entry_data.password);
        entry.set_unprotected(fields::URL, entry_data.url);
        entry.set_unprotected(fields::NOTES, entry_data.notes);
        entry.set_unprotected(TAGS_FIELD, entry_data.tags);

        if entry_data.is_favorite {
            entry.set_unprotected(FAVORITE_FIELD, "true");
        } else {
            entry.fields.remove(FAVORITE_FIELD);
        }

        for field in entry_data.custom_fields {
            if field.protected {
                entry.set_protected(field.name, field.value);
            } else {
                entry.set_unprotected(field.name, field.value);
            }
        }

        if let Some(icon) = entry_data.icon_id {
            entry.set_icon_builtin(icon);
        } else {
            entry.set_icon_none();
        }

        entry.times.expires = Some(entry_data.expires);
        if entry_data.expires {
            if let Some(expiry_str) = entry_data.expiry_time {
                if !expiry_str.is_empty() {
                    entry.times.expiry = parse_expiry(&expiry_str);
                }
            }
        }

        Ok(())
    }

    pub fn delete_entry(&mut self, entry_uuid: &str) -> Result<(), DatabaseError> {
        let entry_id = Self::parse_entry_id(entry_uuid)?;
        let entry = self
            .db
            .entry_mut(entry_id)
            .ok_or(DatabaseError::EntryNotFound)?;
        entry.remove();
        Ok(())
    }

    pub fn move_entry(
        &mut self,
        entry_uuid: &str,
        new_group_uuid: &str,
    ) -> Result<(), DatabaseError> {
        let entry_id = Self::parse_entry_id(entry_uuid)?;
        let new_group_id = Self::parse_group_id(new_group_uuid)?;

        let mut entry = self
            .db
            .entry_mut(entry_id)
            .ok_or(DatabaseError::EntryNotFound)?;

        entry
            .move_to(new_group_id)
            .map_err(|_| DatabaseError::GroupNotFound)?;
        Ok(())
    }

    pub(super) fn parse_entry_id(uuid_str: &str) -> Result<EntryId, DatabaseError> {
        Uuid::parse_str(uuid_str)
            .map(EntryId::from_uuid)
            .map_err(|_| DatabaseError::EntryNotFound)
    }
}

// Parse the frontend-supplied expiry string (datetime-local format, optionally
// with seconds). We subtract one hour to compensate for the timezone shift
// keepass-rs applies during write.
fn parse_expiry(s: &str) -> Option<NaiveDateTime> {
    let parsed = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .ok()?;
    Some(parsed - chrono::Duration::hours(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kdbx::types::CustomField;
    use tempfile::TempDir;

    fn fresh_db() -> (TempDir, Database) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("entry_tests.kdbx");
        let db = Database::create(path, "pw".into()).unwrap();
        (dir, db)
    }

    fn make_entry(group_uuid: &str, title: &str) -> EntryData {
        EntryData {
            uuid: String::new(),
            title: title.into(),
            username: "alice".into(),
            password: "s3cr3t".into(),
            url: "https://example.com".into(),
            notes: "n".into(),
            tags: "personal".into(),
            group_uuid: group_uuid.into(),
            icon_id: None,
            is_favorite: false,
            created: None,
            modified: None,
            last_accessed: None,
            expiry_time: None,
            expires: false,
            usage_count: 0,
            custom_fields: Vec::new(),
            history: Vec::new(),
        }
    }

    #[test]
    fn create_entry_persists_all_standard_fields() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_entry(make_entry(&root, "Example")).unwrap();

        let entries = db.get_entries_in_group(&root).unwrap();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.title, "Example");
        assert_eq!(e.username, "alice");
        assert_eq!(e.password, "s3cr3t");
        assert_eq!(e.url, "https://example.com");
        assert_eq!(e.notes, "n");
        assert_eq!(e.tags, "personal");
        assert!(e.created.is_some());
        assert!(e.modified.is_some());
        assert!(!e.is_favorite);
    }

    #[test]
    fn create_entry_with_explicit_uuid_keeps_it() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        let mut e = make_entry(&root, "X");
        e.uuid = "11111111-1111-1111-1111-111111111111".into();
        db.create_entry(e).unwrap();

        let got = db.get_entry("11111111-1111-1111-1111-111111111111").unwrap();
        assert_eq!(got.title, "X");
    }

    #[test]
    fn create_entry_with_bad_group_uuid_fails() {
        let (_dir, mut db) = fresh_db();
        let res = db.create_entry(make_entry("not-a-uuid", "X"));
        assert!(matches!(res, Err(DatabaseError::GroupNotFound)));
    }

    #[test]
    fn create_entry_with_favorite_flag() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        let mut e = make_entry(&root, "Fav");
        e.is_favorite = true;
        db.create_entry(e).unwrap();

        let got = &db.get_entries_in_group(&root).unwrap()[0];
        assert!(got.is_favorite);
    }

    #[test]
    fn create_entry_with_custom_fields_protected_and_unprotected() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        let mut e = make_entry(&root, "X");
        e.custom_fields = vec![
            CustomField {
                name: "Recovery".into(),
                value: "code-123".into(),
                protected: false,
            },
            CustomField {
                name: "PIN".into(),
                value: "9999".into(),
                protected: true,
            },
        ];
        db.create_entry(e).unwrap();

        let got = &db.get_entries_in_group(&root).unwrap()[0];
        let by_name: std::collections::HashMap<_, _> =
            got.custom_fields.iter().map(|f| (f.name.as_str(), f)).collect();
        assert_eq!(by_name["Recovery"].value, "code-123");
        assert!(!by_name["Recovery"].protected);
        assert_eq!(by_name["PIN"].value, "9999");
        assert!(by_name["PIN"].protected);
    }

    #[test]
    fn update_entry_changes_fields() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_entry(make_entry(&root, "Old")).unwrap();
        let uuid = db.get_entries_in_group(&root).unwrap()[0].uuid.clone();

        let mut updated = make_entry(&root, "New");
        updated.uuid = uuid.clone();
        updated.password = "rotated".into();
        db.update_entry(updated).unwrap();

        let got = db.get_entry(&uuid).unwrap();
        assert_eq!(got.title, "New");
        assert_eq!(got.password, "rotated");
    }

    #[test]
    fn update_entry_snapshots_previous_into_history() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_entry(make_entry(&root, "Initial")).unwrap();
        let uuid = db.get_entries_in_group(&root).unwrap()[0].uuid.clone();

        let mut updated = make_entry(&root, "Initial");
        updated.uuid = uuid.clone();
        updated.password = "changed".into();
        db.update_entry(updated).unwrap();

        let got = db.get_entry(&uuid).unwrap();
        assert_eq!(got.history.len(), 1, "history should have one snapshot");
        assert_eq!(got.history[0].password, "s3cr3t", "snapshot has the OLD password");
    }

    #[test]
    fn update_entry_with_no_visible_changes_does_not_grow_history() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_entry(make_entry(&root, "Same")).unwrap();
        let uuid = db.get_entries_in_group(&root).unwrap()[0].uuid.clone();

        let mut same = make_entry(&root, "Same");
        same.uuid = uuid.clone();
        db.update_entry(same).unwrap();

        let got = db.get_entry(&uuid).unwrap();
        assert_eq!(got.history.len(), 0);
    }

    #[test]
    fn update_entry_drops_removed_custom_fields() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        let mut e = make_entry(&root, "X");
        e.custom_fields = vec![CustomField {
            name: "Recovery".into(),
            value: "abc".into(),
            protected: false,
        }];
        db.create_entry(e).unwrap();
        let uuid = db.get_entries_in_group(&root).unwrap()[0].uuid.clone();

        let mut update = make_entry(&root, "X");
        update.uuid = uuid.clone();
        // No custom_fields this time → previous custom field should be gone.
        db.update_entry(update).unwrap();

        let got = db.get_entry(&uuid).unwrap();
        assert!(got.custom_fields.is_empty());
    }

    #[test]
    fn update_entry_clears_favorite_when_unset() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        let mut e = make_entry(&root, "X");
        e.is_favorite = true;
        db.create_entry(e).unwrap();
        let uuid = db.get_entries_in_group(&root).unwrap()[0].uuid.clone();

        let mut unfav = make_entry(&root, "X");
        unfav.uuid = uuid.clone();
        unfav.is_favorite = false;
        db.update_entry(unfav).unwrap();

        assert!(!db.get_entry(&uuid).unwrap().is_favorite);
    }

    #[test]
    fn delete_entry_removes_it() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_entry(make_entry(&root, "X")).unwrap();
        let uuid = db.get_entries_in_group(&root).unwrap()[0].uuid.clone();

        db.delete_entry(&uuid).unwrap();
        assert!(db.get_entry(&uuid).is_err());
    }

    #[test]
    fn move_entry_changes_group() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_group("Sub".into(), Some(root.clone()), None).unwrap();
        let sub_uuid = db.get_root_group().children[0].uuid.clone();

        db.create_entry(make_entry(&root, "X")).unwrap();
        let entry_uuid = db.get_entries_in_group(&root).unwrap()[0].uuid.clone();

        db.move_entry(&entry_uuid, &sub_uuid).unwrap();

        assert!(db.get_entries_in_group(&root).unwrap().is_empty());
        assert_eq!(db.get_entries_in_group(&sub_uuid).unwrap().len(), 1);
        assert_eq!(db.get_entry(&entry_uuid).unwrap().group_uuid, sub_uuid);
    }

    #[test]
    fn move_entry_to_nonexistent_group_fails() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_entry(make_entry(&root, "X")).unwrap();
        let uuid = db.get_entries_in_group(&root).unwrap()[0].uuid.clone();

        let err = db
            .move_entry(&uuid, "00000000-0000-0000-0000-000000000000")
            .unwrap_err();
        assert!(matches!(err, DatabaseError::GroupNotFound));
    }

    #[test]
    fn delete_entry_with_bad_uuid_fails() {
        let (_dir, mut db) = fresh_db();
        let err = db.delete_entry("not-a-uuid").unwrap_err();
        assert!(matches!(err, DatabaseError::EntryNotFound));
    }

    #[test]
    fn expiry_roundtrip_compensates_for_timezone_shift() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        let mut e = make_entry(&root, "Expiring");
        e.expires = true;
        e.expiry_time = Some("2030-06-15T10:30".into());
        db.create_entry(e).unwrap();

        let got = &db.get_entries_in_group(&root).unwrap()[0];
        assert!(got.expires);
        assert_eq!(got.expiry_time.as_deref(), Some("2030-06-15T10:30"));
    }

    #[test]
    fn icon_roundtrip() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        let mut e = make_entry(&root, "X");
        e.icon_id = Some(42);
        db.create_entry(e).unwrap();

        let got = &db.get_entries_in_group(&root).unwrap()[0];
        assert_eq!(got.icon_id, Some(42));
    }

    #[test]
    fn get_all_entries_walks_subtree() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_group("Sub".into(), Some(root.clone()), None).unwrap();
        let sub = db.get_root_group().children[0].uuid.clone();

        db.create_entry(make_entry(&root, "A")).unwrap();
        db.create_entry(make_entry(&sub, "B")).unwrap();

        let all = db.get_all_entries();
        assert_eq!(all.len(), 2);
        let titles: Vec<_> = all.iter().map(|e| e.title.as_str()).collect();
        assert!(titles.contains(&"A"));
        assert!(titles.contains(&"B"));
    }

    #[test]
    fn parse_expiry_accepts_with_and_without_seconds() {
        let a = parse_expiry("2030-01-02T03:04").unwrap();
        let b = parse_expiry("2030-01-02T03:04:00").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn parse_expiry_subtracts_one_hour() {
        let parsed = parse_expiry("2030-01-02T10:00").unwrap();
        // 10:00 input → 09:00 stored
        assert_eq!(parsed.format("%H:%M").to_string(), "09:00");
    }

    #[test]
    fn parse_expiry_rejects_garbage() {
        assert!(parse_expiry("not a date").is_none());
        assert!(parse_expiry("").is_none());
    }
}

