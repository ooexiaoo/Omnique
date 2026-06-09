use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::backends::SearchResult;
use crate::search::{GroupedResults, SearchEngine};

const POLL_DURATION: Duration = Duration::from_millis(50);

pub struct App {
    query: String,
    cursor_position: usize,
    grouped_results: Vec<GroupedResults>,
    selected_group: usize,
    selected_item: usize,
    search_engine: SearchEngine,
    last_search: Instant,
    debounce_duration: Duration,
    search_version: usize,
    pending_search: bool,
}

impl App {
    pub fn new(search_engine: SearchEngine) -> Self {
        Self {
            query: String::new(),
            cursor_position: 0,
            grouped_results: Vec::new(),
            selected_group: 0,
            selected_item: 0,
            search_engine,
            last_search: Instant::now(),
            debounce_duration: Duration::from_millis(100),
            search_version: 0,
            pending_search: false,
        }
    }

    fn trigger_search(&mut self) {
        self.pending_search = true;
    }

    fn execute_search(&mut self) {
        let query = self.query.clone();
        let version = self.search_version;

        let grouped = self.search_engine.search(&query, 10);

        if version == self.search_version {
            self.grouped_results = grouped;
            self.selected_group = 0;
            self.selected_item = 0;
        }
    }

    fn open_selected(&self) {
        if let Some(result) = self.selected_result() {
            match &result.kind {
                crate::backends::ResultKind::File { path, line, .. } => {
                    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
                    let _ = std::process::Command::new("sh")
                        .args(["-c", &format!("{} +{} {}", editor, line, path)])
                        .spawn();
                }
                crate::backends::ResultKind::GitCommit { hash, .. } => {
                    let _ = std::process::Command::new("sh")
                        .args(["-c", &format!("git show {}", hash)])
                        .spawn();
                }
                crate::backends::ResultKind::ShellCommand { command, .. } => {
                    let _ = std::process::Command::new("sh")
                        .args(["-c", &format!("echo {} | xclip -selection clipboard", command)])
                        .spawn();
                }
            }
        }
    }

    fn selected_result(&self) -> Option<&SearchResult> {
        self.grouped_results
            .get(self.selected_group)
            .and_then(|g| g.results.get(self.selected_item))
    }

    fn move_selection(&mut self, delta: isize) {
        let total_items: usize = self
            .grouped_results
            .iter()
            .map(|g| g.results.len())
            .sum();

        if total_items == 0 {
            return;
        }

        let current_idx: usize = self
            .grouped_results
            .iter()
            .take(self.selected_group)
            .map(|g| g.results.len())
            .sum::<usize>()
            + self.selected_item;

        let new_idx = if delta < 0 {
            current_idx.saturating_sub(delta.unsigned_abs())
        } else {
            (current_idx + delta as usize).min(total_items - 1)
        };

        let mut remaining = new_idx;
        for (gi, group) in self.grouped_results.iter().enumerate() {
            if remaining < group.results.len() {
                self.selected_group = gi;
                self.selected_item = remaining;
                return;
            }
            remaining = remaining.saturating_sub(group.results.len());
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
        self.cursor_position = self.cursor_position.min(self.query.len().saturating_sub(1));
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
    }
}

pub fn run_tui(search_engine: SearchEngine) -> color_eyre::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;
    let mut app = App::new(search_engine);
    let res = run_app(terminal, &mut app);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    if let Err(e) = res {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

fn run_app(
    mut terminal: Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
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
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                            return Ok(());
                        }
                        KeyCode::Enter => {
                            app.open_selected();
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
                            if app.selected_group + 1 < app.grouped_results.len() {
                                app.selected_group += 1;
                                app.selected_item = 0;
                            } else if !app.grouped_results.is_empty() {
                                app.selected_group = 0;
                                app.selected_item = 0;
                            }
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

fn ui(f: &mut Frame, app: &App) {
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
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = Text::from(Line::from(vec![
        Span::raw("> "),
        Span::raw(&app.query),
    ]));

    let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
    f.render_widget(paragraph, inner);

    let cursor_x = inner.x + 2 + app.cursor_position as u16;
    let cursor_y = inner.y;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn render_results(f: &mut Frame, app: &App, area: Rect) {
    if app.grouped_results.is_empty() {
        let text = if app.query.is_empty() {
            Text::from(Line::from(Span::styled(
                "Type to search across files, git history, shell history...",
                Style::default().fg(Color::DarkGray),
            )))
        } else {
            Text::from(Line::from(Span::styled(
                "No results found.",
                Style::default().fg(Color::DarkGray),
            )))
        };

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let header_texts: Vec<String> = app
        .grouped_results
        .iter()
        .map(|g| format!(" {} {} ({} entries)", g.icon, g.name, g.results.len()))
        .collect();

    let mut items: Vec<ListItem<'_>> = Vec::new();

    for (gi, group) in app.grouped_results.iter().enumerate() {
        let is_selected_group = gi == app.selected_group;

        let header_style = if is_selected_group {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };

        items.push(ListItem::new(Line::from(Span::styled(
            &header_texts[gi],
            header_style,
        ))));

        for (ri, result) in group.results.iter().enumerate() {
            let is_selected = is_selected_group && ri == app.selected_item;

            let style = if is_selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let subtitle_style = if is_selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::Gray)
            } else {
                Style::default().fg(Color::Gray)
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

    let list = List::new(items)
        .block(Block::default().borders(Borders::TOP))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_widget(list, area);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let total: usize = app
        .grouped_results
        .iter()
        .map(|g| g.results.len())
        .sum();

    let status = format!(
        " [Tab: cycle groups] [Enter: open] [Esc: clear] [q: quit] | {} results in {} groups",
        total,
        app.grouped_results.len(),
    );

    let text = Text::from(Line::from(Span::styled(
        status,
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(text);
    f.render_widget(paragraph, area);
}
