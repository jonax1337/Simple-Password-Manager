use super::database::Database;
use super::types::EntryData;

impl Database {
    pub fn search_entries(&self, query: &str) -> Vec<EntryData> {
        let query_lower = query.to_lowercase();
        self.get_all_entries()
            .into_iter()
            .filter(|entry| matches_query(entry, &query_lower))
            .collect()
    }

    pub fn search_entries_in_group(&self, query: &str, group_uuid: &str) -> Vec<EntryData> {
        let query_lower = query.to_lowercase();

        let Ok(group_id) = Self::parse_group_id(group_uuid) else {
            return Vec::new();
        };
        let Some(_) = self.db.group(group_id) else {
            return Vec::new();
        };

        collect_descendant_entries(self, group_id)
            .into_iter()
            .filter(|entry| matches_query(entry, &query_lower))
            .collect()
    }
}

fn matches_query(entry: &EntryData, query_lower: &str) -> bool {
    entry.title.to_lowercase().contains(query_lower)
        || entry.username.to_lowercase().contains(query_lower)
        || entry.url.to_lowercase().contains(query_lower)
        || entry.notes.to_lowercase().contains(query_lower)
        || entry.tags.to_lowercase().contains(query_lower)
}

// Walk the group subtree rooted at `start_id`, converting every entry as we go.
// Children are looked up through the database's flat HashMap, so this stays
// O(n) in the subtree size.
fn collect_descendant_entries(
    db: &Database,
    start_id: keepass::db::GroupId,
) -> Vec<EntryData> {
    let mut out = Vec::new();
    let mut stack = vec![start_id];
    while let Some(group_id) = stack.pop() {
        let Some(group) = db.db.group(group_id) else {
            continue;
        };
        let group_uuid = group_id.uuid().to_string();
        for entry in group.entries() {
            out.push(Database::convert_entry(entry, &group_uuid));
        }
        stack.extend(group.group_ids());
    }
    out
}
