use std::fmt;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub backend: String,
    pub icon: String,
    pub score: i64,
    pub title: String,
    pub subtitle: Option<String>,
    #[serde(skip)]
    pub kind: ResultKind,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind")]
pub enum ResultKind {
    File { path: String, line: usize, content: String },
    GitCommit { hash: String, message: String, date: String },
    ShellCommand { command: String, timestamp: Option<i64> },
}

impl fmt::Display for ResultKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResultKind::File { path, line, .. } => write!(f, "{}:{}", path, line),
            ResultKind::GitCommit { hash, .. } => write!(f, "{}", hash),
            ResultKind::ShellCommand { command, .. } => write!(f, "{}", command),
        }
    }
}

pub trait SearchBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn icon(&self) -> &'static str;
    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult>;
}

pub mod files;
pub mod git;
pub mod shell;

pub fn all_backends() -> Vec<Box<dyn SearchBackend>> {
    vec![
        Box::new(files::FilesBackend::new()),
        Box::new(git::GitBackend::new()),
        Box::new(shell::ShellBackend::new()),
    ]
}
