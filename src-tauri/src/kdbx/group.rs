use keepass::db::{CustomDataItem, CustomDataValue, Group, GroupId, GroupRef, Icon};
use uuid::Uuid;

use super::database::Database;
use super::error::DatabaseError;
use super::types::GroupData;

// keepass 0.13.x stores child groups as `HashSet<GroupId>` so iteration order
// is implementation-defined. We persist an explicit ordering by writing an
// integer index into each group's `custom_data` under a namespaced key.
// Older databases (or groups created by KeePass / KeePassXC) won't have the
// key — they sort last, by name, which matches the "newcomers go to the end"
// behavior most users expect.
const SORT_INDEX_KEY: &str = "_spm_sort_index";

fn read_sort_index(group: &Group) -> u32 {
    group
        .custom_data
        .get(SORT_INDEX_KEY)
        .and_then(|item| item.value.as_ref())
        .and_then(|value| match value {
            CustomDataValue::String(s) => s.parse::<u32>().ok(),
            CustomDataValue::Binary(_) => None,
        })
        .unwrap_or(u32::MAX)
}

fn write_sort_index(group: &mut Group, index: u32) {
    group.custom_data.insert(
        SORT_INDEX_KEY.to_string(),
        CustomDataItem {
            value: Some(CustomDataValue::String(index.to_string())),
            last_modification_time: Some(chrono::Utc::now().naive_utc()),
        },
    );
}

impl Database {
    pub fn get_root_group(&self) -> GroupData {
        Self::convert_group(self.db.root(), None)
    }

    pub(super) fn convert_group(group: GroupRef<'_>, parent_uuid: Option<String>) -> GroupData {
        let uuid = group.id().uuid().to_string();

        // Sort children by (persisted index, name) so the UI sees a stable
        // ordering across reads. Groups without a recorded index fall back
        // to u32::MAX and are tie-broken by name.
        let mut sorted_children: Vec<GroupRef<'_>> = group.groups().collect();
        sorted_children.sort_by(|a, b| {
            let ai = read_sort_index(a);
            let bi = read_sort_index(b);
            ai.cmp(&bi).then_with(|| a.name.cmp(&b.name))
        });

        let children: Vec<GroupData> = sorted_children
            .into_iter()
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

        // Find the largest existing sibling sort index so we can append the
        // new group at the end. Anything missing an index (u32::MAX sentinel)
        // is ignored so a single legacy sibling doesn't push the new group
        // into MAX-territory.
        let next_index = self
            .db
            .group(parent_id)
            .map(|p| {
                p.groups()
                    .map(|g| read_sort_index(&g))
                    .filter(|&i| i != u32::MAX)
                    .max()
                    .map(|m| m.saturating_add(1))
                    .unwrap_or(0)
            })
            .unwrap_or(0);

        let mut parent = self
            .db
            .group_mut(parent_id)
            .ok_or(DatabaseError::GroupNotFound)?;

        let mut new_group = parent.add_group();
        new_group.name = name;
        if let Some(id) = icon_id {
            new_group.set_icon_builtin(id as usize);
        }
        write_sort_index(&mut new_group, next_index);
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

    // Persist a new position for `group_uuid` among its siblings by
    // rewriting every sibling's sort index. We always renumber from 0
    // upward so an arbitrarily-old database with sparse / missing indices
    // ends up densely ordered after the first reorder of its parent.
    pub fn reorder_group(
        &mut self,
        group_uuid: &str,
        target_index: usize,
    ) -> Result<(), DatabaseError> {
        let id = Self::parse_group_id(group_uuid)?;

        // Build the desired ordering while holding only an immutable borrow.
        let (parent_id, mut ordered_ids) = {
            let group_ref = self.db.group(id).ok_or(DatabaseError::GroupNotFound)?;
            let parent_ref = group_ref.parent().ok_or(DatabaseError::GroupNotFound)?;
            let parent_id = parent_ref.id();

            let mut siblings: Vec<(u32, String, GroupId)> = parent_ref
                .groups()
                .map(|g| (read_sort_index(&g), g.name.clone(), g.id()))
                .collect();
            // GroupId isn't Ord (just Eq/Hash). Sort by the (index, name)
            // prefix only — that's a total order for our purposes.
            siblings.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
            let ids: Vec<GroupId> = siblings.into_iter().map(|(_, _, gid)| gid).collect();
            (parent_id, ids)
        };

        // Re-position the moved group inside the ordering.
        ordered_ids.retain(|&gid| gid != id);
        let target = target_index.min(ordered_ids.len());
        ordered_ids.insert(target, id);

        // Now flip to mutable mode and write back densely numbered indices.
        for (new_idx, gid) in ordered_ids.into_iter().enumerate() {
            if let Some(mut sibling) = self.db.group_mut(gid) {
                write_sort_index(&mut sibling, new_idx as u32);
            }
        }

        // Silence unused-warning if parent_id ends up not needed in future
        // refactors. The lookup above also validates the group has a parent.
        let _ = parent_id;
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
    fn create_group_appends_new_groups_at_the_end() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_group("First".into(), Some(root.clone()), None).unwrap();
        db.create_group("Second".into(), Some(root.clone()), None).unwrap();
        db.create_group("Third".into(), Some(root), None).unwrap();

        let names: Vec<_> = db
            .get_root_group()
            .children
            .into_iter()
            .map(|g| g.name)
            .collect();
        assert_eq!(names, vec!["First", "Second", "Third"]);
    }

    #[test]
    fn reorder_group_validates_uuid_and_rejects_root() {
        let (_dir, mut db) = fresh_db();
        // Fake uuid → GroupNotFound
        let err = db
            .reorder_group("00000000-0000-0000-0000-000000000000", 0)
            .unwrap_err();
        assert!(matches!(err, DatabaseError::GroupNotFound));

        // Root has no parent so it can't be reordered.
        let root = db.get_root_group().uuid;
        let err = db.reorder_group(&root, 0).unwrap_err();
        assert!(matches!(err, DatabaseError::GroupNotFound));
    }

    #[test]
    fn reorder_group_moves_to_front() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_group("A".into(), Some(root.clone()), None).unwrap();
        db.create_group("B".into(), Some(root.clone()), None).unwrap();
        db.create_group("C".into(), Some(root), None).unwrap();

        let c_uuid = db
            .get_root_group()
            .children
            .iter()
            .find(|g| g.name == "C")
            .unwrap()
            .uuid
            .clone();

        db.reorder_group(&c_uuid, 0).unwrap();

        let names: Vec<_> = db
            .get_root_group()
            .children
            .into_iter()
            .map(|g| g.name)
            .collect();
        assert_eq!(names, vec!["C", "A", "B"]);
    }

    #[test]
    fn reorder_group_moves_to_middle() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_group("A".into(), Some(root.clone()), None).unwrap();
        db.create_group("B".into(), Some(root.clone()), None).unwrap();
        db.create_group("C".into(), Some(root), None).unwrap();

        let a_uuid = db
            .get_root_group()
            .children
            .iter()
            .find(|g| g.name == "A")
            .unwrap()
            .uuid
            .clone();

        // Move A from index 0 to index 1: expect ["B", "A", "C"]
        db.reorder_group(&a_uuid, 1).unwrap();

        let names: Vec<_> = db
            .get_root_group()
            .children
            .into_iter()
            .map(|g| g.name)
            .collect();
        assert_eq!(names, vec!["B", "A", "C"]);
    }

    #[test]
    fn reorder_group_clamps_oversized_index_to_end() {
        let (_dir, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_group("A".into(), Some(root.clone()), None).unwrap();
        db.create_group("B".into(), Some(root), None).unwrap();
        let a_uuid = db
            .get_root_group()
            .children
            .iter()
            .find(|g| g.name == "A")
            .unwrap()
            .uuid
            .clone();

        db.reorder_group(&a_uuid, 999).unwrap();
        let names: Vec<_> = db
            .get_root_group()
            .children
            .into_iter()
            .map(|g| g.name)
            .collect();
        assert_eq!(names, vec!["B", "A"]);
    }

    #[test]
    fn reorder_persists_after_save_and_reopen() {
        // Make sure the sort_index survives a KDBX roundtrip.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("reorder.kdbx");
        let mut db = Database::create(path.clone(), "pw".into()).unwrap();
        let root = db.get_root_group().uuid;

        db.create_group("A".into(), Some(root.clone()), None).unwrap();
        db.create_group("B".into(), Some(root.clone()), None).unwrap();
        let b_uuid = db
            .get_root_group()
            .children
            .iter()
            .find(|g| g.name == "B")
            .unwrap()
            .uuid
            .clone();
        db.reorder_group(&b_uuid, 0).unwrap();
        db.save().unwrap();
        drop(db);

        let reopened = Database::open(path, "pw".into()).unwrap();
        let names: Vec<_> = reopened
            .get_root_group()
            .children
            .into_iter()
            .map(|g| g.name)
            .collect();
        assert_eq!(names, vec!["B", "A"]);
    }
}
