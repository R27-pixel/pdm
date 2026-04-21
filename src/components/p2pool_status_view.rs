// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::app::{App, P2POOL_STATUS_TABS};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Tabs, Wrap},
};

#[derive(Debug, Clone)]
pub struct P2PoolStatusView;

impl P2PoolStatusView {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Tabs bar
                Constraint::Min(0),    // Content area
            ])
            .split(area);

        let tabs = Tabs::new(P2POOL_STATUS_TABS.to_vec())
            .block(Block::default().borders(Borders::ALL).title(" Info "))
            .select(app.p2pool_status_tab)
            .highlight_style(Style::default().bg(Color::Gray).fg(Color::Black));

        f.render_widget(tabs, outer[0]);

        match app.p2pool_status_tab {
            0 => Self::render_chain_info(f, app, outer[1]),
            1 => Self::render_system(f, app, outer[1]),
            2 => Self::render_logs(f, app, outer[1]),
            3 => Self::render_peers(f, app, outer[1]),
            4 => Self::render_shares(f, app, outer[1]),
            _ => {}
        }
    }

    // CHAIN INFO TAB
    fn render_chain_info(f: &mut Frame, app: &App, area: Rect) {
        let text = if let Some(info) = &app.chain_info {
            vec![
                Line::from(format!(
                    "Chain Tip Height      : {}",
                    info.chain_tip_height.unwrap_or(0)
                )),
                Line::from(format!(
                    "Top Candidate Height  : {:?}",
                    info.top_candidate_height
                )),
                Line::from(format!("Total Work            : {}", info.total_work)),
                Line::from(format!(
                    "Tip Blockhash         : {}",
                    info.chain_tip_blockhash.as_deref().unwrap_or("-")
                )),
            ]
        } else {
            vec![Line::from("Loading chain info...")]
        };

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(" Chain Info "))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    // SYSTEM TAB
    fn render_system(f: &mut Frame, app: &App, area: Rect) {
        let api_status = if app.chain_info.is_some() {
            "Connected"
        } else {
            "Disconnected"
        };

        let miner_connected = if app.p2pool_state.miner_connected {
            "Yes"
        } else {
            "No"
        };

        let text = format!(
            "API Status       : {}\n\
             Miner Connected  : {}\n\
             Last Share       : {}\n\
             Last Block       : {}\n\
             Last Submit Time : {}",
            api_status,
            miner_connected,
            app.p2pool_state.last_share_status,
            app.p2pool_state.last_block_status,
            app.p2pool_state.last_submit_time,
        );

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(" System "))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    // LOGS TAB
    fn render_logs(f: &mut Frame, app: &App, area: Rect) {
        let text = if app.p2pool_state.recent_logs.is_empty() {
            "Waiting for live daemon logs...".to_string()
        } else {
            app.p2pool_state.recent_logs.join("\n")
        };

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(" Logs "))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    // PEERS TAB
    fn render_peers(f: &mut Frame, app: &App, area: Rect) {
        let rows: Vec<Row> = if app.peers.is_empty() {
            vec![Row::new(vec![
                Cell::from("No peers connected"),
                Cell::from("-"),
                Cell::from("-"),
            ])]
        } else {
            app.peers
                .iter()
                .map(|peer| {
                    Row::new(vec![
                        Cell::from(peer.peer_id.clone()),
                        Cell::from("Active"),
                        Cell::from("-"),
                    ])
                })
                .collect()
        };

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(60),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ],
        )
        .header(
            Row::new(vec![
                Cell::from("Peer ID"),
                Cell::from("Status"),
                Cell::from("Info"),
            ])
            .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title(" Peers "));

        f.render_widget(table, area);
    }

    // SHARES TAB
    fn render_shares(f: &mut Frame, app: &App, area: Rect) {
        let mut items: Vec<ListItem> = vec![
            ListItem::new(format!(
                "Latest Status: {}",
                app.p2pool_state.last_share_status
            )),
            ListItem::new(""),
        ];

        if app.recent_shares.is_empty() {
            items.push(ListItem::new("No accepted shares yet..."));
        } else {
            for share in &app.recent_shares {
                items.push(ListItem::new(format!(
                    "Height: {} | Miner: {}",
                    share.height, share.miner_address
                )));
            }
        }

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Recent Shares "),
        );

        f.render_widget(list, area);
    }
}

impl Default for P2PoolStatusView {
    fn default() -> Self {
        Self::new()
    }
}
