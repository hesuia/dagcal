use crate::app::{App, Mode, expression_source, filter_label, kind_label, state_summary};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(5),
            Constraint::Length(completion_height(app)),
            Constraint::Length(5),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_entries(frame, app, chunks[0]);
    render_input(frame, app, chunks[1]);
    render_completions(frame, app, chunks[2]);
    render_details(frame, app, chunks[3]);
    render_status(frame, app, chunks[4]);
}

fn render_entries(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let entries = app.visible_entries();
    let rows = entries.iter().map(|entry| {
        let name = entry.name.as_deref().unwrap_or("");
        Row::new([
            Cell::from(entry.id.to_string()),
            Cell::from(name.to_string()),
            Cell::from(expression_source(entry)),
            Cell::from(state_summary(&entry.state)),
        ])
    });

    let title = if app.search_is_open() {
        format!(
            "dagcal | search: {} | filter: {}",
            app.search_query(),
            filter_label(app.entry_state_filter())
        )
    } else {
        "dagcal".to_string()
    };

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
    .block(Block::default().title(title).borders(Borders::ALL))
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

    if app.entries().is_empty() || entries.is_empty() {
        let message = if app.entries().is_empty() {
            "No entries yet"
        } else {
            "No matching entries"
        };
        let paragraph = Paragraph::new(message)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default());
        frame.render_widget(
            paragraph,
            area.inner(ratatui::layout::Margin {
                horizontal: 2,
                vertical: 2,
            }),
        );
    }
}

fn render_input(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let title = match app.mode() {
        Mode::Normal => "Input",
        Mode::Insert => "Insert",
        Mode::Edit => "Edit",
        Mode::Search => "Search",
    };

    let text = match app.mode() {
        Mode::Normal => Text::from(vec![Line::from(vec![
            Span::styled("i", Style::default().fg(Color::Yellow)),
            Span::raw(" insert  "),
            Span::styled("e", Style::default().fg(Color::Yellow)),
            Span::raw(" edit  "),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(" search  "),
            Span::styled("p", Style::default().fg(Color::Yellow)),
            Span::raw(" use  "),
            Span::styled("R", Style::default().fg(Color::Yellow)),
            Span::raw(" recalc  "),
            Span::styled("A", Style::default().fg(Color::Yellow)),
            Span::raw(" recalc all  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit"),
        ])]),
        Mode::Search => Text::from(vec![
            Line::from(format!("Query: {}", app.search_query())),
            Line::from(format!(
                "Filter: {}    Tab: cycle filter    Esc/Enter: close",
                filter_label(app.entry_state_filter())
            )),
        ]),
        Mode::Insert | Mode::Edit => Text::from(vec![
            Line::from(app.input().to_string()),
            Line::from(format!("Resolved: {}", app.resolved_input())),
            Line::from(app.preview_summary()),
        ]),
    };

    let paragraph = Paragraph::new(text)
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_completions(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let lines = if app.completion_is_open() {
        let selected = app.selected_completion_index();
        app.completion_candidates()
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                let marker = if selected == Some(index) { "> " } else { "  " };
                let detail = candidate
                    .detail
                    .as_deref()
                    .map(|detail| format!("  {detail}"))
                    .unwrap_or_default();
                let result = candidate
                    .result
                    .as_deref()
                    .map(|result| format!("  {result}"))
                    .unwrap_or_default();
                Line::from(vec![
                    Span::styled(marker, Style::default().fg(Color::Yellow)),
                    Span::styled(kind_label(candidate.kind), Style::default().fg(Color::Cyan)),
                    Span::raw(format!("  {}{}{}", candidate.label, detail, result)),
                ])
            })
            .collect()
    } else {
        vec![Line::from("Completions: none")]
    };

    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::default().title("Completions").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_details(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let paragraph = Paragraph::new(app.selected_detail_text())
        .block(Block::default().title("Details").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_status(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let paragraph = Paragraph::new(format!(
        "{}    {}    {}",
        app.status(),
        app.entry_count_status_text(),
        app.history_status_text()
    ))
    .block(Block::default().title("Status").borders(Borders::ALL))
    .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn completion_height(app: &App) -> u16 {
    if app.completion_is_open() {
        app.completion_candidates().len() as u16 + 2
    } else {
        3
    }
}
