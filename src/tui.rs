use crate::app::{App, Mode, state_summary};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};
use std::{io, time::Duration};

pub fn run() -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    result
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    while !app.should_quit() {
        terminal.draw(|frame| render(frame, app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                handle_key(app, key);
            }
        }
    }

    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.quit();
        return;
    }

    match app.mode() {
        Mode::Normal => handle_normal_key(app, key.code),
        Mode::Insert | Mode::Edit => handle_input_key(app, key.code),
    }
}

fn handle_normal_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Char('j') | KeyCode::Down => app.move_next(),
        KeyCode::Char('k') | KeyCode::Up => app.move_previous(),
        KeyCode::Char('i') => app.start_insert(),
        KeyCode::Char('e') => app.start_edit(),
        KeyCode::Char('d') => app.delete_selected(),
        KeyCode::Char('c') => app.clear(),
        _ => {}
    }
}

fn handle_input_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter => app.submit_input(),
        KeyCode::Esc => app.cancel_input(),
        KeyCode::Backspace => app.backspace_input(),
        KeyCode::Char(ch) => app.push_input(ch),
        _ => {}
    }
}

fn render(frame: &mut Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_entries(frame, app, chunks[0]);
    render_input(frame, app, chunks[1]);
    render_status(frame, app, chunks[2]);
}

fn render_entries(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let entries = app.entries();
    let rows = entries.iter().map(|entry| {
        let name = entry.name.as_deref().unwrap_or("");
        Row::new([
            Cell::from(entry.id.to_string()),
            Cell::from(name.to_string()),
            Cell::from(entry.source.clone()),
            Cell::from(state_summary(&entry.state)),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(18),
            Constraint::Percentage(42),
            Constraint::Percentage(36),
        ],
    )
    .header(
        Row::new(["ID", "Name", "Source", "Result"]).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(Block::default().title("dagcal").borders(Borders::ALL))
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("> ");

    let mut state = TableState::default();
    if !entries.is_empty() {
        state.select(Some(app.selected()));
    }
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_input(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let title = match app.mode() {
        Mode::Normal => "Input",
        Mode::Insert => "Insert",
        Mode::Edit => "Edit",
    };
    let text = if app.mode() == Mode::Normal {
        Line::from(vec![
            Span::styled("i", Style::default().fg(Color::Yellow)),
            Span::raw(" insert  "),
            Span::styled("e", Style::default().fg(Color::Yellow)),
            Span::raw(" edit  "),
            Span::styled("d", Style::default().fg(Color::Yellow)),
            Span::raw(" delete  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit"),
        ])
    } else {
        Line::from(app.input().to_string())
    };

    let paragraph = Paragraph::new(text)
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_status(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let paragraph = Paragraph::new(app.status().to_string())
        .block(Block::default().title("Status").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
