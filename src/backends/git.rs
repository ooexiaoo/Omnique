use std::process::Command;

use super::{ResultKind, SearchBackend, SearchResult};

pub struct GitBackend;

impl GitBackend {
    pub fn new() -> Self {
        Self
    }
}

impl SearchBackend for GitBackend {
    fn name(&self) -> &'static str {
        "git"
    }

    fn icon(&self) -> &'static str {
        "\u{1f500}"
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return vec![];
        }

        let output = match Command::new("git")
            .args([
                "log",
                "--all",
                "--oneline",
                "--max-count", &max_results.to_string(),
                "--format=%h||%s||%ar",
                "--regexp-ignore-case",
                "--grep",
                query,
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

        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(3, "||").collect();
            if parts.len() >= 2 {
                let hash = parts[0].to_string();
                let message = parts[1].to_string();
                let date = parts.get(2).unwrap_or(&"").to_string();

                results.push(SearchResult {
                    backend: "git".to_string(),
                    icon: "\u{1f500}".to_string(),
                    score: 100 - results.len() as i64,
                    title: message.clone(),
                    subtitle: Some(format!("{}  {}", hash, date)),
                    kind: ResultKind::GitCommit {
                        hash,
                        message,
                        date,
                    },
                });
            }
        }

        results
    }
}
