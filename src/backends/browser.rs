use std::path::PathBuf;

use rusqlite::Connection;

use super::{ResultKind, SearchBackend, SearchResult};

fn chrome_bookmark_urls(path: &str) -> Vec<(String, String)> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let mut urls = Vec::new();
    extract_chrome_bookmarks(&json, &mut urls);
    urls
}

fn extract_chrome_bookmarks(value: &serde_json::Value, urls: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Object(map) => {
            if let (Some(url_val), Some(name_val)) = (map.get("url"), map.get("name")) {
                if let (Some(url), Some(name)) = (url_val.as_str(), name_val.as_str()) {
                    urls.push((name.to_string(), url.to_string()));
                }
            }
            for val in map.values() {
                extract_chrome_bookmarks(val, urls);
            }
        }
        serde_json::Value::Array(arr) => {
            for val in arr {
                extract_chrome_bookmarks(val, urls);
            }
        }
        _ => {}
    }
}

fn db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let data_dir = directories::ProjectDirs::from("com", "omnique", "omnique")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(home).join(".local").join("share").join("omnique"));
    data_dir.join("browser.db")
}

pub fn run_index() -> Result<(), String> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let conn = Connection::open(&path).map_err(|e| e.to_string())?;

    conn.execute_batch(
        "DROP TABLE IF EXISTS history;"
    ).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "DROP TABLE IF EXISTS bookmarks;"
    ).map_err(|e| e.to_string())?;

    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS history USING fts5(title, url, source UNINDEXED, last_visit);"
    ).map_err(|e| e.to_string())?;

    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS bookmarks USING fts5(title, url, source UNINDEXED);"
    ).map_err(|e| e.to_string())?;



    index_chrome(&conn)?;
    index_firefox(&conn)?;

    let h_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let b_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM bookmarks", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;

    println!("Indexed {} history entries and {} bookmarks", h_count, b_count);
    Ok(())
}

fn index_chrome(conn: &Connection) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let chrome_configs = vec![
        "google-chrome",
        "chromium",
    ];

    for config in &chrome_configs {
        let history_path = format!("{}/.config/{}/Default/History", home, config);
        let bookmarks_path = format!("{}/.config/{}/Default/Bookmarks", home, config);

        if std::path::Path::new(&history_path).exists() {
            if let Ok(chrome_conn) = Connection::open(&history_path) {
                if let Ok(mut stmt) = chrome_conn
                    .prepare("SELECT url, title, last_visit_time FROM urls ORDER BY last_visit_time DESC LIMIT 5000")
                {
                    if let Ok(rows) = stmt.query_map([], |row| {
                        let url: String = row.get(0)?;
                        let title: String = row.get(1).unwrap_or_default();
                        let last_visit: i64 = row.get(2).unwrap_or(0);
                        Ok((url, title, last_visit))
                    }) {
                        for row in rows {
                            if let Ok((url, title, last_visit)) = row {
                                let ts = if last_visit > 0 {
                                    let secs = (last_visit / 1_000_000) - 11_644_473_600i64;
                                    chrono::DateTime::from_timestamp(secs, 0)
                                        .map(|d| d.to_rfc3339())
                                        .unwrap_or_default()
                                } else {
                                    String::new()
                                };
                                let _ = conn.execute(
                                    "INSERT INTO history (title, url, source, last_visit) VALUES (?1, ?2, 'chrome', ?3)",
                                    rusqlite::params![title, url, ts],
                                );
                            }
                        }
                    }
                }
            }
        }

        if std::path::Path::new(&bookmarks_path).exists() {
            let urls = chrome_bookmark_urls(&bookmarks_path);
            for (name, url) in urls {
                let _ = conn.execute(
                    "INSERT INTO bookmarks (title, url, source) VALUES (?1, ?2, 'chrome')",
                    rusqlite::params![name, url],
                );
            }
        }
    }

    Ok(())
}

fn index_firefox(conn: &Connection) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let profiles_dir = format!("{}/.mozilla/firefox", home);
    let profiles_dir = PathBuf::from(profiles_dir);

    if !profiles_dir.exists() {
        return Ok(());
    }

    let entries = std::fs::read_dir(&profiles_dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let places_path = path.join("places.sqlite");
        if !places_path.exists() {
            continue;
        }

        if let Ok(fx_conn) = Connection::open(&places_path) {
            if let Ok(mut stmt) = fx_conn
                .prepare("SELECT url, title, last_visit_date FROM moz_places ORDER BY last_visit_date DESC LIMIT 5000")
            {
                if let Ok(rows) = stmt.query_map([], |row| {
                    let url: String = row.get(0)?;
                    let title: String = row.get(1).unwrap_or_default();
                    let last_visit: Option<i64> = row.get(2)?;
                    Ok((url, title, last_visit))
                }) {
                    for row in rows {
                        if let Ok((url, title, last_visit)) = row {
                            let ts = if let Some(lv) = last_visit {
                                chrono::DateTime::from_timestamp(lv / 1_000_000, 0)
                                    .map(|d| d.to_rfc3339())
                                    .unwrap_or_default()
                            } else {
                                String::new()
                            };
                            let _ = conn.execute(
                                "INSERT INTO history (title, url, source, last_visit) VALUES (?1, ?2, 'firefox', ?3)",
                                rusqlite::params![title, url, ts],
                            );
                        }
                    }
                }
            }

            if let Ok(mut stmt) = fx_conn.prepare(
                "SELECT p.url, b.title FROM moz_bookmarks b \
                 JOIN moz_places p ON b.fk = p.id \
                 WHERE b.type = 1 AND p.url IS NOT NULL"
            ) {
                if let Ok(rows) = stmt.query_map([], |row| {
                    let url: String = row.get(0)?;
                    let title: String = row.get(1).unwrap_or_default();
                    Ok((url, title))
                }) {
                    for row in rows {
                        if let Ok((url, title)) = row {
                            let _ = conn.execute(
                                "INSERT INTO bookmarks (title, url, source) VALUES (?1, ?2, 'firefox')",
                                rusqlite::params![title, url],
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn search_fts(
    table: &str,
    query: &str,
    max_results: usize,
    backend_name: &str,
    icon: &str,
) -> Vec<SearchResult> {
    let path = db_path();
    if !path.exists() {
        return vec![];
    }

    let conn = match Connection::open(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let sql = format!(
        "SELECT title, url, last_visit FROM {} WHERE {} MATCH ?1 ORDER BY rank LIMIT ?2",
        table, table
    );

    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let rows = match stmt.query_map(rusqlite::params![query, max_results as i64], |row| {
        let title: String = row.get(0)?;
        let url: String = row.get(1)?;
        let last_visit: String = row.get(2).unwrap_or_default();
        Ok((title, url, last_visit))
    }) {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let mut results = Vec::new();
    for row in rows {
        if let Ok((title, url, last_visit)) = row {
            let display_title = if title.is_empty() { url.clone() } else { title.clone() };
            results.push(SearchResult {
                backend: backend_name.to_string(),
                icon: icon.to_string(),
                score: 100 - results.len() as i64,
                title: display_title,
                subtitle: Some(url.clone()),
                kind: ResultKind::BrowserHistory {
                    title,
                    url,
                    last_visit: if last_visit.is_empty() { None } else { Some(last_visit) },
                },
            });
        }
    }

    results
}

fn search_bookmarks_fts(
    query: &str,
    max_results: usize,
) -> Vec<SearchResult> {
    let path = db_path();
    if !path.exists() {
        return vec![];
    }

    let conn = match Connection::open(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut stmt = match conn.prepare(
        "SELECT title, url FROM bookmarks WHERE bookmarks MATCH ?1 ORDER BY rank LIMIT ?2"
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let rows = match stmt.query_map(rusqlite::params![query, max_results as i64], |row| {
        let title: String = row.get(0)?;
        let url: String = row.get(1)?;
        Ok((title, url))
    }) {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let mut results = Vec::new();
    for row in rows {
        if let Ok((title, url)) = row {
            let display_title = if title.is_empty() { url.clone() } else { title.clone() };
            results.push(SearchResult {
                backend: "bookmarks".to_string(),
                icon: "\u{1f516}".to_string(),
                score: 100 - results.len() as i64,
                title: display_title,
                subtitle: Some(url.clone()),
                kind: ResultKind::Bookmark { title, url },
            });
        }
    }

    results
}

pub struct BrowserHistoryBackend;

impl BrowserHistoryBackend {
    pub fn new() -> Self {
        Self
    }
}

impl SearchBackend for BrowserHistoryBackend {
    fn name(&self) -> &'static str {
        "browser"
    }

    fn icon(&self) -> &'static str {
        "\u{1f310}"
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return vec![];
        }
        search_fts("history", query, max_results, "browser", "\u{1f310}")
    }
}

pub struct BookmarksBackend;

impl BookmarksBackend {
    pub fn new() -> Self {
        Self
    }
}

impl SearchBackend for BookmarksBackend {
    fn name(&self) -> &'static str {
        "bookmarks"
    }

    fn icon(&self) -> &'static str {
        "\u{1f516}"
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return vec![];
        }
        search_bookmarks_fts(query, max_results)
    }
}