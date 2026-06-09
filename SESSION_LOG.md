# Session 1 — MVP Build (June 9, 2026)

## What We Built

A working MVP of **Omnique** — universal terminal search. One fuzzy query box that searches across files, git history, and shell history simultaneously.

### Project Structure

```
~/projects/omnique/
├── Cargo.toml
├── SPEC.md                        # Full spec document
├── SESSION_LOG.md                 # This file
├── src/
│   ├── main.rs                    # Entry point, CLI dispatch
│   ├── cli.rs                     # clap CLI definition
│   ├── config.rs                  # XDG config/data directories
│   ├── search.rs                  # Search engine (runs backends, groups results)
│   ├── tui.rs                     # Ratatui TUI (search box + results + status bar)
│   └── backends/
│       ├── mod.rs                 # SearchBackend trait + SearchResult + ResultKind
│       ├── files.rs               # ripgrep backend (searches file contents)
│       ├── git.rs                 # git log --grep backend
│       └── shell.rs               # bash/zsh/fish history backend
└── target/release/omnique         # Release binary (2.1MB)
```

### Tech Stack

| Layer | Choice |
|---|---|
| Language | Rust 1.96 |
| TUI | Ratatui 0.30 + Crossterm 0.28 |
| CLI | clap 4 (derive) |
| Storage | directories crate (XDG paths) |
| Search backends | ripgrep + git + shell history parsing |
| Serialization | serde + serde_json (CLI output) |
| Error handling | color-eyre |

### What Works

| Command | Description |
|---|---|
| `omnique` | Launch interactive TUI (type to search) |
| `omnique query <query>` | CLI mode, outputs JSON grouped by backend |
| `omnique sources` | List available search backends |
| `omnique index` | Stub (not yet implemented) |

### Backends Implemented

1. **Files** — uses `rg` (ripgrep) to search file contents. Auto-detects git repo root as search scope.
2. **Git** — runs `git log --all --oneline --grep=<query>`.
3. **Shell** — reads `~/.bash_history`, `~/.zsh_history`, `~/.local/share/fish/fish_history`, and `$HISTFILE` using ripgrep.

### TUI Keybindings

| Key | Action |
|---|---|
| Type | Real-time search (100ms debounce) |
| `j` / `k` | Navigate results up/down |
| `Tab` | Cycle between result groups |
| `Enter` | Open result (file in `$EDITOR`, commit in `git show`) |
| `Esc` | Clear query |
| `q` / `Ctrl+C` / `Ctrl+D` | Quit |

### Dependencies Installed on System

- **Rust** 1.96.0 via rustup (installed from rustup.rs)
- **ripgrep** 14.1.1 binary at `~/.local/bin/rg` (downloaded from GitHub releases)

### Known Issues / Edge Cases

- Files backend silently returns nothing if `rg` is not in PATH
- Git backend silently returns nothing if not in a git repo
- Shell history parsing is naive (no zsh timestamp extraction, no dedup)
- No fuzzy matching yet — uses exact substring via rg/git grep
- No `omnique index` implementation (needed for cached backends)
- Config module is unused (`data_dir`/`config_dir` fields only)
- `move_selection` has an edge case with `delta.unsigned_abs()` on negative values
- TUI cursor position logic is basic

### Next Steps (Phase 1 continued)

1. **Add more backends**: browser history (Chrome SQLite + Firefox SQLite), bookmarks, notes directory, recent files
2. **Implement `omnique index`**: build FTS5 cache for browser history/bookmarks
3. **TUI polish**: themes (catppuccin, dracula), collapsible groups, fuzzy matching via `nucleo`
4. **Shell integration**: Ctrl+R replacement, tmux popup (`display-popup`)
5. **Install script**: `cargo install --path .`, PATH setup instructions

### How to Run

```bash
export PATH="$HOME/.local/bin:$PATH"
cd ~/projects/omnique

# CLI query
./target/release/omnique query "fn main" --max 5

# TUI (requires real terminal)
./target/release/omnique
```
