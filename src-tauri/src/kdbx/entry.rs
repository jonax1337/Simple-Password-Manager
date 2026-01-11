use chrono::NaiveDateTime;
use keepass::{db::{Entry, Group, Node, Value, Times, History}, Database as KeepassDatabase};

#[cfg(test)]
#[path = "entry_tests.rs"]
mod entry_tests;

use uuid::Uuid;

use super::database::Database;
use super::error::DatabaseError;
use super::types::{CustomField, EntryData, HistoryEntry};

impl Database {
    pub fn get_entries_in_group(&self, group_uuid: &str) -> Result<Vec<EntryData>, DatabaseError> {
        let group = self.find_group_by_uuid(group_uuid)?;
        
        let entries: Vec<EntryData> = group
            .children
            .iter()
            .filter_map(|node| match node {
                Node::Entry(e) => Some(self.convert_entry(e, group_uuid)),
                _ => None,
            })
            .collect();

        Ok(entries)
    }

    pub fn get_all_entries(&self) -> Vec<EntryData> {
        let mut entries = Vec::new();
        self.collect_entries(&self.db.root, &mut entries);
        entries
    }

    pub(super) fn collect_entries(&self, group: &Group, entries: &mut Vec<EntryData>) {
        let group_uuid = group.uuid.to_string();
        
        for node in &group.children {
            match node {
                Node::Entry(e) => entries.push(self.convert_entry(e, &group_uuid)),
                Node::Group(g) => self.collect_entries(g, entries),
            }
        }
    }

    pub(super) fn convert_entry(&self, entry: &Entry, group_uuid: &str) -> EntryData {
        let uuid = entry.uuid.to_string();
        let is_favorite = entry.get("_Favorite").unwrap_or("") == "true";
        
        // Extract timestamps and format them
        let created = entry.times.get_creation().map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string());
        let modified = entry.times.get_last_modification().map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string());
        let last_accessed = entry.times.get_last_access().map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string());
        // Format expiry without seconds for datetime-local compatibility
        // Keep as UTC - browser will handle local timezone conversion
        let expiry_time = entry.times.get_expiry().map(|t| {
            t.format("%Y-%m-%dT%H:%M").to_string()
        });
        
        // Standard fields to exclude from custom fields
        let standard_fields = ["Title", "UserName", "Password", "URL", "Notes", "Tags", "_Favorite"];
        
        // Extract custom fields
        let custom_fields: Vec<CustomField> = entry.fields.iter()
            .filter(|(key, _)| !standard_fields.contains(&key.as_str()))
            .map(|(key, value)| {
                let (val, protected) = match value {
                    Value::Unprotected(s) => (s.clone(), false),
                    Value::Protected(s) => (String::from_utf8_lossy(s.unsecure()).to_string(), true),
                    Value::Bytes(b) => (String::from_utf8_lossy(b).to_string(), false),
                };
                CustomField {
                    name: key.clone(),
                    value: val,
                    protected,
                }
            })
            .collect();
        
        // Extract password history
        let history: Vec<HistoryEntry> = if let Some(hist) = &entry.history {
            hist.get_entries().iter()
                .map(|h| {
                    let timestamp = h.times.get_last_modification()
                        .map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string())
                        .unwrap_or_else(|| "".to_string());
                    
                    HistoryEntry {
                        timestamp,
                        title: h.get_title().unwrap_or("").to_string(),
                        username: h.get_username().unwrap_or("").to_string(),
                        password: h.get_password().unwrap_or("").to_string(),
                        url: h.get("URL").unwrap_or("").to_string(),
                        notes: h.get("Notes").unwrap_or("").to_string(),
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        
        EntryData {
            uuid,
            title: entry.get_title().unwrap_or("").to_string(),
            username: entry.get_username().unwrap_or("").to_string(),
            password: entry.get_password().unwrap_or("").to_string(),
            url: entry.get("URL").unwrap_or("").to_string(),
            notes: entry.get("Notes").unwrap_or("").to_string(),
            tags: entry.get("Tags").unwrap_or("").to_string(),
            group_uuid: group_uuid.to_string(),
            icon_id: entry.icon_id,
            is_favorite,
            created,
            modified,
            last_accessed,
            expiry_time,
            expires: entry.times.expires,
            usage_count: entry.times.usage_count,
            custom_fields,
            history,
        }
    }

    pub fn get_entry(&self, entry_uuid: &str) -> Result<EntryData, DatabaseError> {
        let entry = self.find_entry_by_uuid(entry_uuid)?;
        let group_uuid = self.find_entry_group_uuid(entry_uuid)?;
        Ok(self.convert_entry(entry, &group_uuid))
    }

    pub fn create_entry(&mut self, entry_data: EntryData) -> Result<(), DatabaseError> {
        let group = self.find_group_by_uuid_mut(&entry_data.group_uuid)?;
        
        // Use provided UUID or generate a new one
        let uuid = if entry_data.uuid.is_empty() {
            Uuid::new_v4()
        } else {
            Uuid::parse_str(&entry_data.uuid).map_err(|_| DatabaseError::InvalidUuid)?
        };
        
        let mut entry = Entry {
            uuid,
            history: None,
            ..Default::default()
        };
        
        // Set creation and modification timestamps
        let now = Times::now();
        entry.times.set_creation(now);
        entry.times.set_last_modification(now);
        entry.times.set_last_access(now);
        
        entry.fields.insert("Title".to_string(), Value::Unprotected(entry_data.title));
        entry.fields.insert("UserName".to_string(), Value::Unprotected(entry_data.username));
        entry.fields.insert("Password".to_string(), Value::Protected(entry_data.password.into()));
        if !entry_data.url.is_empty() {
            entry.fields.insert("URL".to_string(), Value::Unprotected(entry_data.url));
        }
        if !entry_data.notes.is_empty() {
            entry.fields.insert("Notes".to_string(), Value::Unprotected(entry_data.notes));
        }
        if !entry_data.tags.is_empty() {
            entry.fields.insert("Tags".to_string(), Value::Unprotected(entry_data.tags));
        }
        if entry_data.is_favorite {
            entry.fields.insert("_Favorite".to_string(), Value::Unprotected("true".to_string()));
        }
        
        // Add custom fields
        for field in entry_data.custom_fields {
            let value = if field.protected {
                Value::Protected(field.value.into())
            } else {
                Value::Unprotected(field.value)
            };
            entry.fields.insert(field.name, value);
        }
        
        // Set icon ID if provided
        if let Some(icon_id) = entry_data.icon_id {
            entry.icon_id = Some(icon_id);
        }
        
        // Set expiry settings
        entry.times.expires = entry_data.expires;
        if entry_data.expires {
            if let Some(expiry_str) = entry_data.expiry_time {
                if let Ok(expiry) = NaiveDateTime::parse_from_str(&expiry_str, "%Y-%m-%dT%H:%M") {
                    // Store as UTC - frontend sends local time as UTC
                    entry.times.set_expiry(expiry);
                } else if let Ok(expiry) = NaiveDateTime::parse_from_str(&expiry_str, "%Y-%m-%dT%H:%M:%S") {
                    // Store as UTC - frontend sends local time as UTC
                    entry.times.set_expiry(expiry);
                }
            }
        }
        
        group.add_child(entry);
        Ok(())
    }

    pub fn update_entry(&mut self, entry_data: EntryData) -> Result<(), DatabaseError> {
        let entry = self.find_entry_by_uuid_mut(&entry_data.uuid)?;
        
        // Check if any field has changed
        let title_changed = entry.get_title().unwrap_or("") != entry_data.title;
        let username_changed = entry.get_username().unwrap_or("") != entry_data.username;
        let password_changed = entry.get_password().unwrap_or("") != entry_data.password;
        let url_changed = entry.get("URL").unwrap_or("") != entry_data.url;
        let notes_changed = entry.get("Notes").unwrap_or("") != entry_data.notes;
        let tags_changed = entry.get("Tags").unwrap_or("") != entry_data.tags;
        
        let any_change = title_changed || username_changed || password_changed || 
                        url_changed || notes_changed || tags_changed;
        
        if any_change {
            // Clone the current entry state before updating
            let mut history_entry = entry.clone();
            // Remove history from the history entry to avoid nested history
            history_entry.history = None;
            // Set the history entry's modification time to the current entry's last modification time
            if let Some(last_mod) = entry.times.get_last_modification() {
                history_entry.times.set_last_modification(*last_mod);
            }
            // Add to history - initialize History if it doesn't exist
            if let Some(ref mut hist) = entry.history {
                hist.add_entry(history_entry);
            } else {
                let mut new_history = History::default();
                new_history.add_entry(history_entry);
                entry.history = Some(new_history);
            }
        }
        
        // Update modification and access timestamps
        let now = Times::now();
        entry.times.set_last_modification(now);
        entry.times.set_last_access(now);
        
        // Standard fields to keep
        let standard_fields = ["Title", "UserName", "Password", "URL", "Notes", "Tags", "_Favorite"];
        
        // Remove old custom fields (keep only standard fields)
        entry.fields.retain(|key, _| standard_fields.contains(&key.as_str()));
        
        entry.fields.insert("Title".to_string(), Value::Unprotected(entry_data.title));
        entry.fields.insert("UserName".to_string(), Value::Unprotected(entry_data.username));
        entry.fields.insert("Password".to_string(), Value::Protected(entry_data.password.into()));
        entry.fields.insert("URL".to_string(), Value::Unprotected(entry_data.url));
        entry.fields.insert("Notes".to_string(), Value::Unprotected(entry_data.notes));
        entry.fields.insert("Tags".to_string(), Value::Unprotected(entry_data.tags));
        
        // Update favorite status
        if entry_data.is_favorite {
            entry.fields.insert("_Favorite".to_string(), Value::Unprotected("true".to_string()));
        } else {
            entry.fields.remove("_Favorite");
        }
        
        // Add custom fields
        for field in entry_data.custom_fields {
            let value = if field.protected {
                Value::Protected(field.value.into())
            } else {
                Value::Unprotected(field.value)
            };
            entry.fields.insert(field.name, value);
        }
        
        // Update icon ID
        entry.icon_id = entry_data.icon_id;
        
        // Update expiry settings
        entry.times.expires = entry_data.expires;
        if entry_data.expires {
            // If expires is checked, we need to set an expiry time
            if let Some(expiry_str) = entry_data.expiry_time {
                if !expiry_str.is_empty() {
                    if let Ok(expiry) = NaiveDateTime::parse_from_str(&expiry_str, "%Y-%m-%dT%H:%M") {
                        // Store as UTC - frontend sends local time as UTC
                        entry.times.set_expiry(expiry);
                    } else if let Ok(expiry) = NaiveDateTime::parse_from_str(&expiry_str, "%Y-%m-%dT%H:%M:%S") {
                        // Store as UTC - frontend sends local time as UTC
                        entry.times.set_expiry(expiry);
                    }
                }
            }
        }
        
        Ok(())
    }

    pub fn delete_entry(&mut self, entry_uuid: &str) -> Result<(), DatabaseError> {
        let group_uuid = self.find_entry_group_uuid(entry_uuid)?;
        let group = self.find_group_by_uuid_mut(&group_uuid)?;
        
        let uuid = Uuid::parse_str(entry_uuid).map_err(|_| DatabaseError::EntryNotFound)?;
        group.children.retain(|node| {
            if let Node::Entry(e) = node {
                e.uuid != uuid
            } else {
                true
            }
        });
        
        Ok(())
    }

    pub fn move_entry(&mut self, entry_uuid: &str, new_group_uuid: &str) -> Result<(), DatabaseError> {
        // Find the current group containing the entry
        let current_group_uuid = self.find_entry_group_uuid(entry_uuid)?;
        
        // If already in the target group, do nothing
        if current_group_uuid == new_group_uuid {
            return Ok(());
        }
        
        // Verify target group exists
        let _ = self.find_group_by_uuid(new_group_uuid)?;
        
        let uuid = Uuid::parse_str(entry_uuid).map_err(|_| DatabaseError::EntryNotFound)?;
        
        // Remove entry from current group and obtain it
        let entry_to_move: Entry = {
            let current_group = self.find_group_by_uuid_mut(&current_group_uuid)?;

            // Find the position of the entry within the current group's children
            let index = current_group
                .children
                .iter()
                .position(|node| matches!(node, Node::Entry(e) if e.uuid == uuid))
                .ok_or(DatabaseError::EntryNotFound)?;

            // Remove the node at the found index and extract the entry
            match current_group.children.remove(index) {
                Node::Entry(e) => e,
                _ => unreachable!("Non-entry node found at entry position"),
            }
        };
        
        // Add entry to new group
        let new_group = self.find_group_by_uuid_mut(new_group_uuid)?;
        new_group.add_child(entry_to_move);
        Ok(())
    }

    pub(super) fn find_entry_by_uuid(&self, uuid: &str) -> Result<&Entry, DatabaseError> {
        let uuid_parsed = Uuid::parse_str(uuid).map_err(|_| DatabaseError::EntryNotFound)?;
        self.find_entry_recursive(&self.db.root, &uuid_parsed)
            .ok_or(DatabaseError::EntryNotFound)
    }

    pub(super) fn find_entry_by_uuid_mut(&mut self, uuid: &str) -> Result<&mut Entry, DatabaseError> {
        let uuid_parsed = Uuid::parse_str(uuid).map_err(|_| DatabaseError::EntryNotFound)?;
        let root_ptr = &mut self.db.root as *mut Group;
        unsafe { Self::find_entry_recursive_mut_static(&mut *root_ptr, &uuid_parsed) }
            .ok_or(DatabaseError::EntryNotFound)
    }

    fn find_entry_recursive<'a>(&'a self, group: &'a Group, uuid: &Uuid) -> Option<&'a Entry> {
        for node in &group.children {
            match node {
                Node::Entry(e) if &e.uuid == uuid => return Some(e),
                Node::Group(g) => {
                    if let Some(found) = self.find_entry_recursive(g, uuid) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn find_entry_recursive_mut_static<'a>(
        group: &'a mut Group,
        uuid: &Uuid,
    ) -> Option<&'a mut Entry> {
        for node in &mut group.children {
            match node {
                Node::Entry(e) if &e.uuid == uuid => return Some(e),
                Node::Group(g) => {
                    if let Some(found) = Self::find_entry_recursive_mut_static(g, uuid) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub(super) fn find_entry_group_uuid(&self, entry_uuid: &str) -> Result<String, DatabaseError> {
        let uuid_parsed = Uuid::parse_str(entry_uuid).map_err(|_| DatabaseError::EntryNotFound)?;
        self.find_entry_group_uuid_recursive(&self.db.root, &uuid_parsed)
            .ok_or(DatabaseError::EntryNotFound)
    }

    fn find_entry_group_uuid_recursive(&self, group: &Group, uuid: &Uuid) -> Option<String> {
        for node in &group.children {
            match node {
                Node::Entry(e) if &e.uuid == uuid => return Some(group.uuid.to_string()),
                Node::Group(g) => {
                    if let Some(found) = self.find_entry_group_uuid_recursive(g, uuid) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }
}
