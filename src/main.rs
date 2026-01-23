// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use pdm::app::AppAction;
use pdm::app::{App, CurrentScreen};
use pdm::config::parse_config as parse_bitcoin_config;
use pdm::p2poolv2_config_parser::parse_config as parse_p2pool_config;
use pdm::ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::Backend, backend::CrosstermBackend};
use std::io;

fn main() -> Result<()> {
    // Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run App
    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // Restore Terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                if key.code == KeyCode::Char('q')
                    || (key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c'))
                {
                    return Ok(());
                }

                if app.current_screen == CurrentScreen::FileExplorer {
                    let action = app.explorer.handle_input(key);
                    handle_action(action, app)?;
                    continue;
                }

                let action = if app.current_screen == CurrentScreen::FileExplorer {
                    app.explorer.handle_input(key)
                } else {
                    // Global Navigation (Sidebar & Opens Explorer)
                    match key.code {
                        // Enter: Opens Explorer if we are on a config screen
                        KeyCode::Enter => {
                            if app.current_screen == CurrentScreen::BitcoinConfig
                                || app.current_screen == CurrentScreen::P2PoolConfig
                            {
                                AppAction::OpenExplorer(app.current_screen.clone())
                            } else {
                                AppAction::None
                            }
                        }

                        // Sidebar Navigation
                        KeyCode::Down => {
                            if app.sidebar_index < 2 {
                                app.sidebar_index += 1;
                                AppAction::ToggleMenu
                            } else {
                                AppAction::None
                            }
                        }
                        KeyCode::Up => {
                            if app.sidebar_index > 0 {
                                app.sidebar_index -= 1;
                                AppAction::ToggleMenu
                            } else {
                                AppAction::None
                            }
                        }
                        _ => AppAction::None,
                    }
                };

                if handle_action(action, app)? {
                    return Ok(());
                }
            }
        }
    }
}

// Logic Handler
fn handle_action(action: AppAction, app: &mut App) -> Result<bool> {
    match action {
        AppAction::Quit => return Ok(true),

        AppAction::ToggleMenu => app.toggle_menu(),

        AppAction::OpenExplorer(trigger) => {
            app.explorer_trigger = Some(trigger);
            app.current_screen = CurrentScreen::FileExplorer;
        }

        AppAction::CloseModal => {
            if let Some(trigger) = &app.explorer_trigger {
                app.current_screen = trigger.clone();
            } else {
                app.toggle_menu();
            }
            app.explorer_trigger = None;
        }

        AppAction::FileSelected(path) => {
            if let Some(trigger) = &app.explorer_trigger {
                match trigger {
                    CurrentScreen::P2PoolConfig => {
                        app.p2pool_conf_path = Some(path.clone());
                        // DIRECT LOAD: Save to vector
                        if let Ok(entries) = parse_p2pool_config(&path) {
                            app.p2pool_data = entries;
                        }
                        app.current_screen = CurrentScreen::P2PoolConfig;
                    }
                    CurrentScreen::BitcoinConfig => {
                        app.bitcoin_conf_path = Some(path.clone());
                        // DIRECT LOAD: Save to vector
                        if let Ok(entries) = parse_bitcoin_config(&path) {
                            app.bitcoin_data = entries;
                        }
                        app.current_screen = CurrentScreen::BitcoinConfig;
                    }
                    _ => {}
                }
            }
            app.explorer_trigger = None;
        }

        AppAction::Navigate(screen) => {
            app.current_screen = screen;
        }

        AppAction::None => {}
    }
    Ok(false)
}
