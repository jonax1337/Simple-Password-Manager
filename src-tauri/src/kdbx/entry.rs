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

