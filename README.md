# Omnique

Universal terminal search. One query box for files, git history, shell history, browser history, bookmarks, notes, recent files — all at once.

```bash
# Interactive TUI
omnique

# CLI mode, JSON output
omnique query "fn main" --max 5

# Cache browser history/bookmarks
omnique index
```

## Install

**Option A — Download binary (recommended, 5.5 MB)**

```bash
curl -LO https://github.com/ooexiaoo/Omnique/releases/latest/download/omnique-x86_64-linux.tar.gz
tar xzf omnique-x86_64-linux.tar.gz
sudo mv omnique /usr/local/bin
```

**Option B — From source**

```bash
cargo install --git https://github.com/ooexiaoo/Omnique
```

Zero runtime dependencies. The binary is fully static (links only libc/libm).

## Usage

```
omnique                    # Launch TUI
omnique query <query>      # CLI search, JSON output
omnique index              # Cache browser history/bookmarks
omnique sources            # List backends
```

## Keybindings (TUI)

| Key | Action |
|---|---|
| Type | Real-time fuzzy search |
| `↑` / `↓` | Navigate results |
| `Tab` | Cycle result groups |
| `Enter` | Open result (file in $EDITOR, URL in browser) |
| `Esc` | Clear query |
| `Alt+H` / `Alt+L` | Collapse / expand group |
| `Ctrl+↑` / `Ctrl+↓` | Increase / decrease results per page |
| `Ctrl+R` | Re-index browser caches |
| `Ctrl+C` / `Ctrl+D` | Quit |

## Config

```toml
# ~/.config/omnique/config.toml
theme = "catppuccin"       # catppuccin, dracula, nord
max_results = 10            # Results per backend
notes_dir = "~/notes"       # Notes directory (auto-detected)
```

## Backends

| Backend | Source | Status |
|---------|--------|--------|
| Files | Built-in file walk + `.gitignore` | ✅ |
| Git log | `git log --grep` | ✅ |
| Shell history | bash/zsh/fish | ✅ |
| Browser history | Chrome/Firefox SQLite (cached) | ✅ |
| Bookmarks | Chrome/Firefox (cached) | ✅ |
| Notes | Markdown/Org directory | ✅ |
| Recent files | freedesktop XBEL | ✅ |

## Star History

<a href="https://www.star-history.com/#ooexiaoo/Omnique&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ooexiaoo/Omnique&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ooexiaoo/Omnique&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ooexiaoo/Omnique&type=Date" />
 </picture>
</a>
