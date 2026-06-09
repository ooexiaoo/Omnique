mod backends;
mod cli;
mod config;
mod search;
mod tui;

use clap::Parser;
use color_eyre::eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = cli::Cli::parse();

    match cli.command {
        Some(cli::Commands::Tui) | None => {
            let engine = search::SearchEngine::new(backends::all_backends());
            tui::run_tui(engine)?;
        }
        Some(cli::Commands::Query { query, max }) => {
            let engine = search::SearchEngine::new(backends::all_backends());
            let grouped = engine.search(&query, max);
            println!("{}", serde_json::to_string_pretty(&grouped)?);
        }
        Some(cli::Commands::Index) => {
            let _cfg = config::Config::load();
            eprintln!("Index command not yet implemented");
        }
        Some(cli::Commands::Sources) => {
            println!("Available backends:");
            for backend in backends::all_backends() {
                println!("  {} {} — {}", backend.icon(), backend.name(), backend.name());
            }
        }
    }

    Ok(())
}
