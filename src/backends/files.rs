use std::process::Command;

use super::{ResultKind, SearchBackend, SearchResult};

pub struct FilesBackend;

impl FilesBackend {
    pub fn new() -> Self {
        Self
    }

    fn find_repo_root(&self) -> Option<String> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .ok()?;
        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }
}

impl SearchBackend for FilesBackend {
    fn name(&self) -> &'static str {
        "files"
    }

    fn icon(&self) -> &'static str {
        "\u{1f4c1}"
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return vec![];
        }

        let search_dir = self.find_repo_root().unwrap_or_else(|| ".".to_string());

        let output = match Command::new("rg")
            .args([
                "--line-number",
                "--max-count", "1",
                "--color", "never",
                "-i",
                query,
                &search_dir,
            ])
            .output()
        {
            Ok(o) => o,
            Err(_) => return vec![],
        };

        if !output.status.success() {
            return vec![];
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut results = Vec::new();

        for line in stdout.lines().take(max_results) {
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            if parts.len() >= 2 {
                let path = parts[0].to_string();
                let line_num: usize = parts[1].parse().unwrap_or(0);
                let content = parts.get(2).unwrap_or(&"").to_string();

                results.push(SearchResult {
                    backend: "files".to_string(),
                    icon: "\u{1f4c1}".to_string(),
                    score: 100 - results.len() as i64,
                    title: format!("{}:{}", path, line_num),
                    subtitle: Some(content.trim().to_string()),
                    kind: ResultKind::File {
                        path,
                        line: line_num,
                        content,
                    },
                });
            }
        }

        results
    }
}
