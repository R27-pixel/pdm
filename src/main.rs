// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use pdm::app::{App, CurrentScreen};
use pdm::components::metrics::BitcoinMetrics;
use pdm::config::parse_config;
use pdm::ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::Backend, backend::CrosstermBackend};
use reqwest::Client;
use std::io;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    //  Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    //  Create channel for Bitcoin metrics
    let (btc_tx, btc_rx) = mpsc::channel::<BitcoinMetrics>(16);

    //  Shared RPC parameters (URL, user, pass)
    let btc_rpc_params = Arc::new(Mutex::new(None::<(String, String, String)>));
    let btc_rpc_params_task = btc_rpc_params.clone();

    //  TESTING: Initialize with hardcoded test URL (comment out for production)
    //  This allows testing the connection before loading a bitcoin.conf file
    #[allow(unreachable_code)]
    {
        // Uncomment the line below to enable test mode
        *btc_rpc_params.lock().unwrap() = Some((
            "http://127.0.0.1:38332".to_string(),
            "p2pool".to_string(),
            "p2pool".to_string(),
        ));
    }

    //  Spawn Bitcoin background worker task
    tokio::spawn(async move {
        let client = Client::new();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

        loop {
            interval.tick().await;

            // Check if we have RPC parameters
            let params = btc_rpc_params_task.lock().unwrap().clone();

            if let Some((url, user, pass)) = params {
                // Try to fetch metrics
                match BitcoinMetrics::fetch(&client, &url, &user, &pass).await {
                    Ok(metrics) => {
                        // Send metrics through channel (ignore if receiver is gone)
                        let _ = btc_tx.send(metrics).await;
                    }
                    Err(_) => {
                        // Connection error - will retry next interval
                    }
                }
            }
        }
    });

    //  Run App
    let mut app = App::new();
    let res = run_app(
        &mut terminal,
        &mut app,
        btc_rx,
        btc_rpc_params,
        |_app: &mut App| event::read(),
    )
    .await;

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
async fn run_app<B: Backend, F>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mut btc_rx: mpsc::Receiver<BitcoinMetrics>,
    btc_rpc_params: Arc<Mutex<Option<(String, String, String)>>>,
    mut event_provider: F,
) -> io::Result<()>
where
    F: FnMut(&mut App) -> io::Result<Event>,
{
    loop {
        terminal.draw(|f| ui::ui(f, app))?;

        // Check for metrics updates from the Bitcoin background worker
        while let Ok(metrics) = btc_rx.try_recv() {
            app.bitcoin_metrics = Some(metrics);
        }

        // We check the event from our provider
        if let Event::Key(key) = event_provider(app)?
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
                            app.bitcoin_conf_path = Some(path.clone());
                            // Parse and save the typed struct
                            if let Ok((bitcoin_config, _)) = parse_config(&path) {
                                app.bitcoin_config = Some(bitcoin_config.clone());

                                // Extract RPC credentials and populate the shared params
                                let user = bitcoin_config.rpc.rpcuser.clone().unwrap_or_default();
                                let pass =
                                    bitcoin_config.rpc.rpcpassword.clone().unwrap_or_default();
                                let port = bitcoin_config.rpc.rpcport.unwrap_or(8332); // mainnet default
                                let rpc_url = format!("http://127.0.0.1:{}", port);

                                // Update the shared RPC parameters for the background task
                                *btc_rpc_params.lock().unwrap() = Some((rpc_url, user, pass));
                            }
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
                        if app.sidebar_index < 4 {
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

// AppAction enum to track file selections
#[derive(Clone, Debug)]
pub enum AppAction {
    FileSelected(std::path::PathBuf),
}

// Handle actions with URL/param updates
fn handle_action(action: AppAction, app: &mut App) -> anyhow::Result<bool> {
    match action {
        AppAction::FileSelected(path) => {
            app.bitcoin_conf_path = Some(path.clone());
            // Parse and save the typed struct
            if let Ok((bitcoin_config, _)) = parse_config(&path) {
                app.bitcoin_config = Some(bitcoin_config);
            }
            app.toggle_menu(); // Go back to main screen
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventState, KeyModifiers};
    use ratatui::backend::TestBackend;

    #[tokio::test]
    async fn test_app_integration_smoke_test() {
        let backend = TestBackend::new(80, 25);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        let mut step = 0;

        let event_provider = |_app: &mut App| {
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

        // Create channel for metrics
        let (_btc_tx, btc_rx) = mpsc::channel::<BitcoinMetrics>(16);
        let btc_rpc_params = Arc::new(Mutex::new(None::<(String, String, String)>));

        // First frame
        terminal.draw(|f| ui::ui(f, &mut app)).unwrap();
        insta::assert_debug_snapshot!("home_screen", terminal.backend());

        // Run app (process events + redraws)
        let res = run_app(
            &mut terminal,
            &mut app,
            btc_rx,
            btc_rpc_params,
            event_provider,
        )
        .await;
        assert!(res.is_ok());

        // Final frame after DOWN
        insta::assert_debug_snapshot!("menu_toggled", terminal.backend());

        assert_eq!(app.sidebar_index, 1);
    }

    #[tokio::test]
    async fn test_file_explorer_flow() {
        // Setup
        let backend = TestBackend::new(80, 25);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        // Define Steps
        let mut step = 0;
        let event_provider = |app: &mut App| {
            step += 1;
            match step {
                1 => {
                    // Start at Home.
                    // Action: Move DOWN to "Bitcoin Config"
                    Ok(Event::Key(KeyEvent::new(
                        KeyCode::Down,
                        KeyModifiers::empty(),
                    )))
                }
                2 => {
                    // Action: Press ENTER to open File Explorer
                    Ok(Event::Key(KeyEvent::new(
                        KeyCode::Enter,
                        KeyModifiers::empty(),
                    )))
                }
                3 => {
                    // WE ARE NOW IN FILE EXPLORER
                    // Assertion: Check internal state (safer than snapshotting dynamic file lists)
                    assert_eq!(
                        app.current_screen,
                        CurrentScreen::FileExplorer,
                        "Should have switched to File Explorer"
                    );

                    // Action: Move DOWN (Navigate file list)
                    Ok(Event::Key(KeyEvent::new(
                        KeyCode::Down,
                        KeyModifiers::empty(),
                    )))
                }
                4 => {
                    // Assertion: Check that selection moved
                    assert_eq!(
                        app.explorer.selected_index, 1,
                        "Should have selected the second file"
                    );

                    // Action: Press ESC to Cancel/Close
                    Ok(Event::Key(KeyEvent::new(
                        KeyCode::Esc,
                        KeyModifiers::empty(),
                    )))
                }
                5 => {
                    // BACK TO SIDEBAR
                    assert_eq!(
                        app.current_screen,
                        CurrentScreen::BitcoinConfig,
                        "Should have returned to Sidebar"
                    );

                    // Action: Quit
                    Ok(Event::Key(KeyEvent::new(
                        KeyCode::Char('q'),
                        KeyModifiers::empty(),
                    )))
                }
                _ => panic!("Step {} not handled", step),
            }
        };

        // Create channel for metrics
        let (_btc_tx, btc_rx) = mpsc::channel::<BitcoinMetrics>(16);
        let btc_rpc_params = Arc::new(Mutex::new(None::<(String, String, String)>));

        // Run
        let res = run_app(
            &mut terminal,
            &mut app,
            btc_rx,
            btc_rpc_params,
            event_provider,
        )
        .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_file_explorer_wrap_and_select_sets_config() {
        use std::env::temp_dir;
        use std::fs::{File, create_dir_all};

        // Setup a temporary filesystem sandbox
        let base = temp_dir().join("pdm_select_test");
        let _ = std::fs::remove_dir_all(&base);
        create_dir_all(&base).unwrap();
        let file_path = base.join("bitcoin.conf");
        File::create(&file_path).unwrap();

        let backend = TestBackend::new(80, 25);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.explorer.current_dir = base.clone();
        app.explorer.load_directory();

        let mut step = 0;

        let event_provider = |app: &mut App| {
            step += 1;
            match step {
                1 => Ok(Event::Key(KeyEvent::new(
                    KeyCode::Down,
                    KeyModifiers::empty(),
                ))), // move to bitcoin config
                2 => Ok(Event::Key(KeyEvent::new(
                    KeyCode::Enter,
                    KeyModifiers::empty(),
                ))), // open explorer
                3 => Ok(Event::Key(KeyEvent::new(
                    KeyCode::Up,
                    KeyModifiers::empty(),
                ))), // force wrap-around
                4 => Ok(Event::Key(KeyEvent::new(
                    KeyCode::Enter,
                    KeyModifiers::empty(),
                ))), // select file
                5 => Ok(Event::Key(KeyEvent::new(
                    KeyCode::Char('q'),
                    KeyModifiers::empty(),
                ))),
                _ => panic!("unexpected"),
            }
        };

        // Create channel for metrics
        let (_btc_tx, btc_rx) = mpsc::channel::<BitcoinMetrics>(16);
        let btc_rpc_params = Arc::new(Mutex::new(None::<(String, String, String)>));

        let res = run_app(
            &mut terminal,
            &mut app,
            btc_rx,
            btc_rpc_params,
            event_provider,
        )
        .await;
        assert!(res.is_ok());

        assert_eq!(app.bitcoin_conf_path, Some(file_path));
    }
}
