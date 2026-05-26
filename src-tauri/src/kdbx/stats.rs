use sha1::{Sha1, Digest};

use super::database::Database;
use super::types::DashboardStats;

impl Database {
    pub(super) fn calculate_password_entropy(&self, password: &str) -> f64 {
        if password.is_empty() {
            return 0.0;
        }

        let mut char_space = 0;
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_digits = password.chars().any(|c| c.is_numeric());
        let has_symbols = password.chars().any(|c| !c.is_alphanumeric());

        if has_lowercase { char_space += 26; }
        if has_uppercase { char_space += 26; }
        if has_digits { char_space += 10; }
        if has_symbols { char_space += 33; }

        if char_space == 0 {
            return 0.0;
        }

        (password.len() as f64) * (char_space as f64).log2()
    }

    pub fn get_dashboard_stats(&self) -> DashboardStats {
        let all_entries = self.get_all_entries();
        let total_entries = all_entries.len();
        let total_groups = self.db.num_groups();

        let mut weak_passwords = 0;
        let mut old_passwords = 0;
        let mut expired_entries = 0;
        let mut favorite_entries = 0;
        let mut total_entropy = 0.0;
        let mut password_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        let now = chrono::Utc::now().naive_utc();
        let ninety_days_ago = now - chrono::Duration::days(90);

        for entry in &all_entries {
            let entropy = self.calculate_password_entropy(&entry.password);
            total_entropy += entropy;

            if entropy < 40.0 {
                weak_passwords += 1;
            }

            if !entry.password.is_empty() {
                // Hash password before using as key to avoid storing plaintext in memory
                let mut hasher = Sha1::new();
                hasher.update(entry.password.as_bytes());
                let digest = hasher.finalize();
                let password_hash: String = digest.iter().map(|b| format!("{:02X}", b)).collect();
                *password_counts.entry(password_hash).or_insert(0) += 1;
            }

            if let Some(modified_str) = &entry.modified {
                if let Ok(modified) = chrono::NaiveDateTime::parse_from_str(modified_str, "%Y-%m-%dT%H:%M:%S") {
                    if modified < ninety_days_ago {
                        old_passwords += 1;
                    }
                }
            }

            if entry.expires {
                if let Some(expiry_str) = &entry.expiry_time {
                    if let Ok(expiry) = chrono::NaiveDateTime::parse_from_str(expiry_str, "%Y-%m-%dT%H:%M") {
                        if expiry < now {
                            expired_entries += 1;
                        }
                    }
                }
            }

            if entry.is_favorite {
                favorite_entries += 1;
            }
        }

        let reused_passwords = password_counts.values().filter(|&&count| count > 1).count();
        let average_password_strength = if total_entries > 0 {
            total_entropy / total_entries as f64
        } else {
            0.0
        };

        DashboardStats {
            total_entries,
            total_groups,
            weak_passwords,
            reused_passwords,
            old_passwords,
            expired_entries,
            favorite_entries,
            average_password_strength,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kdbx::types::EntryData;
    use tempfile::TempDir;

    fn fresh_db() -> (TempDir, Database) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("stats.kdbx");
        let db = Database::create(path, "pw".into()).unwrap();
        (dir, db)
    }

    fn blank(group_uuid: &str, password: &str) -> EntryData {
        EntryData {
            uuid: String::new(),
            title: "T".into(),
            username: String::new(),
            password: password.into(),
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

    // -------- entropy --------

    #[test]
    fn entropy_zero_for_empty() {
        let (_d, db) = fresh_db();
        assert_eq!(db.calculate_password_entropy(""), 0.0);
    }

    #[test]
    fn entropy_grows_with_length() {
        let (_d, db) = fresh_db();
        let short = db.calculate_password_entropy("aaaa");
        let long = db.calculate_password_entropy("aaaaaaaaaaaaaaaa");
        assert!(long > short);
    }

    #[test]
    fn entropy_grows_with_charset_diversity() {
        let (_d, db) = fresh_db();
        let lower = db.calculate_password_entropy("abcdefgh");
        let mixed = db.calculate_password_entropy("Abcd1!fg");
        assert!(mixed > lower);
    }

    #[test]
    fn entropy_lowercase_only_charset_is_26() {
        let (_d, db) = fresh_db();
        // 1 char × log2(26) ≈ 4.7
        let e = db.calculate_password_entropy("a");
        assert!((e - (26f64).log2()).abs() < 1e-9);
    }

    // -------- dashboard stats --------

    #[test]
    fn empty_db_has_zero_stats() {
        let (_d, db) = fresh_db();
        let s = db.get_dashboard_stats();
        assert_eq!(s.total_entries, 0);
        // Root group itself is one group.
        assert!(s.total_groups >= 1);
        assert_eq!(s.weak_passwords, 0);
        assert_eq!(s.reused_passwords, 0);
        assert_eq!(s.favorite_entries, 0);
        assert_eq!(s.average_password_strength, 0.0);
    }

    #[test]
    fn weak_password_counted() {
        let (_d, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_entry(blank(&root, "abc")).unwrap();  // entropy ≈ 14 → weak
        db.create_entry(blank(&root, "ThisIsAVeryLongAndComplex_P4ssword!"))
            .unwrap();

        let s = db.get_dashboard_stats();
        assert_eq!(s.total_entries, 2);
        assert_eq!(s.weak_passwords, 1);
    }

    #[test]
    fn reused_password_counted_once_per_collision_group() {
        let (_d, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        // Three entries sharing one password and two with unique passwords.
        db.create_entry(blank(&root, "shared!Pw_123_long")).unwrap();
        db.create_entry(blank(&root, "shared!Pw_123_long")).unwrap();
        db.create_entry(blank(&root, "shared!Pw_123_long")).unwrap();
        db.create_entry(blank(&root, "uniqueOne_abc_123!XYZ")).unwrap();
        db.create_entry(blank(&root, "uniqueTwo_def_456!XYZ")).unwrap();

        let s = db.get_dashboard_stats();
        // One *kind* of password is reused.
        assert_eq!(s.reused_passwords, 1);
    }

    #[test]
    fn empty_password_not_counted_as_reused() {
        let (_d, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_entry(blank(&root, "")).unwrap();
        db.create_entry(blank(&root, "")).unwrap();

        let s = db.get_dashboard_stats();
        assert_eq!(s.reused_passwords, 0);
    }

    #[test]
    fn favorites_counted() {
        let (_d, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        let mut fav = blank(&root, "x");
        fav.is_favorite = true;
        db.create_entry(fav).unwrap();
        db.create_entry(blank(&root, "y")).unwrap();

        let s = db.get_dashboard_stats();
        assert_eq!(s.favorite_entries, 1);
    }

    #[test]
    fn expired_entry_counted() {
        let (_d, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        let mut e = blank(&root, "x");
        e.expires = true;
        e.expiry_time = Some("2020-01-01T00:00".into());
        db.create_entry(e).unwrap();

        let s = db.get_dashboard_stats();
        assert_eq!(s.expired_entries, 1);
    }

    #[test]
    fn future_expiry_not_counted_as_expired() {
        let (_d, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        let mut e = blank(&root, "x");
        e.expires = true;
        e.expiry_time = Some("2099-12-31T23:59".into());
        db.create_entry(e).unwrap();

        let s = db.get_dashboard_stats();
        assert_eq!(s.expired_entries, 0);
    }

    #[test]
    fn average_password_strength_is_mean_of_entropies() {
        let (_d, mut db) = fresh_db();
        let root = db.get_root_group().uuid;
        db.create_entry(blank(&root, "aaaa")).unwrap();
        db.create_entry(blank(&root, "AAAA")).unwrap();

        let s = db.get_dashboard_stats();
        // Both entries have entropy 4 × log2(26) ≈ 18.8, so the mean is the same value.
        let expected = 4.0 * (26f64).log2();
        assert!((s.average_password_strength - expected).abs() < 1e-9);
    }
}
