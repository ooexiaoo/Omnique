use super::{ResultKind, SearchBackend, SearchResult};

pub struct RecentFilesBackend;

impl RecentFilesBackend {
    pub fn new() -> Self {
        Self
    }

    fn xbel_path() -> Option<String> {
        let home = std::env::var("HOME").ok()?;
        let paths = vec![
            format!("{}/.local/share/recently-used.xbel", home),
        ];
        paths.into_iter().find(|p| std::path::Path::new(p).exists())
    }
}

impl SearchBackend for RecentFilesBackend {
    fn name(&self) -> &'static str {
        "recent"
    }

    fn icon(&self) -> &'static str {
        "\u{23f3}"
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return vec![];
        }

        let xbel_path = match Self::xbel_path() {
            Some(p) => p,
            None => return vec![],
        };

        let content = match std::fs::read_to_string(&xbel_path) {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        let mut in_bookmark = false;
        let mut href = String::new();
        let mut modified = String::new();
        let mut title = String::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("<bookmark") {
                in_bookmark = true;
                href = Self::extract_attr(line, "href").unwrap_or_default();
                modified = Self::extract_attr(line, "modified").unwrap_or_default();
                title = String::new();
            } else if in_bookmark && line.starts_with("<title>") && line.ends_with("</title>") {
                title = line[7..line.len() - 8].to_string();
            } else if in_bookmark && line.starts_with("</bookmark>") {
                let search_text = format!("{} {} {}", title, href, modified).to_lowercase();
                if search_text.contains(&query_lower) || title.to_lowercase().contains(&query_lower) {
                    results.push(SearchResult {
                        backend: "recent".to_string(),
                        icon: "\u{23f3}".to_string(),
                        score: 100 - results.len() as i64,
                        title: if title.is_empty() { href.clone() } else { title.clone() },
                        subtitle: Some(format!("{} | {}", &href, &modified)),
                        kind: ResultKind::RecentFile {
                            path: href.clone(),
                            last_opened: if modified.is_empty() { None } else { Some(modified.clone()) },
                        },
                    });
                }
                in_bookmark = false;
                if results.len() >= max_results {
                    break;
                }
            }
        }

        results
    }
}

impl RecentFilesBackend {
    fn extract_attr(line: &str, attr: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr);
        let start = line.find(&pattern)?;
        let value_start = start + pattern.len();
        let value_end = line[value_start..].find('"')?;
        Some(line[value_start..value_start + value_end].to_string())
    }
}