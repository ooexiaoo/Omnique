use std::process::Command;

use super::{ResultKind, SearchBackend, SearchResult};

pub struct ShellBackend;

impl ShellBackend {
    pub fn new() -> Self {
        Self
    }

    fn history_files() -> Vec<String> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
        let mut files = vec![
            format!("{}/.bash_history", home),
            format!("{}/.zsh_history", home),
            format!("{}/.local/share/fish/fish_history", home),
        ];
        if let Ok(histfile) = std::env::var("HISTFILE") {
            files.push(histfile);
        }
        files.retain(|p| std::path::Path::new(p).exists());
        files
    }
}

impl SearchBackend for ShellBackend {
    fn name(&self) -> &'static str {
        "shell"
    }

    fn icon(&self) -> &'static str {
        "\u{1f4bb}"
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return vec![];
        }

        let history_files = Self::history_files();
        if history_files.is_empty() {
            return vec![];
        }

        let mut results = Vec::new();

        for histfile in &history_files {
            let output = match Command::new("rg")
                .args([
                    "--line-number",
                    "--max-count", &max_results.to_string(),
                    "--color", "never",
                    "-i",
                    query,
                    histfile,
                ])
                .output()
            {
                Ok(o) => o,
                Err(_) => continue,
            };

            if !output.status.success() {
                continue;
            }

            let stdout = String::from_utf8_lossy(&output.stdout);

            for line in stdout.lines().take(max_results) {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() >= 2 {
                    let command = parts[1].to_string();

                    results.push(SearchResult {
                        backend: "shell".to_string(),
                        icon: "\u{1f4bb}".to_string(),
                        score: 100 - results.len() as i64,
                        title: command.clone(),
                        subtitle: Some(histfile.clone()),
                        kind: ResultKind::ShellCommand {
                            command,
                            timestamp: None,
                        },
                    });

                    if results.len() >= max_results {
                        return results;
                    }
                }
            }
        }

        results
    }
}
