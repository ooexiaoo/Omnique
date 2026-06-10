use std::fmt;
use std::path::Path;

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
    Note { path: String, line: usize, content: String },
    RecentFile { path: String, last_opened: Option<String> },
    BrowserHistory { title: String, url: String, last_visit: Option<String> },
    Bookmark { title: String, url: String },
}

impl fmt::Display for ResultKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResultKind::File { path, line, .. } => write!(f, "{}:{}", path, line),
            ResultKind::GitCommit { hash, .. } => write!(f, "{}", hash),
            ResultKind::ShellCommand { command, .. } => write!(f, "{}", command),
            ResultKind::Note { path, line, .. } => write!(f, "{}:{}", path, line),
            ResultKind::RecentFile { path, .. } => write!(f, "{}", path),
            ResultKind::BrowserHistory { title, .. } => write!(f, "{}", title),
            ResultKind::Bookmark { title, .. } => write!(f, "{}", title),
        }
    }
}

pub trait SearchBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn icon(&self) -> &'static str;
    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult>;
}

pub mod browser;
pub mod files;
pub mod git;
pub mod notes;
pub mod recent;
pub mod shell;

pub fn all_backends(notes_dir: Option<String>) -> Vec<Box<dyn SearchBackend>> {
    let notes = match notes_dir {
        Some(ref d) if Path::new(d).exists() => Box::new(notes::NotesBackend::with_dir(d.clone())),
        _ => Box::new(notes::NotesBackend::new()),
    };

    vec![
        Box::new(files::FilesBackend::new()),
        Box::new(git::GitBackend::new()),
        Box::new(shell::ShellBackend::new()),
        notes,
        Box::new(recent::RecentFilesBackend::new()),
        Box::new(browser::BrowserHistoryBackend::new()),
        Box::new(browser::BookmarksBackend::new()),
    ]
}
