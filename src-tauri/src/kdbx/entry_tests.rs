#[cfg(test)]
mod tests {
    use super::super::{Database, EntryData};
    use tempfile::NamedTempFile;

    const TEST_PASSWORD: &str = "test_password_123";

    fn create_test_db() -> (Database, String) {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();
        let db = Database::create(db_path, TEST_PASSWORD.to_string()).unwrap();
        let root_uuid = db.get_root_group().uuid.clone();
        (db, root_uuid)
    }

    fn create_test_entry(group_uuid: &str) -> EntryData {
        EntryData {
            uuid: uuid::Uuid::new_v4().to_string(),
            title: "Test Entry".to_string(),
            username: "test_user".to_string(),
            password: "test_pass_123".to_string(),
            url: "https://example.com".to_string(),
            notes: "Test notes".to_string(),
            tags: String::new(),
            group_uuid: group_uuid.to_string(),
            is_favorite: false,
            icon_id: Some(0),
            created: None,
            modified: None,
            last_accessed: None,
            expires: false,
            expiry_time: None,
            usage_count: 0,
            custom_fields: vec![],
            history: vec![],
        }
    }

    #[test]
    fn test_create_entry() {
        let (mut db, root_uuid) = create_test_db();
        let entry = create_test_entry(&root_uuid);
        let entry_uuid = entry.uuid.clone();

        let result = db.create_entry(entry);
        assert!(result.is_ok(), "Failed to create entry: {:?}", result.err());

        // Verify entry exists
        let entries = db.get_entries_in_group(&root_uuid).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uuid, entry_uuid);
        assert_eq!(entries[0].title, "Test Entry");
    }

    #[test]
    fn test_get_entry() {
        let (mut db, root_uuid) = create_test_db();
        let entry = create_test_entry(&root_uuid);
        let entry_uuid = entry.uuid.clone();

        db.create_entry(entry).unwrap();

        // Get specific entry
        let result = db.get_entry(&entry_uuid);
        assert!(result.is_ok(), "Failed to get entry: {:?}", result.err());

        let retrieved = result.unwrap();
        assert_eq!(retrieved.uuid, entry_uuid);
        assert_eq!(retrieved.title, "Test Entry");
        assert_eq!(retrieved.username, "test_user");
    }

    #[test]
    fn test_update_entry() {
        let (mut db, root_uuid) = create_test_db();
        let entry = create_test_entry(&root_uuid);
        let entry_uuid = entry.uuid.clone();

        db.create_entry(entry).unwrap();

        // Update entry
        let mut updated_entry = db.get_entry(&entry_uuid).unwrap();
        updated_entry.title = "Updated Title".to_string();
        updated_entry.password = "new_password_456".to_string();
        updated_entry.is_favorite = true;

        let result = db.update_entry(updated_entry);
        assert!(result.is_ok(), "Failed to update entry: {:?}", result.err());

        // Verify updates
        let retrieved = db.get_entry(&entry_uuid).unwrap();
        assert_eq!(retrieved.title, "Updated Title");
        assert_eq!(retrieved.password, "new_password_456");
        assert_eq!(retrieved.is_favorite, true);
    }

    #[test]
    fn test_delete_entry() {
        let (mut db, root_uuid) = create_test_db();
        let entry = create_test_entry(&root_uuid);
        let entry_uuid = entry.uuid.clone();

        db.create_entry(entry).unwrap();

        // Delete entry
        let result = db.delete_entry(&entry_uuid);
        assert!(result.is_ok(), "Failed to delete entry: {:?}", result.err());

        // Verify deletion
        let entries = db.get_entries_in_group(&root_uuid).unwrap();
        assert_eq!(entries.len(), 0);

        // Try to get deleted entry
        let get_result = db.get_entry(&entry_uuid);
        assert!(get_result.is_err(), "Should not find deleted entry");
    }

    #[test]
    fn test_move_entry() {
        let (mut db, root_uuid) = create_test_db();

        // Create second group
        db.create_group("Target Group".to_string(), Some(root_uuid.clone()), None).unwrap();
        let target_uuid = db.get_root_group().children[0].uuid.clone();

        // Create entry in root
        let entry = create_test_entry(&root_uuid);
        let entry_uuid = entry.uuid.clone();
        db.create_entry(entry).unwrap();

        // Move entry to target group
        let result = db.move_entry(&entry_uuid, &target_uuid);
        assert!(result.is_ok(), "Failed to move entry: {:?}", result.err());

        // Verify entry moved
        let root_entries = db.get_entries_in_group(&root_uuid).unwrap();
        assert_eq!(root_entries.len(), 0, "Entry should be removed from root");

        let target_entries = db.get_entries_in_group(&target_uuid).unwrap();
        assert_eq!(target_entries.len(), 1, "Entry should be in target group");
        assert_eq!(target_entries[0].uuid, entry_uuid);
    }

    #[test]
    fn test_favorite_entries() {
        let (mut db, root_uuid) = create_test_db();

        // Create multiple entries
        let mut entry1 = create_test_entry(&root_uuid);
        entry1.title = "Favorite 1".to_string();
        entry1.is_favorite = true;
        db.create_entry(entry1).unwrap();

        let mut entry2 = create_test_entry(&root_uuid);
        entry2.title = "Not Favorite".to_string();
        entry2.is_favorite = false;
        db.create_entry(entry2).unwrap();

        let mut entry3 = create_test_entry(&root_uuid);
        entry3.title = "Favorite 2".to_string();
        entry3.is_favorite = true;
        db.create_entry(entry3).unwrap();

        // Get all entries
        let all_entries = db.get_all_entries();
        assert_eq!(all_entries.len(), 3);

        // Filter favorites
        let favorites: Vec<_> = all_entries.iter().filter(|e| e.is_favorite).collect();
        assert_eq!(favorites.len(), 2);
        assert!(favorites.iter().any(|e| e.title == "Favorite 1"));
        assert!(favorites.iter().any(|e| e.title == "Favorite 2"));
    }

    #[test]
    fn test_search_entries() {
        let (mut db, root_uuid) = create_test_db();

        // Create entries with different content
        let mut entry1 = create_test_entry(&root_uuid);
        entry1.title = "GitHub Account".to_string();
        entry1.username = "john@example.com".to_string();
        db.create_entry(entry1).unwrap();

        let mut entry2 = create_test_entry(&root_uuid);
        entry2.title = "Gmail Account".to_string();
        entry2.username = "jane@gmail.com".to_string();
        db.create_entry(entry2).unwrap();

        let mut entry3 = create_test_entry(&root_uuid);
        entry3.title = "Bank Login".to_string();
        entry3.username = "user123".to_string();
        db.create_entry(entry3).unwrap();

        // Search for "git"
        let results = db.search_entries("git");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "GitHub Account");

        // Search for "gmail"
        let results = db.search_entries("gmail");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Gmail Account");

        // Search for "@" (should find 2 entries)
        let results = db.search_entries("@");
        assert_eq!(results.len(), 2);

        // Case insensitive search
        let results = db.search_entries("GITHUB");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_entry_with_custom_fields() {
        let (mut db, root_uuid) = create_test_db();

        let mut entry = create_test_entry(&root_uuid);
        use crate::kdbx::types::CustomField;
        entry.custom_fields = vec![
            CustomField {
                name: "API Key".to_string(),
                value: "abc123xyz".to_string(),
                protected: true,
            },
            CustomField {
                name: "Recovery Email".to_string(),
                value: "backup@example.com".to_string(),
                protected: false,
            },
        ];

        db.create_entry(entry.clone()).unwrap();

        // Retrieve and verify custom fields
        let retrieved = db.get_entry(&entry.uuid).unwrap();
        assert_eq!(retrieved.custom_fields.len(), 2);
        assert_eq!(retrieved.custom_fields[0].name, "API Key");
        assert_eq!(retrieved.custom_fields[0].value, "abc123xyz");
        assert_eq!(retrieved.custom_fields[0].protected, true);
    }
}
