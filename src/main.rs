// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use pdm::app::{App, CurrentScreen};
use pdm::ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::Backend, backend::CrosstermBackend};
use std::io;

fn main() -> Result<()> {
    //  Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    //  Run App
    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app, event::read);

    //  Restore Terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

// Accept any Backend and an Event Provider Closure
fn run_app<B: Backend, F>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mut event_provider: F,
) -> io::Result<()>
where
    F: FnMut() -> io::Result<Event>,
{
    loop {
        terminal.draw(|f| ui::ui(f, app))?;

        // We check the event from our provider
        if let Event::Key(key) = event_provider()?
            && key.kind == KeyEventKind::Press
        {
            if key.code == KeyCode::Char('q') {
                return Ok(());
            }
            match app.current_screen {
                // File Explorer Modal
                CurrentScreen::FileExplorer => match key.code {
                    KeyCode::Up => app.explorer.previous(),
                    KeyCode::Down => app.explorer.next(),
                    KeyCode::Esc => app.toggle_menu(), // Cancel
                    KeyCode::Enter => {
                        if let Some(path) = app.explorer.select() {
                            // File Selected!
                            app.bitcoin_conf_path = Some(path);
                            app.toggle_menu(); // Go back to main screen
                        }
                    }
                    _ => {}
                },

                // Standard Navigation
                _ => match key.code {
                    KeyCode::Up => {
                        if app.sidebar_index > 0 {
                            app.sidebar_index -= 1;
                            app.toggle_menu();
                        }
                    }
                    KeyCode::Down => {
                        if app.sidebar_index < 1 {
                            app.sidebar_index += 1;
                            app.toggle_menu();
                        }
                    }
                    KeyCode::Enter => {
                        // If we are on "Bitcoin Config", open the explorer
                        if app.current_screen == CurrentScreen::BitcoinConfig {
                            app.current_screen = CurrentScreen::FileExplorer;
                        }
                    }
                    _ => {}
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventState, KeyModifiers};
    use ratatui::backend::TestBackend;

    #[test]
    fn test_app_integration_smoke_test() {
        let backend = TestBackend::new(80, 25);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        let mut step = 0;

        let event_provider = || {
            step += 1;
            match step {
                1 => Ok(Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                })),
                2 => Ok(Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                })),
                _ => panic!("Should have exited"),
            }
        };

        // First frame
        terminal.draw(|f| ui::ui(f, &mut app)).unwrap();
        insta::assert_debug_snapshot!("home_screen", terminal.backend());

        // Run app (process events + redraws)
        let res = run_app(&mut terminal, &mut app, event_provider);
        assert!(res.is_ok());

        // Final frame after DOWN
        insta::assert_debug_snapshot!("menu_toggled", terminal.backend());

        assert_eq!(app.sidebar_index, 1);
    }
}
