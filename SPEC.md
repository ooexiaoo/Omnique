# Omnique — Universal Terminal Search

One fuzzy query box. Searches everything: files, git history, shell history, calendar, notes, browser history, bookmarks, recent files. Results grouped by source. Opens instantly.

---

## Elevator Pitch

> Every developer has the same muscle memory: hit a keybinding, type a fuzzy query, find what they need. But the query only searches ONE thing — files, or git log, or shell history, or browser bookmarks. Omnique puts a single search box over **all** of them at once. Type `"fix login"` and see matching files _and_ git commits _and_ terminal history _and_ browser tabs all in one unified results panel.

---

## Tech Stack

| Layer | Choice | Why |
|---|---|---|
| **Language** | Rust | Single binary, performance critical, memory safe |
| **TUI** | Ratatui + Crossterm | 21k ★, de-facto Rust TUI standard 2026 |
| **CLI** | clap v4 | Industry standard, derive API |
| **Fuzzy matching** | `nucleo` | Same engine as Helix editor, extremely fast |
| **Storage** | SQLite via `rusqlite` | Cache browser history/bookmarks/notes index |
| **Search** | `grep-cli` (ripgrep crate) | Fastest file content search |
| **Full-text search** | SQLite FTS5 | Indexed search for cached backends |
| **Config** | `directories` crate | XDG-compliant paths |
| **Error handling** | `color-eyre` | Pretty panics and errors |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         omnique                              │
│                                                              │
│  ┌─────────────────────┐   ┌───────────────────────────────┐ │
│  │     CLI Mode        │   │          TUI Mode              │ │
│  │                     │   │                               │ │
│  │  omnique query <q>  │   │  ┌─────────────────────────┐  │ │
│  │  omnique index      │   │  │  Search Box (nucleo)    │  │ │
│  │  omnique daemon     │   │  │  [fix login         ]   │  │ │
│  │  omnique sources    │   │  └─────────────────────────┘  │ │
│  └─────────────────────┘   │           │                    │ │
│                            │           ▼                    │ │
│                            │  ┌─────────────────────────┐  │ │
│                            │  │     Results Panel        │  │ │
│                            │  │                          │  │ │
│                            │  │ ┌──────────────────────┐ │  │ │
│                            │  │ │ Files (3)            │ │  │ │
│                            │  │ │ src/auth.rs:142      │ │  │ │
│                            │  │ │   "fix login token"  │ │  │ │
│                            │  │ │ routes/session.rs:56 │ │  │ │
│                            │  │ │   "login validation" │ │  │ │
│                            │  │ └──────────────────────┘ │  │ │
│                            │  │ ┌──────────────────────┐ │  │ │
│                            │  │ │ Git Log (2)          │ │  │ │
│                            │  │ │ 3d ago fix login bug │ │  │ │
│                            │  │ │ 1w ago add login btn │ │  │ │
│                            │  │ └──────────────────────┘ │  │ │
│                            │  │ ┌──────────────────────┐ │  │ │
│                            │  │ │ Shell History (4)    │ │  │ │
│                            │  │ │ cargo test auth      │ │  │ │
│                            │  │ │ git log --grep login │ │  │ │
│                            │  │ └──────────────────────┘ │  │ │
│                            │  └─────────────────────────┘  │ │
│                            └───────────────────────────────┘ │
│                                                              │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                   Backend Manager                        │ │
│  │                                                         │ │
│  │  ┌──────────┐ ┌───────────┐ ┌──────────┐ ┌──────────┐  │ │
│  │  │  Files   │ │  Git Log  │ │  Shell   │ │  Notes   │  │ │
│  │  │  (rg)    │ │  (git)    │ │  (hist)  │ │  (.md)   │  │ │
│  │  └──────────┘ └───────────┘ └──────────┘ └──────────┘  │ │
│  │  ┌──────────┐ ┌───────────┐ ┌──────────┐ ┌──────────┐  │ │
│  │  │ Browser  │ │  Calendar │ │ Bookmarks│ │  Recent  │  │ │
│  │  │ (SQLite) │ │  (files)  │ │ (browser)│ │  Files   │  │ │
│  │  └──────────┘ └───────────┘ └──────────┘ └──────────┘  │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## Search Backends (Phase 1)

Each backend is a trait implementation:

```rust
#[async_trait]
trait SearchBackend {
    fn name(&self) -> &'static str;
    fn icon(&self) -> &'static str;  // Unicode icon for results panel
    async fn search(&self, query: &str, ctx: &SearchContext) -> Vec<SearchResult>;
    fn priority(&self) -> u8;        // Lower = higher in results
}
```

### 1. Files (`ripgrep`)

- Searches file **contents** using ripgrep
- Context: show matched line + 1 line before/after
- Scoped to current git repo (if inside one) or `~` otherwise
- Respects `.gitignore`
- Counts: limit to top 20 results per query

### 2. Git Log

- `git log --all --oneline --grep=<query>` with `--format=%h %s %ar`
- Includes commit hash (truncated), subject, relative date
- Falls back to `git log --all --oneline --author-date-order` for empty query

### 3. Shell History

- Parses `~/.bash_history`, `~/.zsh_history`, or Atuin DB
- Filters by substring match + fuzzy scores via nucleo
- Shows timestamp, command, directory (if available)

### 4. Notes

- Searches markdown files under a configurable notes directory (~/notes, ~/Obsidian, etc.)
- Uses ripgrep for content, `fd` for filenames
- Shows file path + matched line

### 5. Browser History (cached)

- Reads Chrome/Firefox SQLite history databases
- Indexed into omnique's own SQLite cache for fast querying
- Cache refreshed on `omnique index`
- Shows page title, URL, last visit time

### 6. Bookmarks (cached)

- Same as browser history — reads Chrome/Firefox bookmark SQLite
- Cached locally
- Shows title, URL, folder

### 7. Recent Files

- Uses `~/.local/share/recently-used.xbel` (freedesktop) or editor MRU files
- Shows filename, full path, last opened time

---

## TUI Design

### Layout

```
┌─────────────────────────────────────────────┐
│  Omnique                              Ctrl+D │
│─────────────────────────────────────────────│
│  > fix login                          [3/42]│
│─────────────────────────────────────────────│
│                                              │
│  📁 Files (3)                ▼ expand        │
│  ┌ src/auth.rs:142                          │
│  │  · "fix login token validation"          │
│  ├ routes/session.rs:56                     │
│  │  · "login endpoint handler"              │
│  └ src/middleware.rs:12                      │
│  ··· ···                                    │
│  🔖 Git Log (2)                             │
│  ┌ a1b2c3d fix login bug        3 days ago  │
│  └ e5f6g7a add login button     1 week ago  │
│  ··· ···                                    │
│  💻 Shell History (4)                       │
│  ┌ cargo test auth             2h ago        │
│  │ git log --grep login        yesterday     │
│  └ npm run dev                 2d ago        │
│  ··· ···                                    │
│                                              │
└─────────────────────────────────────────────┘
│  [Tab: cycle] [Enter: open] [/: filter] [q: quit] │
└─────────────────────────────────────────────────────┘
```

### Keybindings

| Key | Action |
|---|---|
| Type | Filter results in real-time (nucleo fuzzy match) |
| `Enter` | Open selected result (file in $EDITOR, URL in browser, commit in git log, etc.) |
| `Tab` | Cycle focus between search box and result groups |
| `j` / `k` | Navigate results (when focus in results) |
| `h` / `l` | Collapse / expand result group |
| `Ctrl+d` | Toggle debug info |
| `Ctrl+r` | Refresh / re-index |
| `Esc` | Clear query / close |
| `Ctrl+c` / `q` | Quit |

### Opening Results

| Result Type | Action |
|---|---|
| File match | Open in `$EDITOR` at line number |
| Git commit | Run `git show <hash>` |
| Shell command | Copy to clipboard / execute? |
| URL | Open in `$BROWSER` |
| Note | Open in `$EDITOR` |

---

## CLI Design

```
omnique                    # Launch TUI (interactive mode)
omnique query <query>      # CLI mode: print results as JSON
omnique index              # Build/refresh caches (browser history, bookmarks)
omnique daemon             # Start background indexer (optional)
omnique sources            # List enabled backends
omnique source enable <s>  # Enable a backend
omnique source disable <s> # Disable a backend
```

### JSON Output (CLI Mode)

```json
{
  "query": "fix login",
  "duration_ms": 142,
  "results": [
    {
      "source": "files",
      "entries": [
        { "path": "src/auth.rs", "line": 142, "content": "fix login token validation", "score": 0.95 }
      ]
    },
    {
      "source": "git",
      "entries": [
        { "hash": "a1b2c3d", "message": "fix login bug", "relative_time": "3 days ago", "score": 0.88 }
      ]
    }
  ]
}
```

---

## Data Flow

```
User types query
       │
       ▼
  nucleo fuzzy matcher (incremental)
       │
       ▼
  Backend Manager ──► Files (rg) ─────► parse matches ──► score + rank
                   │─► Git Log ───────► parse output
                   │─► Shell History ─► fuzzy match history lines
                   │─► Browser Cache ─► FTS5 query on SQLite
                   │─► Notes ─────────► rg on notes dir
                   └─► ... (more backends)
       │
       ▼
  Merge + sort results (by score, interleaved by source)
       │
       ▼
  Render TUI panel
```

### Caching Strategy

| Backend | Cache | Refresh |
|---|---|---|
| Files | None (always live via rg) | N/A |
| Git Log | None (always live via git) | N/A |
| Shell History | None (memmap history file) | N/A |
| Browser History | SQLite table | `omnique index` or daemon |
| Bookmarks | SQLite table | `omnique index` or daemon |
| Notes | Optional FTS5 index | `omnique index` |
| Recent Files | SQLite table | `omnique index` or daemon |

---

## Phased Roadmap

### Phase 1 — MVP (2-3 weeks)

- [ ] Project scaffolding (Cargo, clap, Ratatui, crossterm)
- [ ] Basic TUI shell (search box + results panel)
- [ ] Files backend via `grep-cli` crate
- [ ] Git log backend
- [ ] Shell history backend (`.bash_history`/`.zsh_history`)
- [ ] nucleo fuzzy matching wired up
- [ ] `omnique query <q>` CLI mode with JSON output
- [ ] Open file results in `$EDITOR`

### Phase 2 — Cached backends (1-2 weeks)

- [ ] Browser history cache + FTS5 (Chrome + Firefox)
- [ ] Browser bookmarks cache
- [ ] Notes backend (configurable directory)
- [ ] Recent files backend
- [ ] `omnique index` command
- [ ] Results grouping by source

### Phase 3 — Polish (1 week)

- [ ] Collapse/expand result groups
- [ ] Tab to cycle focus
- [ ] Theme support (catppuccin, dracula, nord, etc.)
- [ ] Config file (`~/.config/omnique/config.toml`)
- [ ] Installation: `cargo install omnique`
- [ ] Documentation + demo GIF

### Phase 4 — Advanced (future)

- [ ] Daemon mode with inotify watchers for auto-indexing
- [ ] Tmux popup integration (`display-popup`)
- [ ] Plugin system for custom backends
- [ ] Scored interleaving (mix results from different sources by relevance)
- [ ] Shell integration (Ctrl+R replacement)
- [ ] MCP server (AI agent integration)

---

## Naming

**omnique** — from Latin _omnis_ (all/every) + _-ique_ (suffix forming adverbs). "From everywhere."

---

## Why This Will Work

1. **Daily driver** — developers search constantly, this consolidates 6+ separate search tools
2. **No competitor** — no unified terminal search tool exists
3. **Obvious value** — the use case is immediately understood
4. **Feasible MVP** — file search + git log + shell history works with zero caching, pure existing tools
5. **Extensible** — backend trait makes adding sources trivial
6. **Community hook** — "lazygit for search" is an easy pitch

---

## Similar Tools (for reference)

| Tool | What it does | How omnique differs |
|---|---|---|
| `fzf` | Fuzzy find stdin | Single-source, no TUI, no grouping |
| `ripgrep` | File search | Files only, no TUI |
| `atuin` | Shell history search + sync | Shell history only |
| `broot` | TUI file browser + search | Files only |
| `ctrlp`/`telescope` (neovim) | Editor-integrated search | Editor-only, not system-wide |
| `Raycast`/`Alfred` (macOS) | Desktop search | Not terminal-native, no pipe/SSH/CLI use |
