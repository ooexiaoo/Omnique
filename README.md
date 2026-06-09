# Omnique

Universal terminal search. One fuzzy query box for files, git history, shell history — all at once.

```bash
omnique query "fn main" --max 5   # CLI mode, JSON output
omnique                            # Interactive TUI
```

## Backends

| Backend | Source | Status |
|---------|--------|--------|
| Files | `rg` (ripgrep) | ✅ |
| Git log | `git log --grep` | ✅ |
| Shell history | bash/zsh/fish | ✅ |
| Browser history | Chrome/Firefox SQLite | 📝 |
| Bookmarks | Chrome/Firefox | 📝 |
| Notes | Markdown directory | 📝 |

## Install

```bash
cargo install --git https://github.com/ooexiaoo/Omnique
```

Requires [ripgrep](https://github.com/BurntSushi/ripgrep) for the files backend.

## Usage

```
omnique                    # Launch TUI
omnique query <query>      # CLI search, JSON output
omnique sources            # List backends
omnique index              # Refresh caches
```

## Keybindings (TUI)

| Key | Action |
|---|---|
| Type | Real-time search |
| `j` / `k` | Navigate |
| `Tab` | Cycle groups |
| `Enter` | Open result |
| `Esc` | Clear query |
| `q` | Quit |
