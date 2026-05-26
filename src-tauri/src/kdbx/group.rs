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
