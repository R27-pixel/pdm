// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use pdm::app::AppAction;
use pdm::app::{App, CurrentScreen};
use pdm::components::metrics::P2PoolMetrics;
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
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run App
    let mut app = App::new();
    //  A shared thread-safe String for the API URL
    let api_url = Arc::new(Mutex::new(Some(
        "http://127.0.0.1:46884/metrics".to_string(),
    )));
    let api_url_clone = Arc::clone(&api_url);

    //  A channel to send metrics from the background task to the UI
    let (tx, mut rx) = mpsc::unbounded_channel::<P2PoolMetrics>();

    //  Spawn the background fetcher task
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            // Check if we have a URL yet
            let url = { api_url_clone.lock().await.clone() };

            if let Some(u) = url {
                // Fetch the metrics!
                if let Ok(resp) = client.get(&u).send().await {
                    if let Ok(text) = resp.text().await {
                        let metrics = P2PoolMetrics::parse_prometheus(&text);
                        // Send them to the UI thread
                        let _ = tx.send(metrics);
                    }
                }
            }
            // Wait 2 seconds before polling again
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    // We use poll() with a 250ms timeout. If no key is pressed, we return None
    // so the loop can continue and check for new metrics.
    let res = run_app(&mut terminal, &mut app, api_url, &mut rx, |_| {
        if event::poll(Duration::from_millis(250))? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    })
    .await;

    // Restore Terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app<B: Backend, F>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    api_url: Arc<Mutex<Option<String>>>,
    rx: &mut mpsc::UnboundedReceiver<P2PoolMetrics>,
    mut event_handler: F,
) -> Result<()>
where
    F: FnMut(&mut App) -> Result<Option<Event>>,
{
    loop {
        // try_recv() reads from the channel instantly without blocking
        while let Ok(metrics) = rx.try_recv() {
            app.node_metrics = Some(metrics);
        }
        terminal.draw(|f| ui::ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Hard exit (always allowed)
            if key.code == KeyCode::Char('q')
                || (key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c'))
            {
                return Ok(());
            }

            let action = match app.current_screen {
                CurrentScreen::FileExplorer => app.explorer.handle_input(key),

                _ => match key.code {
                    KeyCode::Char('q') => AppAction::Quit,

                    KeyCode::Enter => {
                        if matches!(
                            app.current_screen,
                            CurrentScreen::BitcoinConfig | CurrentScreen::P2PoolConfig
                        ) {
                            AppAction::OpenExplorer(app.current_screen.clone())
                        } else {
                            AppAction::None
                        }
                    }

                    KeyCode::Down => {
                        if app.sidebar_index < 3 {
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
                },
            };

            if handle_action_with_url(action, app, &api_url).await? {
                return Ok(());
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
                        if let Ok(entries) = parse_p2pool_config(&path) {
                            app.p2pool_data = entries;
                        }
                        app.current_screen = CurrentScreen::P2PoolConfig;
                    }
                    CurrentScreen::BitcoinConfig => {
                        app.bitcoin_conf_path = Some(path.clone());
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

async fn handle_action_with_url(
    action: AppAction,
    app: &mut App,
    api_url: &Arc<Mutex<Option<String>>>,
) -> anyhow::Result<bool> {
    //  Run the normal action FIRST so the file is parsed
    let should_quit = handle_action(action.clone(), app)?;

    //  If the user manually loaded a config file, override the sensible default
    if let AppAction::FileSelected(_) = action {
        if app.current_screen == CurrentScreen::P2PoolConfig {
            if let Some(config) = &app.p2pool_conf_path {
                // dynamic override:
                // let host = &config.api_host;
                // let port = config.api_port;

                let host = "127.0.0.1";
                let port = 46884;
                let dynamic_url = format!("http://{}:{}/metrics", host, port);
                *api_url.lock().await = Some(dynamic_url);
            }
        }
    }

    Ok(should_quit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdm::components::metrics::P2PoolMetrics;
    use ratatui::backend::TestBackend;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio::sync::mpsc;

    #[test]
    fn test_app_integration_smoke_test() {
        let backend = TestBackend::new(80, 25);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        // Initial render
        terminal.draw(|f| ui::ui(f, &mut app)).unwrap();
        insta::assert_debug_snapshot!("home_screen", terminal.backend());

        // Simulate sidebar move
        app.sidebar_index = 1;
        app.toggle_menu();

        terminal.draw(|f| ui::ui(f, &mut app)).unwrap();
        insta::assert_debug_snapshot!("menu_toggled", terminal.backend());

        assert_eq!(app.current_screen, CurrentScreen::BitcoinConfig);
    }

    #[test]
    fn test_file_explorer_flow_state_only() {
        let backend = TestBackend::new(80, 25);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        // Navigate to Bitcoin config
        app.sidebar_index = 1;
        app.toggle_menu();
        assert_eq!(app.current_screen, CurrentScreen::BitcoinConfig);

        // Open explorer
        handle_action(
            AppAction::OpenExplorer(CurrentScreen::BitcoinConfig),
            &mut app,
        )
        .unwrap();

        assert_eq!(app.current_screen, CurrentScreen::FileExplorer);

        // Close explorer
        handle_action(AppAction::CloseModal, &mut app).unwrap();
        assert_eq!(app.current_screen, CurrentScreen::BitcoinConfig);

        terminal.draw(|f| ui::ui(f, &mut app)).unwrap();
    }

    #[test]
    fn test_file_explorer_wrap_and_select_sets_config() {
        use crossterm::event::KeyEvent;
        use std::env::temp_dir;
        use std::fs::{File, create_dir_all};
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

        handle_action(
            AppAction::OpenExplorer(CurrentScreen::BitcoinConfig),
            &mut app,
        )
        .unwrap();

        // Move selection DOWN to the actual file (skip "..")
        app.explorer
            .handle_input(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
        let action = app.explorer.handle_input(crossterm::event::KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::empty(),
        ));

        handle_action(action, &mut app).unwrap();

        assert_eq!(app.bitcoin_conf_path, Some(file_path));

        terminal.draw(|f| ui::ui(f, &mut app)).unwrap();
    }

    #[test]
    fn app_action_open_explorer_sets_state() {
        let mut app = App::new();

        let exited = handle_action(
            AppAction::OpenExplorer(CurrentScreen::BitcoinConfig),
            &mut app,
        )
        .unwrap();

        assert!(!exited);
        assert_eq!(app.current_screen, CurrentScreen::FileExplorer);
        assert_eq!(app.explorer_trigger, Some(CurrentScreen::BitcoinConfig));
    }

    #[test]
    fn app_action_close_modal_returns_to_trigger_screen() {
        let mut app = App::new();

        app.explorer_trigger = Some(CurrentScreen::BitcoinConfig);
        app.current_screen = CurrentScreen::FileExplorer;

        let exited = handle_action(AppAction::CloseModal, &mut app).unwrap();

        assert!(!exited);
        assert_eq!(app.current_screen, CurrentScreen::BitcoinConfig);
        assert!(app.explorer_trigger.is_none());
    }

    #[test]
    fn app_action_quit_requests_exit() {
        let mut app = App::new();

        let exited = handle_action(AppAction::Quit, &mut app).unwrap();

        assert!(exited);
    }

    #[tokio::test]
    async fn test_handle_action_with_url_updates_mutex() {
        let mut app = App::new();
        // Start with a test default URL
        let api_url = Arc::new(Mutex::new(Some(
            "http://default-host:9999/metrics".to_string(),
        )));

        // Setup App state: Simulate the user opening the Explorer from P2PoolConfig
        app.explorer_trigger = Some(CurrentScreen::P2PoolConfig);
        app.current_screen = CurrentScreen::FileExplorer;

        // Simulate the user selecting a file
        let fake_path = std::path::PathBuf::from("test_p2pool.toml");
        let action = AppAction::FileSelected(fake_path);

        // Execute the wrapper function
        let _ = handle_action_with_url(action, &mut app, &api_url)
            .await
            .unwrap();

        // Check that the App navigated back to the config screen
        assert_eq!(app.current_screen, CurrentScreen::P2PoolConfig);

        // Check that the Mutex was updated with our new dynamic URL
        // (Currently hardcoded to 127.0.0.1:46884 in our logic until you map the struct fields)
        let updated_url = api_url.lock().await.clone();
        assert_eq!(
            updated_url,
            Some("http://127.0.0.1:46884/metrics".to_string())
        );
    }

    #[test]
    fn test_app_receives_metrics_from_channel() {
        let mut app = App::new();
        let (tx, mut rx) = mpsc::unbounded_channel::<P2PoolMetrics>();

        // Verify app starts with no metrics
        assert!(app.node_metrics.is_none());

        // Create fake metrics simulating the background task
        let mut fake_metrics = P2PoolMetrics::default();
        fake_metrics.shares_accepted = 999;
        fake_metrics.pool_difficulty = 5000;

        // Send down the channel
        tx.send(fake_metrics).unwrap();

        // Simulate the top of the run_app() loop
        if let Ok(metrics) = rx.try_recv() {
            app.node_metrics = Some(metrics);
        }

        // Verify the App absorbed the data correctly
        assert!(app.node_metrics.is_some());
        let saved_metrics = app.node_metrics.unwrap();
        assert_eq!(saved_metrics.shares_accepted, 999);
        assert_eq!(saved_metrics.pool_difficulty, 5000);
    }
}
