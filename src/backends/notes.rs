use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::{ResultKind, SearchBackend, SearchResult};

const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024; // 5 MB

pub struct NotesBackend {
    notes_dir: String,
}

impl NotesBackend {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let candidates = vec![
            format!("{}/notes", home),
            format!("{}/Obsidian", home),
            format!("{}/Documents/notes", home),
            format!("{}/Documents/Obsidian", home),
        ];
        let dir = candidates.into_iter().find(|d| std::path::Path::new(d).exists());
        Self {
            notes_dir: dir.unwrap_or_else(|| format!("{}/notes", home)),
        }
    }

    #[allow(dead_code)]
    pub fn with_dir(dir: String) -> Self {
        Self { notes_dir: dir }
    }
}

impl SearchBackend for NotesBackend {
    fn name(&self) -> &'static str {
        "notes"
    }

    fn icon(&self) -> &'static str {
        "\u{1f4dd}"
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return vec![];
        }

        let notes_path = Path::new(&self.notes_dir);
        if !notes_path.exists() {
            return vec![];
        }

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        let walker = ignore::WalkBuilder::new(notes_path)
            .standard_filters(true)
            .follow_links(false)
            .build();

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "md" && ext != "markdown" && ext != "txt" && ext != "org" {
                continue;
            }

            let metadata = match path.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            if metadata.len() > MAX_FILE_SIZE {
                continue;
            }

            let file = match File::open(path) {
                Ok(f) => f,
                Err(_) => continue,
            };

            let reader = BufReader::new(file);

            for (line_num, line) in reader.lines().enumerate() {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => break,
                };

                if !line.to_lowercase().contains(&query_lower) {
                    continue;
                }

                results.push(SearchResult {
                    backend: "notes".to_string(),
                    icon: "\u{1f4dd}".to_string(),
                    score: 100 - results.len() as i64,
                    title: format!("{}:{}", path.display(), line_num + 1),
                    subtitle: Some(line.trim().to_string()),
                    kind: ResultKind::Note {
                        path: path.to_string_lossy().to_string(),
                        line: line_num + 1,
                        content: line,
                    },
                });

                if results.len() >= max_results {
                    return results;
                }
            }
        }

        results
    }
}
