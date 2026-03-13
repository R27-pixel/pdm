// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod bitcoin;
mod file_explorer;
mod home;
mod p2poolv2;

use crate::app::{App, CurrentScreen};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};

// Render the sidebar and dispatch the active content pane for the current screen
pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(25), Constraint::Min(0)])
        .split(f.area());

    let items = vec![
        ListItem::new("Home"),
        ListItem::new("Bitcoin Config"),
        ListItem::new("P2Pool Config"),
    ];

    let mut state = ListState::default();
    state.select(Some(app.sidebar_index));

    let sidebar = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" PDM "))
        .highlight_style(Style::default().bg(Color::Gray).fg(Color::Black));

    f.render_stateful_widget(sidebar, chunks[0], &mut state);

    let main_area = chunks[1];

    match app.current_screen {
        CurrentScreen::Home => home::render(f, app, main_area),
        CurrentScreen::BitcoinConfig => bitcoin::render(f, app, main_area),
        CurrentScreen::P2PoolConfig => p2poolv2::render(f, app, main_area),
        CurrentScreen::FileExplorer => file_explorer::render(f, app, main_area),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_home_screen_render() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        insta::assert_debug_snapshot!(terminal.backend());
    }

    #[test]
    fn test_bitcoin_screen_render() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.sidebar_index = 1;
        app.toggle_menu();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        insta::assert_debug_snapshot!(terminal.backend());
    }

    #[test]
    fn test_p2pool_screen_render() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.sidebar_index = 2;
        app.toggle_menu();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        insta::assert_debug_snapshot!(terminal.backend());
    }
}
