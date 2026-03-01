mod app;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::time::Duration;

pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;
    let res = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn run_loop<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    const PAGE_STEP: usize = 10;
    let mut last_refresh = std::time::Instant::now();

    loop {
        if app.auto_refresh
            && last_refresh.elapsed() >= Duration::from_millis(app.auto_refresh_interval_ms)
        {
            app.refresh()?;
            last_refresh = std::time::Instant::now();
        }

        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.inspect_mode {
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => app.toggle_inspect(),
                            _ => {}
                        }
                    } else if app.filter_mode {
                        match key.code {
                            KeyCode::Esc => app.cancel_filter(),
                            KeyCode::Enter => app.apply_filter(),
                            KeyCode::Backspace => app.pop_filter_char(),
                            KeyCode::Char(c) => app.push_filter_char(c),
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('r') => {
                                app.refresh()?;
                                last_refresh = std::time::Instant::now();
                            }
                            KeyCode::Up => app.prev(),
                            KeyCode::Down => app.next(),
                            KeyCode::Char('k') => app.prev(),
                            KeyCode::Char('j') => app.next(),
                            KeyCode::Home => app.first(),
                            KeyCode::End => app.last(),
                            KeyCode::PageUp => app.page_up(PAGE_STEP),
                            KeyCode::PageDown => app.page_down(PAGE_STEP),
                            KeyCode::Char('K') => app.kill_selected()?,
                            KeyCode::Char('B') => app.kill_selected_batch()?,
                            KeyCode::Char(' ') => app.toggle_select_selected(),
                            KeyCode::Char('x') => app.clear_selection(),
                            KeyCode::Char('a') => app.toggle_all(),
                            KeyCode::Char('/') => app.begin_filter(),
                            KeyCode::Char('s') => app.cycle_sort(),
                            KeyCode::Char('d') => app.toggle_sort_direction(),
                            KeyCode::Char('p') => app.cycle_protocol_filter(),
                            KeyCode::Char('w') => app.cycle_state_filter(),
                            KeyCode::Char('t') => {
                                app.toggle_auto_refresh();
                                last_refresh = std::time::Instant::now();
                            }
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                app.increase_refresh_interval()
                            }
                            KeyCode::Char('-') => app.decrease_refresh_interval(),
                            KeyCode::Char('g') => app.toggle_group(),
                            KeyCode::Enter => app.toggle_inspect(),
                            KeyCode::Char('c') => app.copy_selected(),
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
