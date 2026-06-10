use std::collections::HashSet;

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

    fn parse_zsh_line(line: &str) -> Option<(String, Option<i64>)> {
        if line.starts_with(": ") {
            let rest = &line[2..];
            if let Some(semi_pos) = rest.find(';') {
                let meta = &rest[..semi_pos];
                let cmd = rest[semi_pos + 1..].to_string();
                let ts = meta.split(':').next().and_then(|s| s.parse::<i64>().ok());
                return Some((cmd, ts));
            }
        }
        Some((line.to_string(), None))
    }

    fn parse_fish_line(line: &str) -> Option<(String, Option<i64>)> {
        if line.starts_with("- cmd: ") {
            let cmd = line[7..].to_string();
            Some((cmd, None))
        } else {
            None
        }
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

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        for histfile in &history_files {
            let content = match std::fs::read_to_string(histfile) {
                Ok(c) => c,
                Err(_) => continue,
            };

            for line in content.lines().rev() {
                if results.len() >= max_results {
                    return results;
                }

                let (cmd, ts) = if histfile.ends_with("zsh_history") {
                    match Self::parse_zsh_line(line) {
                        Some((c, t)) => (c, t),
                        None => continue,
                    }
                } else if histfile.ends_with("fish_history") {
                    match Self::parse_fish_line(line) {
                        Some((c, t)) => (c, t),
                        None => continue,
                    }
                } else {
                    (line.to_string(), None)
                };

                if !cmd.to_lowercase().contains(&query_lower) {
                    continue;
                }

                let cmd_trimmed = cmd.trim().to_string();
                if cmd_trimmed.is_empty() || seen.contains(&cmd_trimmed) {
                    continue;
                }
                seen.insert(cmd_trimmed);

                results.push(SearchResult {
                    backend: "shell".to_string(),
                    icon: "\u{1f4bb}".to_string(),
                    score: 100 - results.len() as i64,
                    title: cmd.clone(),
                    subtitle: Some(histfile.clone()),
                    kind: ResultKind::ShellCommand {
                        command: cmd,
                        timestamp: ts,
                    },
                });
            }
        }

        results
    }
}
