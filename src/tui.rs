use std::collections::HashSet;
use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::backends::SearchResult;
use crate::config::{Config, Theme};
use crate::search::{GroupedResults, SearchEngine};

const POLL_DURATION: Duration = Duration::from_millis(50);

pub struct App {
    query: String,
    cursor_position: usize,
    grouped_results: Vec<GroupedResults>,
    selected_group: usize,
    selected_item: usize,
    list_state: ListState,
    search_engine: SearchEngine,
    last_search: Instant,
    debounce_duration: Duration,
    search_version: usize,
    pending_search: bool,
    collapsed: HashSet<usize>,
    theme: Theme,
    max_results: usize,
}

impl App {
    pub fn new(search_engine: SearchEngine, cfg: Config) -> Self {
        let theme = Theme::from_name(&cfg.theme);
        Self {
            query: String::new(),
            cursor_position: 0,
            grouped_results: Vec::new(),
            selected_group: 0,
            selected_item: 0,
            list_state: ListState::default(),
            search_engine,
            last_search: Instant::now(),
            debounce_duration: Duration::from_millis(100),
            search_version: 0,
            pending_search: false,
            collapsed: HashSet::new(),
            theme,
            max_results: cfg.max_results,
        }
    }

    fn trigger_search(&mut self) {
        self.pending_search = true;
    }

    fn execute_search(&mut self) {
        let query = self.query.clone();
        let version = self.search_version;

        let grouped = self.search_engine.search(&query, self.max_results);

        if version == self.search_version {
            self.grouped_results = grouped;
            self.selected_group = 0;
            self.selected_item = 0;
            self.update_list_state();
        }
    }

    fn open_selected_command(&self) -> Option<std::process::Command> {
        let result = self.selected_result()?;
        match &result.kind {
            crate::backends::ResultKind::File { path, line, .. } => {
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
                let mut cmd = std::process::Command::new("sh");
                cmd.args(["-c", &format!("{} +{} {}", editor, line, path)]);
                Some(cmd)
            }
            crate::backends::ResultKind::GitCommit { hash, .. } => {
                let mut cmd = std::process::Command::new("git");
                cmd.args(["show", hash]);
                Some(cmd)
            }
            crate::backends::ResultKind::ShellCommand { command, .. } => {
                let mut cmd = std::process::Command::new("sh");
                cmd.args(["-c", &format!("echo \"{}\" | xclip -selection clipboard 2>/dev/null; echo \"Copied to clipboard: {}\"", command, command)]);
                Some(cmd)
            }
            crate::backends::ResultKind::Note { path, line, .. } => {
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
                let mut cmd = std::process::Command::new("sh");
                cmd.args(["-c", &format!("{} +{} {}", editor, line, path)]);
                Some(cmd)
            }
            crate::backends::ResultKind::RecentFile { path, .. } => {
                let mut cmd = std::process::Command::new("xdg-open");
                cmd.args([path]);
                Some(cmd)
            }
            crate::backends::ResultKind::BrowserHistory { url, .. } => {
                let mut cmd = std::process::Command::new("xdg-open");
                cmd.args([url]);
                Some(cmd)
            }
            crate::backends::ResultKind::Bookmark { url, .. } => {
                let mut cmd = std::process::Command::new("xdg-open");
                cmd.args([url]);
                Some(cmd)
            }
        }
    }

    fn toggle_collapse(&mut self) {
        if self.selected_group < self.grouped_results.len() {
            if self.collapsed.contains(&self.selected_group) {
                self.collapsed.remove(&self.selected_group);
            } else {
                self.collapsed.insert(self.selected_group);
            }
            self.update_list_state();
        }
    }

    fn update_list_state(&mut self) {
        let flat = self.selected_flat_index();
        self.list_state.select(Some(flat));
    }

    fn selected_flat_index(&self) -> usize {
        let mut idx = 0;
        for (gi, group) in self.grouped_results.iter().enumerate() {
            if gi == self.selected_group {
                idx += 1;
                if !self.collapsed.contains(&gi) {
                    for (ri, result) in group.results.iter().enumerate() {
                        if ri == self.selected_item {
                            return idx;
                        }
                        idx += 1;
                        if result.subtitle.is_some() {
                            idx += 1;
                        }
                    }
                }
                return idx;
            }
            idx += 1;
            if !self.collapsed.contains(&gi) {
                for result in &group.results {
                    idx += 1;
                    if result.subtitle.is_some() {
                        idx += 1;
                    }
                }
            }
        }
        idx
    }

    fn selected_result(&self) -> Option<&SearchResult> {
        self.grouped_results
            .get(self.selected_group)
            .and_then(|g| g.results.get(self.selected_item))
    }

    fn move_selection(&mut self, delta: isize) {
        let visible_groups: Vec<usize> = (0..self.grouped_results.len())
            .filter(|i| !self.collapsed.contains(i))
            .collect();

        let total_items: usize = visible_groups
            .iter()
            .map(|&gi| self.grouped_results[gi].results.len())
            .sum();

        if total_items == 0 {
            return;
        }

        let current_visible_idx: usize = visible_groups
            .iter()
            .position(|&gi| gi == self.selected_group)
            .unwrap_or(0);

        let prefix_items: usize = visible_groups
            .iter()
            .take(current_visible_idx)
            .map(|&gi| self.grouped_results[gi].results.len())
            .sum();

        let current_idx = prefix_items + self.selected_item;

        let new_idx = if delta < 0 {
            current_idx.saturating_sub(delta.unsigned_abs())
        } else {
            (current_idx + delta as usize).min(total_items - 1)
        };

        let mut remaining = new_idx;
        for &gi in &visible_groups {
            let len = self.grouped_results[gi].results.len();
            if remaining < len {
                self.selected_group = gi;
                self.selected_item = remaining;
                self.update_list_state();
                return;
            }
            remaining = remaining.saturating_sub(len);
        }
    }

    fn insert_char(&mut self, c: char) {
        self.query.insert(self.cursor_position, c);
        self.cursor_position += 1;
        self.trigger_search();
    }

    fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.query.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
            self.trigger_search();
        }
    }

    fn delete_forward(&mut self) {
        if self.cursor_position < self.query.len() {
            self.query.remove(self.cursor_position);
            self.trigger_search();
        }
    }

    fn move_cursor_left(&mut self) {
        self.cursor_position = self.cursor_position.saturating_sub(1);
    }

    fn move_cursor_right(&mut self) {
        self.cursor_position = (self.cursor_position + 1).min(self.query.len());
    }

    fn move_to_start(&mut self) {
        self.cursor_position = 0;
    }

    fn move_to_end(&mut self) {
        self.cursor_position = self.query.len();
    }

    fn clear_query(&mut self) {
        self.query.clear();
        self.cursor_position = 0;
        self.grouped_results.clear();
        self.selected_group = 0;
        self.selected_item = 0;
        self.list_state = ListState::default();
    }

    fn adjust_max_results(&mut self, delta: isize) {
        let new = if delta > 0 {
            self.max_results.saturating_add(5).min(100)
        } else {
            self.max_results.saturating_sub(5).max(5)
        };
        if new != self.max_results {
            self.max_results = new;
            self.trigger_search();
        }
    }

}

pub fn run_tui(search_engine: SearchEngine, cfg: Config) -> color_eyre::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;
    let mut app = App::new(search_engine, cfg);
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    if let Err(e) = res {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> color_eyre::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(POLL_DURATION)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                            return Ok(());
                        }
                        KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                            return Ok(());
                        }
                        KeyCode::Char('r') if key.modifiers == KeyModifiers::CONTROL => {
                            std::thread::spawn(|| {
                                let _ = crate::backends::browser::run_index();
                            });
                        }
                        KeyCode::Enter => {
                            if let Some(mut cmd) = app.open_selected_command() {
                                disable_raw_mode()?;
                                execute!(io::stdout(), LeaveAlternateScreen)?;
                                let _ = cmd.status();
                                println!("\nPress Enter to return to Omnique...");
                                let _ = std::io::stdin().read_line(&mut String::new());
                                enable_raw_mode()?;
                                execute!(io::stdout(), EnterAlternateScreen)?;
                            }
                        }
                        KeyCode::Char('h') if key.modifiers == KeyModifiers::ALT => {
                            app.toggle_collapse();
                        }
                        KeyCode::Char('l') if key.modifiers == KeyModifiers::ALT => {
                            app.toggle_collapse();
                        }
                        KeyCode::Char(c) => {
                            app.insert_char(c);
                        }
                        KeyCode::Backspace => {
                            app.delete_char();
                        }
                        KeyCode::Delete => {
                            app.delete_forward();
                        }
                        KeyCode::Left => {
                            app.move_cursor_left();
                        }
                        KeyCode::Right => {
                            app.move_cursor_right();
                        }
                        KeyCode::Home => {
                            app.move_to_start();
                        }
                        KeyCode::End => {
                            app.move_to_end();
                        }
                        KeyCode::Up if key.modifiers == KeyModifiers::CONTROL => {
                            app.adjust_max_results(1);
                        }
                        KeyCode::Down if key.modifiers == KeyModifiers::CONTROL => {
                            app.adjust_max_results(-1);
                        }
                        KeyCode::Up => {
                            app.move_selection(-1);
                        }
                        KeyCode::Down => {
                            app.move_selection(1);
                        }
                        KeyCode::Esc => {
                            if !app.query.is_empty() {
                                app.clear_query();
                            }
                        }
                        KeyCode::Tab => {
                            let visible: Vec<usize> = (0..app.grouped_results.len())
                                .filter(|i| !app.collapsed.contains(i))
                                .collect();
                            if let Some(pos) = visible.iter().position(|&gi| gi == app.selected_group) {
                                if pos + 1 < visible.len() {
                                    app.selected_group = visible[pos + 1];
                                    app.selected_item = 0;
                                } else if !visible.is_empty() {
                                    app.selected_group = visible[0];
                                    app.selected_item = 0;
                                }
                            }
                            app.update_list_state();
                        }
                        _ => {}
                    }
                }
            }
        }

        if app.pending_search && app.last_search.elapsed() >= app.debounce_duration {
            app.search_version += 1;
            app.execute_search();
            app.last_search = Instant::now();
            app.pending_search = false;
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    render_search_box(f, app, layout[0]);
    render_results(f, app, layout[1]);
    render_status_bar(f, app, layout[2]);
}

fn render_search_box(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Omnique ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = Text::from(Line::from(vec![
        Span::styled("> ", Style::default().fg(app.theme.primary)),
        Span::styled(&app.query, Style::default().fg(app.theme.text)),
    ]));

    let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
    f.render_widget(paragraph, inner);

    let cursor_x = inner.x + 2 + app.cursor_position as u16;
    let cursor_y = inner.y;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn render_results(f: &mut Frame, app: &mut App, area: Rect) {
    if app.grouped_results.is_empty() {
        let text = if app.query.is_empty() {
            Text::from(Line::from(Span::styled(
                "Type to search across files, git history, shell history...",
                Style::default().fg(app.theme.text_dim),
            )))
        } else {
            Text::from(Line::from(Span::styled(
                "No results found.",
                Style::default().fg(app.theme.text_dim),
            )))
        };

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let mut items: Vec<ListItem<'_>> = Vec::new();

    for (gi, group) in app.grouped_results.iter().enumerate() {
        let is_selected_group = gi == app.selected_group;
        let is_collapsed = app.collapsed.contains(&gi);

        let collapse_indicator = if is_collapsed { " ▶" } else { " ▼" };
        let header_text = format!(
            " {} {} ({} entries){}",
            group.icon, group.name, group.results.len(), collapse_indicator
        );

        let header_style = if is_selected_group {
            Style::default()
                .fg(app.theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.primary)
        };

        items.push(ListItem::new(Line::from(Span::styled(
            header_text,
            header_style,
        ))));

        if is_collapsed {
            continue;
        }

        for (ri, result) in group.results.iter().enumerate() {
            let is_selected = is_selected_group && ri == app.selected_item;

            let style = if is_selected {
                Style::default()
                    .bg(app.theme.selection)
                    .fg(app.theme.text)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.text)
            };

            let subtitle_style = if is_selected {
                Style::default()
                    .bg(app.theme.selection)
                    .fg(app.theme.text_dim)
            } else {
                Style::default().fg(app.theme.text_dim)
            };

            let line = Line::from(vec![
                Span::styled("  ", style),
                Span::styled(&result.title, style),
            ]);

            items.push(ListItem::new(line));

            if let Some(sub) = &result.subtitle {
                let sub_line = Line::from(vec![
                    Span::styled("    ", subtitle_style),
                    Span::styled(sub, subtitle_style),
                ]);
                items.push(ListItem::new(sub_line));
            }
        }
    }

    let flat_index = app.selected_flat_index();
    let list_state = &mut app.list_state;
    *list_state.selected_mut() = Some(flat_index);

    let list = List::new(items).block(Block::default().borders(Borders::TOP));

    f.render_stateful_widget(list, area, list_state);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let total: usize = app
        .grouped_results
        .iter()
        .map(|g| g.results.len())
        .sum();

    let status = format!(" {} results in {} groups | max={} ^Up/^Dn", total, app.grouped_results.len(), app.max_results);

    let text = Text::from(Line::from(Span::styled(
        status,
        Style::default().fg(app.theme.text_dim),
    )));

    let paragraph = Paragraph::new(text);
    f.render_widget(paragraph, area);
}
