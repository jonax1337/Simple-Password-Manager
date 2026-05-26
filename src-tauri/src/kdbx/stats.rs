use keepass::db::{Group, Node};
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

    pub(super) fn count_groups(&self, group: &Group) -> usize {
        let mut count = 1;
        for node in &group.children {
            if let Node::Group(g) = node {
                count += self.count_groups(g);
            }
        }
        count
    }

    pub fn get_dashboard_stats(&self) -> DashboardStats {
        let all_entries = self.get_all_entries();
        let total_entries = all_entries.len();
        let total_groups = self.count_groups(&self.db.root);

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
