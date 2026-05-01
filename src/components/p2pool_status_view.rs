// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::app::{App, P2POOL_STATUS_TABS};
use crate::components::difficulty::difficulty_from_bits;
use crate::components::p2pool_client::{ShareInfo, UncleInfo};
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

    fn render_chain_info(f: &mut Frame, app: &App, area: Rect) {
        let text = if let Some(info) = &app.chain_info {
            vec![
                Line::from(format!(
                    "Genesis Blockhash      : {}",
                    info.genesis_blockhash.as_deref().unwrap_or("-")
                )),
                Line::from(format!(
                    "Chain Tip Height       : {}",
                    info.chain_tip_height.unwrap_or(0)
                )),
                Line::from(format!(
                    "Chain Tip Blockhash    : {}",
                    info.chain_tip_blockhash.as_deref().unwrap_or("-")
                )),
                Line::from(format!(
                    "Top Candidate Height   : {:?}",
                    info.top_candidate_height
                )),
                Line::from(format!(
                    "Top Candidate Blockhash: {}",
                    info.top_candidate_blockhash.as_deref().unwrap_or("-")
                )),
                Line::from(format!("Total Work             : {}", info.total_work)),
            ]
        } else {
            vec![Line::from(Span::styled(
                "Loading chain info...",
                Style::default().fg(Color::DarkGray),
            ))]
        };

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(" Chain Info "))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    // SYSTEM TAB
    fn render_system(f: &mut Frame, app: &App, area: Rect) {
        let state = &app.p2pool_state;
        let api_ok = app
            .api_status
            .as_ref()
            .map(|s| s.trim() == "OK")
            .unwrap_or(false);

        let label = |s: &'static str| Span::styled(s, Style::default().fg(Color::Cyan));
        let value = |s: String| Span::raw(s);

        let api_span = if api_ok {
            Span::styled("Connected", Style::default().fg(Color::Green))
        } else {
            Span::styled("Unreachable", Style::default().fg(Color::Red))
        };

        // "Miner Connected" is not exposed — show worker activity instead
        let worker_span = if state.any_worker_active() {
            Span::styled(
                format!("{} worker(s) active", state.workers.len()),
                Style::default().fg(Color::Green),
            )
        } else {
            Span::styled(
                "No worker activity yet",
                Style::default().fg(Color::DarkGray),
            )
        };

        let mut lines = vec![
            Line::from(vec![label("API Status          : "), api_span]),
            Line::from(vec![label("Worker Activity     : "), worker_span]),
            Line::from(vec![
                label("Shares Accepted     : "),
                value(state.shares_accepted.to_string()),
            ]),
            Line::from(vec![
                label("Shares Rejected     : "),
                value(state.shares_rejected.to_string()),
            ]),
            Line::from(vec![
                label("Pool Difficulty     : "),
                value(Self::format_difficulty_f64(state.pool_difficulty)),
            ]),
            Line::from(vec![
                label("Best Share          : "),
                value(Self::format_difficulty_f64(state.best_share)),
            ]),
            Line::from(vec![
                label("Best Share Ever     : "),
                value(Self::format_difficulty_f64(state.best_share_ever)),
            ]),
            Line::from(vec![
                label("Last Share          : "),
                value(state.last_share_status.clone()),
            ]),
            Line::from(vec![
                label("Last Submit Time    : "),
                value(state.last_submit_time.clone()),
            ]),
        ];

        // Per-worker breakdown (only shown when workers exist)
        if !state.workers.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Workers",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));

            for w in &state.workers {
                let ts = if w.last_share_at > 0 {
                    chrono::DateTime::from_timestamp(w.last_share_at as i64, 0)
                        .map(|dt| dt.format("%H:%M:%S UTC").to_string())
                        .unwrap_or_else(|| w.last_share_at.to_string())
                } else {
                    "-".to_string()
                };

                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(
                        Self::truncate_addr(&w.address, 20),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!("  shares:{}", w.shares_valid),
                        Style::default().fg(Color::Green),
                    ),
                    Span::styled(
                        format!("  last:{}", ts),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        }

        f.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(" System "))
                .wrap(Wrap { trim: true }),
            area,
        );
    }

    fn format_difficulty_f64(value: f64) -> String {
        if value <= 0.0 {
            return "0".to_string();
        }
        const SUFFIXES: &[&str] = &["", "K", "M", "G", "T", "P"];
        let mut scaled = value;
        let mut tier = 0;
        while scaled >= 1_000.0 && tier < SUFFIXES.len() - 1 {
            scaled /= 1_000.0;
            tier += 1;
        }
        if scaled >= 100.0 {
            format!("{:.0}{}", scaled, SUFFIXES[tier])
        } else if scaled >= 10.0 {
            format!("{:.1}{}", scaled, SUFFIXES[tier])
        } else {
            format!("{:.2}{}", scaled, SUFFIXES[tier])
        }
    }

    fn truncate_addr(s: &str, max: usize) -> String {
        if s.len() <= max {
            s.to_string()
        } else {
            format!("{}…", &s[..max])
        }
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
                Cell::from(Span::styled(
                    "No peers connected",
                    Style::default().fg(Color::DarkGray),
                )),
                Cell::from("-"),
                Cell::from("-"),
            ])]
        } else {
            app.peers
                .iter()
                .map(|peer| {
                    Row::new(vec![
                        Cell::from(peer.peer_id.clone()),
                        Cell::from(peer.status.clone()),
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
        // Split: status line on top, table below
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(area);

        // Status line (mirrors "Latest Status" from the web Shares section)
        let status_line = Line::from(vec![
            Span::styled("Latest Status: ", Style::default().fg(Color::Cyan)),
            Span::raw(app.p2pool_state.last_share_status.clone()),
        ]);
        f.render_widget(Paragraph::new(vec![status_line]), chunks[0]);

        // Build flat row list: each share followed by its uncle rows (dimmed)
        struct ShareRow<'a> {
            height: String,
            hash: String,
            miner: String,
            difficulty: String,
            time: String,
            uncles: String,
            is_uncle: bool,
            _phantom: std::marker::PhantomData<&'a ()>,
        }

        let mut rows: Vec<ShareRow> = Vec::new();

        for share in &app.recent_shares {
            rows.push(ShareRow {
                height: share.height.to_string(),
                hash: truncate(&share.blockhash, 10),
                miner: truncate(&share.miner_address, 18),
                difficulty: difficulty_from_bits(&share.bits),
                time: format_ts(share.timestamp),
                uncles: share.uncles.len().to_string(),
                is_uncle: false,
                _phantom: std::marker::PhantomData,
            });

            // Inline uncle rows — dimmed, indented miner column
            for uncle in &share.uncles {
                rows.push(ShareRow {
                    height: uncle.height.to_string(),
                    hash: truncate(&uncle.blockhash, 10),
                    miner: format!("  └ {}", truncate(&uncle.miner_address, 14)),
                    difficulty: "-".to_string(),
                    time: format_ts(uncle.timestamp),
                    uncles: "uncle".to_string(),
                    is_uncle: true,
                    _phantom: std::marker::PhantomData,
                });
            }
        }

        let header = Row::new(vec![
            Cell::from("Height").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Blockhash").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Miner").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Difficulty").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Time").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Uncles").style(Style::default().add_modifier(Modifier::BOLD)),
        ]);

        let table_rows: Vec<Row> = if rows.is_empty() {
            vec![Row::new(vec![
                Cell::from(Span::styled(
                    "No shares yet",
                    Style::default().fg(Color::DarkGray),
                )),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
            ])]
        } else {
            rows.iter()
                .map(|r| {
                    let style = if r.is_uncle {
                        Style::default().fg(Color::DarkGray) // dimmed, like 0.55 opacity in CSS
                    } else {
                        Style::default()
                    };
                    Row::new(vec![
                        Cell::from(Span::styled(r.height.clone(), style)),
                        Cell::from(Span::styled(r.hash.clone(), style)),
                        Cell::from(Span::styled(r.miner.clone(), style)),
                        Cell::from(Span::styled(r.difficulty.clone(), style)),
                        Cell::from(Span::styled(r.time.clone(), style)),
                        Cell::from(Span::styled(r.uncles.clone(), style)),
                    ])
                })
                .collect()
        };

        f.render_widget(
            Table::new(
                table_rows,
                [
                    Constraint::Length(9),  // Height
                    Constraint::Length(12), // Hash
                    Constraint::Min(20),    // Miner
                    Constraint::Length(12), // Difficulty
                    Constraint::Length(21), // Time
                    Constraint::Length(7),  // Uncles
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Recent Shares "),
            ),
            chunks[1],
        );
    }
}

impl Default for P2PoolStatusView {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

fn format_ts(timestamp: u64) -> String {
    chrono::DateTime::from_timestamp(timestamp as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| timestamp.to_string())
}
