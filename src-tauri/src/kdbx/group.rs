use keepass::db::{GroupId, GroupRef, Icon};
use uuid::Uuid;

use super::database::Database;
use super::error::DatabaseError;
use super::types::GroupData;

impl Database {
    pub fn get_root_group(&self) -> GroupData {
        Self::convert_group(self.db.root(), None)
    }

    pub(super) fn convert_group(group: GroupRef<'_>, parent_uuid: Option<String>) -> GroupData {
        let uuid = group.id().uuid().to_string();

        let children: Vec<GroupData> = group
            .groups()
            .map(|child| Self::convert_group(child, Some(uuid.clone())))
            .collect();

        let icon_id = match group.icon() {
            Some(Icon::BuiltIn(id)) => Some(*id),
            _ => None,
        };

        GroupData {
            uuid,
            name: group.name.clone(),
            parent_uuid,
            children,
            icon_id,
        }
    }

    pub fn create_group(
        &mut self,
        name: String,
        parent_uuid: Option<String>,
        icon_id: Option<u32>,
    ) -> Result<(), DatabaseError> {
        let parent_id = match parent_uuid {
            Some(uuid) => Self::parse_group_id(&uuid)?,
            None => self.db.root().id(),
        };

        let mut parent = self
            .db
            .group_mut(parent_id)
            .ok_or(DatabaseError::GroupNotFound)?;

        let mut new_group = parent.add_group();
        new_group.name = name;
        if let Some(id) = icon_id {
            new_group.set_icon_builtin(id as usize);
        }
        Ok(())
    }

    pub fn rename_group(
        &mut self,
        group_uuid: &str,
        new_name: String,
        icon_id: Option<u32>,
    ) -> Result<(), DatabaseError> {
        let id = Self::parse_group_id(group_uuid)?;
        let mut group = self
            .db
            .group_mut(id)
            .ok_or(DatabaseError::GroupNotFound)?;

        group.name = new_name;
        if let Some(icon) = icon_id {
            group.set_icon_builtin(icon as usize);
        }
        Ok(())
    }

    pub fn move_group(
        &mut self,
        group_uuid: &str,
        new_parent_uuid: &str,
    ) -> Result<(), DatabaseError> {
        if group_uuid == new_parent_uuid {
            return Err(DatabaseError::GroupNotFound);
        }

        let group_id = Self::parse_group_id(group_uuid)?;
        let new_parent_id = Self::parse_group_id(new_parent_uuid)?;

        if group_id == self.db.root().id() {
            return Err(DatabaseError::GroupNotFound);
        }

        let mut group = self
            .db
            .group_mut(group_id)
            .ok_or(DatabaseError::GroupNotFound)?;

        group
            .move_to(new_parent_id)
            .map_err(|_| DatabaseError::GroupNotFound)
    }

    // Group order is not preserved in keepass 0.13.x (children are stored as
    // HashSet<GroupId>), so explicit reordering is a no-op. Kept for API
    // compatibility with the frontend until a custom ordering mechanism is in.
    pub fn reorder_group(
        &mut self,
        group_uuid: &str,
        _target_index: usize,
    ) -> Result<(), DatabaseError> {
        let id = Self::parse_group_id(group_uuid)?;
        if self.db.group(id).is_none() {
            return Err(DatabaseError::GroupNotFound);
        }
        Ok(())
    }

    pub fn delete_group(&mut self, group_uuid: &str) -> Result<(), DatabaseError> {
        let id = Self::parse_group_id(group_uuid)?;
        if id == self.db.root().id() {
            return Err(DatabaseError::GroupNotFound);
        }

        let group = self
            .db
            .group_mut(id)
            .ok_or(DatabaseError::GroupNotFound)?;
        group.remove();
        Ok(())
    }

    pub(super) fn parse_group_id(uuid_str: &str) -> Result<GroupId, DatabaseError> {
        Uuid::parse_str(uuid_str)
            .map(GroupId::from_uuid)
            .map_err(|_| DatabaseError::GroupNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fresh_db() -> (TempDir, Database) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("group_tests.kdbx");
        let db = Database::create(path, "pw".into()).unwrap();
        (dir, db)
    }

    #[test]
    fn root_group_has_no_parent() {
        let (_dir, db) = fresh_db();
        let root = db.get_root_group();
        assert!(root.parent_uuid.is_none());
        assert!(root.children.is_empty());
    }

    #[test]
    fn create_group_under_root() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_group("Work".into(), Some(root.clone()), Some(7)).unwrap();

        let root_after = db.get_root_group();
        assert_eq!(root_after.children.len(), 1);
        assert_eq!(root_after.children[0].name, "Work");
        assert_eq!(root_after.children[0].parent_uuid.as_deref(), Some(root.as_str()));
        assert_eq!(root_after.children[0].icon_id, Some(7));
    }

    #[test]
    fn create_group_with_no_parent_defaults_to_root() {
        let (_dir, mut db) = fresh_db();
        db.create_group("Floating".into(), None, None).unwrap();
        let children = db.get_root_group().children;
        assert!(children.iter().any(|g| g.name == "Floating"));
    }

    #[test]
    fn create_group_under_nonexistent_parent_fails() {
        let (_dir, mut db) = fresh_db();
        let err = db
            .create_group(
                "Orphan".into(),
                Some("00000000-0000-0000-0000-000000000000".into()),
                None,
            )
            .unwrap_err();
        assert!(matches!(err, DatabaseError::GroupNotFound));
    }

    #[test]
    fn rename_group_updates_name_and_icon() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_group("Before".into(), Some(root.clone()), None).unwrap();
        let g_uuid = db.get_root_group().children[0].uuid.clone();

        db.rename_group(&g_uuid, "After".into(), Some(42)).unwrap();

        let g = &db.get_root_group().children[0];
        assert_eq!(g.name, "After");
        assert_eq!(g.icon_id, Some(42));
    }

    #[test]
    fn rename_group_with_bad_uuid_fails() {
        let (_dir, mut db) = fresh_db();
        let err = db.rename_group("not-uuid", "x".into(), None).unwrap_err();
        assert!(matches!(err, DatabaseError::GroupNotFound));
    }

    #[test]
    fn move_group_reparents() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_group("A".into(), Some(root.clone()), None).unwrap();
        db.create_group("B".into(), Some(root.clone()), None).unwrap();
        let children = db.get_root_group().children;
        let a = children.iter().find(|g| g.name == "A").unwrap().uuid.clone();
        let b = children.iter().find(|g| g.name == "B").unwrap().uuid.clone();

        db.move_group(&a, &b).unwrap();

        let root_after = db.get_root_group();
        assert_eq!(root_after.children.len(), 1);
        let b_after = &root_after.children[0];
        assert_eq!(b_after.children.len(), 1);
        assert_eq!(b_after.children[0].uuid, a);
    }

    #[test]
    fn move_group_to_itself_is_rejected() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_group("A".into(), Some(root), None).unwrap();
        let a = db.get_root_group().children[0].uuid.clone();

        let err = db.move_group(&a, &a).unwrap_err();
        assert!(matches!(err, DatabaseError::GroupNotFound));
    }

    #[test]
    fn move_root_group_is_rejected() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_group("A".into(), Some(root.clone()), None).unwrap();
        let a = db.get_root_group().children[0].uuid.clone();

        let err = db.move_group(&root, &a).unwrap_err();
        assert!(matches!(err, DatabaseError::GroupNotFound));
    }

    #[test]
    fn delete_group_removes_it() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_group("Doomed".into(), Some(root), None).unwrap();
        let g = db.get_root_group().children[0].uuid.clone();

        db.delete_group(&g).unwrap();
        assert!(db.get_root_group().children.is_empty());
    }

    #[test]
    fn delete_root_group_is_rejected() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        let err = db.delete_group(&root).unwrap_err();
        assert!(matches!(err, DatabaseError::GroupNotFound));
    }

    #[test]
    fn reorder_group_is_currently_a_noop_but_validates_uuid() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_group("A".into(), Some(root), None).unwrap();
        let a = db.get_root_group().children[0].uuid.clone();

        // Real group → ok (no-op)
        assert!(db.reorder_group(&a, 0).is_ok());

        // Fake uuid → GroupNotFound
        let err = db
            .reorder_group("00000000-0000-0000-0000-000000000000", 0)
            .unwrap_err();
        assert!(matches!(err, DatabaseError::GroupNotFound));
    }
}
