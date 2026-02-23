// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::app::{App, CurrentScreen};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(25), // Sidebar
            Constraint::Min(0),     // Main Content
        ])
        .split(f.area());

    //  Sidebar
    let items = vec![
        ListItem::new("Home"),
        ListItem::new("Bitcoin Config"),
        ListItem::new("Bitcoin Status"),
    ];

    // Highlight the active one
    let mut state = ListState::default();
    state.select(Some(app.sidebar_index));

    let sidebar = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" PDM "))
        .highlight_style(Style::default().bg(Color::Gray).fg(Color::Black));

    f.render_stateful_widget(sidebar, chunks[0], &mut state);

    // Main Content
    let main_area = chunks[1];

    match app.current_screen {
        CurrentScreen::Home => {
            let config_status = match &app.bitcoin_conf_path {
                Some(p) => format!("Loaded: {:?}", p),
                None => "No config loaded".to_string(),
            };

            let text = format!(
                "Welcome to PDM.\n\n{}\n\n(Navigate to 'Bitcoin Config' to load)",
                config_status
            );
            let p = Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title(" Home "))
                .wrap(Wrap { trim: true });
            f.render_widget(p, main_area);
        }
        CurrentScreen::BitcoinConfig => {
            let p = Paragraph::new("Press [Enter] to select a bitcoin.conf file").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Bitcoin Config "),
            );
            f.render_widget(p, main_area);
        }
        CurrentScreen::BitcoinStatus => {
            if let Some(metrics) = &app.bitcoin_metrics {
                let sync_pct = metrics.verification_progress * 100.0;

                let text = format!(
                    " Bitcoin Core is ONLINE \n\n\
                    Network:     {}\n\
                    Blocks:      {}\n\
                    Headers:     {}\n\
                    Sync Status: {:.2}%\n\
                    Connections: {}",
                    metrics.chain.to_uppercase(),
                    metrics.blocks,
                    metrics.headers,
                    sync_pct,
                    metrics.connections
                );

                let p = Paragraph::new(text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Bitcoin Node Live Status "),
                    )
                    .style(Style::default().fg(Color::Yellow))
                    .wrap(Wrap { trim: true });
                f.render_widget(p, main_area);
            } else {
                // Waiting for data / config
                let p = Paragraph::new(
                    "Waiting for RPC data...\n\n\
                    • Load bitcoin.conf via 'Bitcoin Config' tab\n\n\
                    For testing: Uncomment the hardcoded URL in main.rs \
                    and ensure Bitcoin Core is running on 127.0.0.1:38332",
                )
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Bitcoin Node Live Status "),
                )
                .style(Style::default().fg(Color::DarkGray))
                .wrap(Wrap { trim: true });
                f.render_widget(p, main_area);
            }
        }
        CurrentScreen::FileExplorer => {
            render_file_explorer(f, app, main_area);
        }
        _ => {}
    }
}

fn render_file_explorer(f: &mut Frame, app: &mut App, area: Rect) {
    let files: Vec<ListItem> = app
        .explorer
        .files
        .iter()
        .map(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let display_name = if path.is_dir() {
                format!("📁 {}", name)
            } else {
                format!("📄 {}", name)
            };
            ListItem::new(display_name)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.explorer.selected_index));

    let title = format!(" Select File (Current: {:?}) ", app.explorer.current_dir);

    let list = List::new(files)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut state);
}
