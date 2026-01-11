#[cfg(test)]
mod tests {
    use super::super::Database;
    use tempfile::NamedTempFile;
    use std::path::PathBuf;

    const TEST_PASSWORD: &str = "test_password_123";

    #[test]
    fn test_create_and_open_database() {
        // Create temporary file
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();

        // Create database
        let result = Database::create(db_path.clone(), TEST_PASSWORD.to_string());
        assert!(result.is_ok(), "Failed to create database: {:?}", result.err());
        
        let db = result.unwrap();
        let root = db.get_root_group();
        // Root group name is based on filename
        assert!(!root.name.is_empty(), "Root group should have a name");

        // Save database
        let mut db = db;
        let save_result = db.save();
        assert!(save_result.is_ok(), "Failed to save database: {:?}", save_result.err());

        // Open database with correct password
        let open_result = Database::open(db_path.clone(), TEST_PASSWORD.to_string());
        assert!(open_result.is_ok(), "Failed to open database: {:?}", open_result.err());

        // Try opening with wrong password
        let wrong_password_result = Database::open(db_path, "wrong_password".to_string());
        assert!(wrong_password_result.is_err(), "Should fail with wrong password");
    }

    #[test]
    fn test_create_group() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();

        let mut db = Database::create(db_path, TEST_PASSWORD.to_string()).unwrap();
        let root = db.get_root_group();
        let root_uuid = root.uuid.clone();

        // Create a group
        let result = db.create_group("Test Group".to_string(), Some(root_uuid.clone()), Some(1));
        assert!(result.is_ok(), "Failed to create group: {:?}", result.err());

        // Verify group exists
        let root_after = db.get_root_group();
        assert_eq!(root_after.children.len(), 1, "Group should be created");
        assert_eq!(root_after.children[0].name, "Test Group");
        assert_eq!(root_after.children[0].icon_id, Some(1));
    }

    #[test]
    fn test_rename_group() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();

        let mut db = Database::create(db_path, TEST_PASSWORD.to_string()).unwrap();
        let root = db.get_root_group();
        let root_uuid = root.uuid.clone();

        // Create a group
        db.create_group("Original Name".to_string(), Some(root_uuid.clone()), None).unwrap();
        let group_uuid = db.get_root_group().children[0].uuid.clone();

        // Rename group
        let result = db.rename_group(&group_uuid, "New Name".to_string(), Some(2));
        assert!(result.is_ok(), "Failed to rename group: {:?}", result.err());

        // Verify rename
        let root_after = db.get_root_group();
        assert_eq!(root_after.children[0].name, "New Name");
        assert_eq!(root_after.children[0].icon_id, Some(2));
    }

    #[test]
    fn test_delete_group() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();

        let mut db = Database::create(db_path, TEST_PASSWORD.to_string()).unwrap();
        let root = db.get_root_group();
        let root_uuid = root.uuid.clone();

        // Create a group
        db.create_group("To Delete".to_string(), Some(root_uuid.clone()), None).unwrap();
        let group_uuid = db.get_root_group().children[0].uuid.clone();

        // Delete group
        let result = db.delete_group(&group_uuid);
        assert!(result.is_ok(), "Failed to delete group: {:?}", result.err());

        // Verify deletion
        let root_after = db.get_root_group();
        assert_eq!(root_after.children.len(), 0, "Group should be deleted");
    }

    #[test]
    fn test_delete_root_group_fails() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();

        let mut db = Database::create(db_path, TEST_PASSWORD.to_string()).unwrap();
        let root = db.get_root_group();
        let root_uuid = root.uuid.clone();

        // Try to delete root group
        let result = db.delete_group(&root_uuid);
        assert!(result.is_err(), "Should not be able to delete root group");
    }

    #[test]
    fn test_kdf_info() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();

        let db = Database::create(db_path, TEST_PASSWORD.to_string()).unwrap();
        let kdf_info = db.get_kdf_info();

        // Default should be Argon2id
        assert_eq!(kdf_info.kdf_type, "Argon2id");
        assert!(kdf_info.memory.unwrap_or(0) > 0);
        assert!(kdf_info.iterations.unwrap_or(0) > 0);
        assert!(kdf_info.parallelism.unwrap_or(0) > 0);
    }

    #[test]
    fn test_database_persistence() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();

        // Create and save database with a group
        {
            let mut db = Database::create(db_path.clone(), TEST_PASSWORD.to_string()).unwrap();
            let root = db.get_root_group();
            db.create_group("Persistent Group".to_string(), Some(root.uuid.clone()), None).unwrap();
            db.save().unwrap();
        }

        // Open database and verify group persisted
        {
            let db = Database::open(db_path, TEST_PASSWORD.to_string()).unwrap();
            let root = db.get_root_group();
            assert_eq!(root.children.len(), 1);
            assert_eq!(root.children[0].name, "Persistent Group");
        }
    }
}
