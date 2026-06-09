use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "omnique", about = "Universal terminal search", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Launch interactive TUI (default)
    Tui,
    /// Search from CLI, output JSON
    Query {
        /// Search query
        query: String,
        /// Max results per backend
        #[arg(short, long, default_value = "10")]
        max: usize,
    },
    /// Build/refresh caches
    Index,
    /// List available search backends
    Sources,
}
