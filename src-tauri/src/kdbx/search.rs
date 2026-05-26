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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn populated_db() -> (TempDir, Database, String, String) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("search.kdbx");
        let mut db = Database::create(path, "pw".into()).unwrap();
        let root = db.get_root_group().uuid;

        db.create_group("Sub".into(), Some(root.clone()), None).unwrap();
        let sub = db.get_root_group().children[0].uuid.clone();

        let mut a = blank_entry(&root);
        a.title = "GitHub".into();
        a.username = "alice".into();
        a.url = "https://github.com".into();
        a.notes = "main account".into();
        a.tags = "dev".into();
        db.create_entry(a).unwrap();

        let mut b = blank_entry(&sub);
        b.title = "Bank".into();
        b.username = "bob".into();
        b.url = "https://bank.example".into();
        b.notes = "savings".into();
        b.tags = "finance".into();
        db.create_entry(b).unwrap();

        (dir, db, root, sub)
    }

    fn blank_entry(group_uuid: &str) -> EntryData {
        EntryData {
            uuid: String::new(),
            title: String::new(),
            username: String::new(),
            password: "x".into(),
            url: String::new(),
            notes: String::new(),
            tags: String::new(),
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
    fn search_matches_title() {
        let (_d, db, _, _) = populated_db();
        let hits = db.search_entries("GitHub");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "GitHub");
    }

    #[test]
    fn search_is_case_insensitive() {
        let (_d, db, _, _) = populated_db();
        assert_eq!(db.search_entries("github").len(), 1);
        assert_eq!(db.search_entries("GITHUB").len(), 1);
        assert_eq!(db.search_entries("GiThUb").len(), 1);
    }

    #[test]
    fn search_matches_username() {
        let (_d, db, _, _) = populated_db();
        assert_eq!(db.search_entries("bob").len(), 1);
    }

    #[test]
    fn search_matches_url() {
        let (_d, db, _, _) = populated_db();
        let hits = db.search_entries("bank.example");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Bank");
    }

    #[test]
    fn search_matches_notes() {
        let (_d, db, _, _) = populated_db();
        let hits = db.search_entries("savings");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Bank");
    }

    #[test]
    fn search_matches_tags() {
        let (_d, db, _, _) = populated_db();
        let hits = db.search_entries("dev");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "GitHub");
    }

    #[test]
    fn search_substring_in_url() {
        let (_d, db, _, _) = populated_db();
        // .com appears in github.com only
        let hits = db.search_entries(".com");
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn search_empty_query_matches_all() {
        // Every field contains the empty string.
        let (_d, db, _, _) = populated_db();
        assert_eq!(db.search_entries("").len(), 2);
    }

    #[test]
    fn search_no_matches() {
        let (_d, db, _, _) = populated_db();
        assert!(db.search_entries("definitelynotpresent").is_empty());
    }

    #[test]
    fn search_in_group_scoped_to_subtree() {
        let (_d, db, root, sub) = populated_db();
        // root contains "GitHub" + (sub contains "Bank"): empty query returns both
        assert_eq!(db.search_entries_in_group("", &root).len(), 2);
        // sub by itself only contains "Bank"
        assert_eq!(db.search_entries_in_group("", &sub).len(), 1);
        // "github" inside sub yields nothing
        assert!(db.search_entries_in_group("github", &sub).is_empty());
        // "bank" inside root traverses into sub
        assert_eq!(db.search_entries_in_group("bank", &root).len(), 1);
    }

    #[test]
    fn search_in_group_returns_empty_for_bad_uuid() {
        let (_d, db, _, _) = populated_db();
        assert!(db.search_entries_in_group("x", "not-a-uuid").is_empty());
        assert!(db
            .search_entries_in_group("x", "00000000-0000-0000-0000-000000000000")
            .is_empty());
    }
}
