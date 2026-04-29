use std::{io, time::Duration};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
};

use crate::app::App;

pub fn run(app: &mut App) -> Result<()> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to create terminal backend")?;
    let loop_result = event_loop(&mut terminal, app);

    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("failed to leave alternate screen")?;
    terminal.show_cursor().context("failed to show cursor")?;

    loop_result
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    while !app.should_quit {
        terminal
            .draw(|frame| render(frame, app))
            .context("failed to draw terminal frame")?;

        if !event::poll(Duration::from_millis(200)).context("failed to poll events")? {
            continue;
        }

        let Event::Key(key) = event::read().context("failed to read event")? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Tab | KeyCode::Char('l') => app.next_workspace(),
            KeyCode::BackTab | KeyCode::Char('h') => app.previous_workspace(),
            KeyCode::Down | KeyCode::Char('j') => app.move_selection_down(),
            KeyCode::Up | KeyCode::Char('k') => app.move_selection_up(),
            KeyCode::Char(' ') => app.toggle_selected(),
            KeyCode::Char('a') => app.toggle_all(),
            KeyCode::Enter => {
                let updated = app.apply_current_workspace_updates()?;
                if updated == 0 {
                    app.status = String::from("No packages selected");
                } else {
                    let name = app.current_workspace().tab_name.clone();
                    app.status = format!("Updated {updated} package(s) in {name}");
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let titles: Vec<Line<'_>> = app
        .workspaces
        .iter()
        .map(|workspace| Line::from(workspace.tab_name.clone()))
        .collect();
    let tabs = Tabs::new(titles)
        .select(app.active_workspace)
        .block(Block::default().borders(Borders::ALL).title("Workspaces"))
        .highlight_style(
            Style::default()
                .fg(Color::Rgb(152, 195, 121))
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, chunks[0]);

    let ws = app.current_workspace();
    if ws.entries.is_empty() {
        let empty = Paragraph::new("No newer versions found in this workspace.")
            .block(Block::default().borders(Borders::ALL).title("Packages"))
            .style(Style::default().fg(Color::Rgb(171, 178, 191)));
        frame.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem<'_>> = ws
            .entries
            .iter()
            .map(|entry| {
                let marker = if entry.selected { "[x]" } else { "[ ]" };
                let line = format!(
                    "{marker} {:<28} {:<15} -> {:<15} ({})",
                    entry.name,
                    entry.current_version,
                    entry.latest_version,
                    entry.section.label()
                );
                ListItem::new(line)
            })
            .collect();
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Packages"))
            .highlight_style(
                Style::default()
                    .fg(Color::Rgb(152, 195, 121))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(
            ws.selected_index.min(ws.entries.len().saturating_sub(1)),
        ));
        frame.render_stateful_widget(list, chunks[1], &mut state);
    }

    let help = Paragraph::new(vec![
        Line::from(format!("Status: {}", app.status)),
        format_keymap_line(),
    ])
    .style(Style::default().fg(Color::Rgb(171, 178, 191)));
    frame.render_widget(help, chunks[2]);
}

fn format_keymap_line() -> Line<'static> {
    let mut spans = Vec::new();
    for (index, (key, description)) in [
        ("Tab/l", "next ws"),
        ("Shift+Tab/h", "prev ws"),
        ("Up/k", "up"),
        ("Down/j", "down"),
        ("Space", "toggle"),
        ("a", "all"),
        ("Enter", "apply"),
        ("q", "quit"),
    ]
    .iter()
    .enumerate()
    {
        if index > 0 {
            spans.push(Span::raw(" | "));
        }
        spans.push(Span::styled(
            *key,
            Style::default().add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(": "));
        spans.push(Span::raw(*description));
    }
    Line::from(spans)
}
