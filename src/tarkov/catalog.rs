use rusqlite::Connection;
use serde::Deserialize;
use std::sync::RwLock;
use tokio::sync::Mutex;

use super::models::GraphQLResponse;

const API_URL: &str = "https://api.tarkov.dev/graphql";

#[derive(Clone, Debug)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub short_name: String,
}

pub struct ItemCatalog {
    db: Mutex<Connection>,
    entries: RwLock<Vec<CatalogEntry>>,
}

#[derive(Deserialize)]
struct AllItemsData {
    items: Vec<AllItemsEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AllItemsEntry {
    id: String,
    name: String,
    short_name: String,
}

impl ItemCatalog {
    pub fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS item_catalog (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                short_name TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_item_catalog_name
                ON item_catalog(name COLLATE NOCASE);
            CREATE INDEX IF NOT EXISTS idx_item_catalog_short
                ON item_catalog(short_name COLLATE NOCASE);",
        )?;

        // Load existing data into memory
        let mut entries = Vec::new();
        {
            let mut stmt = conn.prepare("SELECT id, name, short_name FROM item_catalog")?;
            let rows = stmt.query_map([], |row| {
                Ok(CatalogEntry {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    short_name: row.get(2)?,
                })
            })?;
            for row in rows {
                entries.push(row?);
            }
        }

        tracing::info!("카탈로그 DB에서 {}개 아이템 로드", entries.len());

        Ok(Self {
            db: Mutex::new(conn),
            entries: RwLock::new(entries),
        })
    }

    pub async fn refresh(
        &self,
        client: &reqwest::Client,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let body = serde_json::json!({
            "query": super::queries::ALL_ITEMS_QUERY,
            "variables": {"lang": "en"},
        });

        let resp = client.post(API_URL).json(&body).send().await?;
        let text = resp.text().await?;
        let gql: GraphQLResponse<AllItemsData> = serde_json::from_str(&text)?;

        let data = gql
            .data
            .ok_or_else(|| "API에서 빈 응답을 받았습니다".to_string())?;

        let db = self.db.lock().await;
        db.execute_batch("BEGIN")?;
        {
            let mut stmt = db.prepare_cached(
                "INSERT OR REPLACE INTO item_catalog (id, name, short_name) VALUES (?1, ?2, ?3)",
            )?;
            for item in &data.items {
                stmt.execute(rusqlite::params![item.id, item.name, item.short_name])?;
            }
        }
        db.execute_batch("COMMIT")?;

        // Update in-memory entries
        let new_entries: Vec<CatalogEntry> = data
            .items
            .into_iter()
            .map(|i| CatalogEntry {
                id: i.id,
                name: i.name,
                short_name: i.short_name,
            })
            .collect();

        let mut entries = self.entries.write().unwrap();
        *entries = new_entries;

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.read().unwrap().is_empty()
    }

    /// Find the best matching catalog entry for a Gemini-returned item name.
    ///
    /// Matching priority:
    /// 1. Exact match on name or short_name
    /// 2. Query contains an item's short_name
    /// 3. Item name contains the query
    /// 4. Highest word overlap (minimum 1 word)
    pub fn find_match(&self, query: &str) -> Option<CatalogEntry> {
        let entries = self.entries.read().unwrap();
        if entries.is_empty() {
            return None;
        }

        let norm_query = normalize(query);
        if norm_query.is_empty() {
            return None;
        }

        // 1. Exact match on name or short_name
        for entry in entries.iter() {
            let norm_name = normalize(&entry.name);
            let norm_short = normalize(&entry.short_name);
            if norm_name == norm_query || norm_short == norm_query {
                return Some(entry.clone());
            }
        }

        // 2. Query contains short_name (e.g. "AFAK medical" matches "AFAK")
        //    Prefer longer short_name matches (more specific)
        let mut short_match: Option<&CatalogEntry> = None;
        for entry in entries.iter() {
            let norm_short = normalize(&entry.short_name);
            if norm_short.len() >= 2
                && norm_query.contains(&norm_short)
                && (short_match.is_none()
                    || entry.short_name.len() > short_match.unwrap().short_name.len())
            {
                short_match = Some(entry);
            }
        }
        if let Some(m) = short_match {
            return Some(m.clone());
        }

        // 3. Item name contains the query (e.g. query "Bastion" matches "Bastion helmet")
        //    Prefer shorter names (more specific match)
        let mut contains_match: Option<&CatalogEntry> = None;
        for entry in entries.iter() {
            let norm_name = normalize(&entry.name);
            if norm_name.contains(&norm_query)
                && (contains_match.is_none()
                    || entry.name.len() < contains_match.unwrap().name.len())
            {
                contains_match = Some(entry);
            }
        }
        if let Some(m) = contains_match {
            return Some(m.clone());
        }

        // 4. Word overlap score
        let query_words: Vec<&str> = norm_query.split_whitespace().collect();
        let mut best: Option<&CatalogEntry> = None;
        let mut best_score: usize = 0;

        for entry in entries.iter() {
            let norm_name = normalize(&entry.name);
            let name_words: Vec<&str> = norm_name.split_whitespace().collect();
            // Count is done by owned strings since lifetimes differ
            let score = count_word_overlap(&query_words, &name_words);
            if score > best_score
                || (score == best_score
                    && score > 0
                    && best.is_some_and(|b| entry.name.len() < b.name.len()))
            {
                best_score = score;
                best = Some(entry);
            }
        }

        if best_score >= 1 {
            return best.cloned();
        }

        None
    }
}

/// Normalize a string for matching: lowercase, remove parenthesized content, trim.
fn normalize(s: &str) -> String {
    let lower = s.to_lowercase();
    let no_parens = remove_parentheses(&lower);
    no_parens.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Remove parenthesized content (supports nesting).
fn remove_parentheses(s: &str) -> String {
    let mut result = String::new();
    let mut depth = 0;
    for ch in s.chars() {
        match ch {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => result.push(ch),
            _ => {}
        }
    }
    result
}

fn count_word_overlap(a: &[&str], b: &[&str]) -> usize {
    a.iter().filter(|w| b.contains(w)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_parentheses() {
        assert_eq!(remove_parentheses("AFAK (tactical)"), "AFAK ");
        assert_eq!(remove_parentheses("item (a (b)) end"), "item  end");
        assert_eq!(remove_parentheses("no parens"), "no parens");
        assert_eq!(remove_parentheses("(all gone)"), "");
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("  AFAK (Tactical)  "), "afak");
        assert_eq!(normalize("Bastion Helmet"), "bastion helmet");
    }

    fn make_entries(items: &[(&str, &str)]) -> Vec<CatalogEntry> {
        items
            .iter()
            .enumerate()
            .map(|(i, (name, short))| CatalogEntry {
                id: format!("id{i}"),
                name: name.to_string(),
                short_name: short.to_string(),
            })
            .collect()
    }

    fn find_in(entries: &[CatalogEntry], query: &str) -> Option<CatalogEntry> {
        let catalog = ItemCatalog {
            db: Mutex::new(Connection::open_in_memory().unwrap()),
            entries: RwLock::new(entries.to_vec()),
        };
        catalog.find_match(query)
    }

    #[test]
    fn test_find_match_exact_name() {
        let entries = make_entries(&[
            ("AFAK tactical individual first aid kit", "AFAK"),
            ("AI-2 medkit", "AI-2"),
        ]);
        let result = find_in(&entries, "AFAK tactical individual first aid kit");
        assert_eq!(result.unwrap().short_name, "AFAK");
    }

    #[test]
    fn test_find_match_exact_short_name() {
        let entries = make_entries(&[
            ("AFAK tactical individual first aid kit", "AFAK"),
            ("AI-2 medkit", "AI-2"),
        ]);
        let result = find_in(&entries, "AFAK");
        assert_eq!(
            result.unwrap().name,
            "AFAK tactical individual first aid kit"
        );
    }

    #[test]
    fn test_find_match_short_name_contained() {
        let entries = make_entries(&[
            ("AFAK tactical individual first aid kit", "AFAK"),
            ("AI-2 medkit", "AI-2"),
        ]);
        let result = find_in(&entries, "AFAK medical kit");
        assert_eq!(result.unwrap().short_name, "AFAK");
    }

    #[test]
    fn test_find_match_partial() {
        let entries = make_entries(&[("Bastion helmet", "Bastion"), ("LZSh light helmet", "LZSh")]);
        let result = find_in(&entries, "Bastion");
        assert_eq!(result.unwrap().name, "Bastion helmet");
    }

    #[test]
    fn test_find_match_word_overlap() {
        let entries = make_entries(&[
            ("5.11 Tactical Hexgrid plate carrier", "Hexgrid"),
            ("BNTI Gzhel-K body armor", "Gzhel-K"),
        ]);
        let result = find_in(&entries, "Hexgrid plate carrier vest");
        assert_eq!(result.unwrap().short_name, "Hexgrid");
    }

    #[test]
    fn test_find_match_no_match() {
        let entries = make_entries(&[
            ("AFAK tactical individual first aid kit", "AFAK"),
            ("AI-2 medkit", "AI-2"),
        ]);
        let result = find_in(&entries, "xyznonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_match_case_insensitive() {
        let entries = make_entries(&[("LEDX Skin Transilluminator", "LEDX")]);
        let result = find_in(&entries, "ledx skin transilluminator");
        assert_eq!(result.unwrap().short_name, "LEDX");
    }

    #[test]
    fn test_find_match_with_parentheses() {
        let entries = make_entries(&[("AK-74N 5.45x39 assault rifle", "AK-74N")]);
        let result = find_in(&entries, "AK-74N (assault rifle)");
        assert_eq!(result.unwrap().short_name, "AK-74N");
    }
}
