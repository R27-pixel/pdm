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
        ListItem::new("P2Pool Config"),
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
            let p = Paragraph::new("Welcome to PDM.\n\nSelect a config from the sidebar to edit.")
                .block(Block::default().borders(Borders::ALL).title(" Home "))
                .wrap(Wrap { trim: true });
            f.render_widget(p, main_area);
        }
        CurrentScreen::BitcoinConfig => {
            if app.bitcoin_conf_path.is_some() {
                render_bitcoin_view(f, app, main_area);
            } else {
                let p = Paragraph::new("Press [Enter] to select a bitcoin.conf file").block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Bitcoin Config "),
                );
                f.render_widget(p, main_area);
            }
        }

        CurrentScreen::P2PoolConfig => {
            if app.p2pool_conf_path.is_some() {
                render_p2pool_view(f, app, main_area);
            } else {
                let p = Paragraph::new("Press [Enter] to select a p2poolv2 config file").block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" P2Pool Config "),
                );
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

fn render_p2pool_view(f: &mut Frame, app: &mut App, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();

    if let Some(cfg) = &app.p2pool_config {
        // STRATUM
        items.push(ListItem::new(Line::from(vec![
            Span::styled("[stratum] ", Style::default().fg(Color::Blue)),
            Span::raw(format!("hostname = {}", cfg.stratum.hostname)),
        ])));

        items.push(ListItem::new(Line::from(vec![
            Span::styled("[stratum] ", Style::default().fg(Color::Blue)),
            Span::raw(format!("port = {}", cfg.stratum.port)),
        ])));

        items.push(ListItem::new(Line::from(vec![
            Span::styled("[stratum] ", Style::default().fg(Color::Blue)),
            Span::raw(format!(
                "start_difficulty = {}",
                cfg.stratum.start_difficulty
            )),
        ])));

        items.push(ListItem::new(Line::from(vec![
            Span::styled("[stratum] ", Style::default().fg(Color::Blue)),
            Span::raw(format!(
                "minimum_difficulty = {}",
                cfg.stratum.minimum_difficulty
            )),
        ])));

        // BITCOIN RPC
        items.push(ListItem::new(Line::from(vec![
            Span::styled("[bitcoinrpc] ", Style::default().fg(Color::Blue)),
            Span::raw(format!("url = {}", cfg.bitcoinrpc.url)),
        ])));

        items.push(ListItem::new(Line::from(vec![
            Span::styled("[bitcoinrpc] ", Style::default().fg(Color::Blue)),
            Span::raw(format!("username = {}", cfg.bitcoinrpc.username)),
        ])));

        // NETWORK
        items.push(ListItem::new(Line::from(vec![
            Span::styled("[network] ", Style::default().fg(Color::Blue)),
            Span::raw(format!("listen_address = {}", cfg.network.listen_address)),
        ])));

        items.push(ListItem::new(Line::from(vec![
            Span::styled("[network] ", Style::default().fg(Color::Blue)),
            Span::raw(format!(
                "max_established_incoming = {}",
                cfg.network.max_established_incoming
            )),
        ])));

        // STORE
        items.push(ListItem::new(Line::from(vec![
            Span::styled("[store] ", Style::default().fg(Color::Blue)),
            Span::raw(format!("path = {}", cfg.store.path)),
        ])));

        // API
        items.push(ListItem::new(Line::from(vec![
            Span::styled("[api] ", Style::default().fg(Color::Blue)),
            Span::raw(format!("hostname = {}", cfg.api.hostname)),
        ])));

        items.push(ListItem::new(Line::from(vec![
            Span::styled("[api] ", Style::default().fg(Color::Blue)),
            Span::raw(format!("port = {}", cfg.api.port)),
        ])));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" P2Pool Configuration "),
    );

    f.render_widget(list, area);
}

fn render_bitcoin_view(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .bitcoin_data
        .iter()
        .map(|entry| {
            let style = if entry.enabled {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let content = Line::from(vec![
                Span::styled(format!("{} = ", entry.key), style),
                Span::styled(&entry.value, style),
                if !entry.enabled {
                    Span::styled(" (disabled)", style)
                } else {
                    Span::raw("")
                },
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Bitcoin Configuration "),
        )
        .highlight_style(Style::default().bg(Color::Yellow));

    f.render_widget(list, area);
}
