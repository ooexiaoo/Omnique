mod backends;
mod cli;
mod config;
mod search;
mod tui;

use clap::Parser;
use color_eyre::eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.to_string();
        if !msg.contains("prefilter") {
            orig_hook(info);
        }
    }));

    let cfg = config::Config::load();
    let cli = cli::Cli::parse();

    match cli.command {
        Some(cli::Commands::Tui) | None => {
            let engine = search::SearchEngine::new(backends::all_backends(cfg.notes_dir.clone()));
            tui::run_tui(engine, cfg)?;
        }
        Some(cli::Commands::Query { query, max }) => {
            let mut engine = search::SearchEngine::new(backends::all_backends(cfg.notes_dir.clone()));
            let grouped = engine.search(&query, max);
            println!("{}", serde_json::to_string_pretty(&grouped)?);
        }
        Some(cli::Commands::Index) => {
            match backends::browser::run_index() {
                Ok(()) => {}
                Err(e) => eprintln!("Index error: {}", e),
            }
        }
        Some(cli::Commands::Sources) => {
            let descriptions = [
                ("files", "File content search via built-in file walk"),
                ("git", "Git commit history search"),
                ("shell", "Shell command history"),
                ("notes", "Markdown/Org notes directory"),
                ("recent", "Recently opened files (freedesktop XBEL)"),
                ("browser", "Browser history (cached via SQLite) — run `omnique index` first"),
                ("bookmarks", "Browser bookmarks (cached via SQLite) — run `omnique index` first"),
            ];
            println!("Available backends:");
            for backend in backends::all_backends(cfg.notes_dir.clone()) {
                let desc = descriptions.iter().find(|(n, _)| *n == backend.name()).map(|(_, d)| *d).unwrap_or("");
                println!("  {} {} — {}", backend.icon(), backend.name(), desc);
            }
        }
    }

    Ok(())
}
