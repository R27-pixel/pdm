// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::app::{App, P2POOL_STATUS_TABS};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs, Wrap},
};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct P2PoolStatusView;

#[derive(Debug)]
struct ShareTableEntry {
    height: u64,
    blockhash: String,
    miner: String,
    bits: String,
    timestamp: u64,
    uncles: usize,
}

impl P2PoolStatusView {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    // P2Pool Status
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
            1 => Self::render_share_info(f, app, outer[1]),
            2 => Self::render_peer_info(f, app, outer[1]),
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
                Line::from(format!("Total Work             : {}", info.total_work)),
            ]
        } else if let Some(err) = &app.p2pool_chain_info_error {
            vec![Line::from(Span::styled(
                format!("Failed to fetch chain info: {err}"),
                Style::default().fg(Color::Red),
            ))]
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

    fn render_share_info(f: &mut Frame, app: &App, area: Rect) {
        let mut rows = Self::share_rows(app);
        if rows.is_empty() {
            rows.push(Self::message_row(Self::share_empty_message(app)));
        }

        let header = Row::new([
            "Height",
            "Blockhash",
            "Miner",
            "Difficulty",
            "Time",
            "Uncles",
        ])
        .style(
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);
        let widths = [
            Constraint::Length(7),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Min(12),
            Constraint::Length(6),
        ];

        let table = Table::new(rows, widths)
            .block(Block::default().borders(Borders::ALL).title(" Shares "))
            .header(header)
            .column_spacing(1)
            .style(Style::default().fg(Color::White));

        f.render_widget(table, area);
    }

    fn render_peer_info(f: &mut Frame, app: &App, area: Rect) {
        let mut text = if let Some(peers) = &app.peer_info {
            if peers.is_empty() {
                vec![Line::from(Span::styled(
                    "No connected peers",
                    Style::default().fg(Color::DarkGray),
                ))]
            } else {
                let mut lines = Vec::with_capacity(peers.len() + 2);
                lines.push(Line::from(format!(
                    "Connected Peers        : {}",
                    peers.len()
                )));
                lines.push(Line::from(""));

                for peer in peers {
                    lines.push(Line::from(format!(
                        "{} ({})",
                        peer.peer_id,
                        peer.status.as_deref().unwrap_or("Connected")
                    )));
                }

                lines
            }
        } else if let Some(err) = &app.p2pool_peer_info_error {
            vec![Line::from(Span::styled(
                format!("Failed to fetch peer info: {err}"),
                Style::default().fg(Color::Red),
            ))]
        } else {
            vec![Line::from(Span::styled(
                "Loading peer info...",
                Style::default().fg(Color::DarkGray),
            ))]
        };

        if let Some(err) = &app.p2pool_live_error {
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                format!("Live stream error: {err}"),
                Style::default().fg(Color::Red),
            )));
        }

        if !app.live_peer_events.is_empty() {
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "Live Peer Events",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for event in app.live_peer_events.iter().rev().take(8) {
                text.push(Line::from(format!(
                    "{}: {}",
                    event.status,
                    Self::short_value(&event.peer_id, 42)
                )));
            }
        }

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(" Peers Info "))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    fn short_value(value: &str, max_len: usize) -> String {
        if value.len() <= max_len {
            return value.to_string();
        }

        if max_len <= 3 {
            return value.chars().take(max_len).collect();
        }

        let head_len = (max_len - 3) / 2;
        let tail_len = max_len - 3 - head_len;
        let head: String = value.chars().take(head_len).collect();
        let tail: String = value
            .chars()
            .rev()
            .take(tail_len)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{head}...{tail}")
    }

    fn share_rows(app: &App) -> Vec<Row<'static>> {
        let mut entries = Vec::new();
        let mut seen = HashSet::new();

        for share in app.live_shares.iter().rev() {
            seen.insert(share.blockhash.clone());
            entries.push(ShareTableEntry {
                height: share.height,
                blockhash: share.blockhash.clone(),
                miner: share.miner_address.clone(),
                bits: share.bits.clone(),
                timestamp: share.timestamp,
                uncles: share.uncles.len(),
            });
        }

        if let Some(info) = &app.share_info {
            for share in info.shares.iter().rev() {
                if seen.insert(share.blockhash.clone()) {
                    entries.push(ShareTableEntry {
                        height: share.height,
                        blockhash: share.blockhash.clone(),
                        miner: share.miner_address.clone(),
                        bits: share.bits.clone(),
                        timestamp: share.timestamp,
                        uncles: share.uncles.len(),
                    });
                }
            }
        }

        entries.sort_by(|left, right| {
            right
                .height
                .cmp(&left.height)
                .then_with(|| right.timestamp.cmp(&left.timestamp))
        });

        entries
            .into_iter()
            .take(50)
            .map(|entry| {
                Self::share_row(
                    entry.height,
                    &entry.blockhash,
                    &entry.miner,
                    &entry.bits,
                    entry.timestamp,
                    entry.uncles,
                )
            })
            .collect()
    }

    fn share_row(
        height: u64,
        blockhash: &str,
        miner: &str,
        bits: &str,
        timestamp: u64,
        uncles: usize,
    ) -> Row<'static> {
        Row::new(vec![
            Cell::from(height.to_string()),
            Self::chip(Self::short_value(blockhash, 10)),
            Self::chip(Self::short_value(miner, 10)),
            Cell::from(Self::format_difficulty(bits)),
            Cell::from(Self::format_timestamp(timestamp)),
            Cell::from(uncles.to_string()),
        ])
        .height(1)
    }

    fn message_row(message: String) -> Row<'static> {
        Row::new(vec![
            Cell::from(Span::styled(message, Style::default().fg(Color::DarkGray))),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ])
    }

    fn share_empty_message(app: &App) -> String {
        if let Some(err) = &app.p2pool_share_info_error {
            return format!("Recent shares unavailable: {}", Self::short_value(err, 64));
        }

        if let Some(err) = &app.p2pool_live_error {
            return format!("Live shares unavailable: {}", Self::short_value(err, 64));
        }

        "Waiting for share data...".to_string()
    }

    fn chip(value: String) -> Cell<'static> {
        Cell::from(Span::styled(
            value,
            Style::default().fg(Color::Gray).bg(Color::Black),
        ))
    }

    fn format_difficulty(bits: &str) -> String {
        let Some(bits) = Self::parse_bits(bits) else {
            return Self::short_value(bits, 10);
        };

        let exponent = (bits >> 24) as i32;
        let mantissa = bits & 0x00ff_ffff;
        if mantissa == 0 {
            return "-".to_string();
        }

        let difficulty = (0x00ff_ff_u32 as f64 / mantissa as f64) * 256_f64.powi(0x1d - exponent);
        if !difficulty.is_finite() || difficulty <= 0.0 {
            return "-".to_string();
        }

        if difficulty >= 100.0 {
            return Self::format_integer_with_commas(difficulty.round() as u64);
        }

        if difficulty >= 1.0 {
            return format!("{difficulty:.2}");
        }

        format!("{difficulty:.4}")
    }

    fn parse_bits(bits: &str) -> Option<u32> {
        let value = bits.trim();
        if value.is_empty() {
            return None;
        }

        if let Some(hex) = value
            .strip_prefix("0x")
            .or_else(|| value.strip_prefix("0X"))
        {
            return u32::from_str_radix(hex, 16).ok();
        }

        if value
            .chars()
            .any(|c| c.is_ascii_hexdigit() && c.is_ascii_alphabetic())
        {
            return u32::from_str_radix(value, 16).ok();
        }

        value.parse::<u32>().ok()
    }

    fn format_integer_with_commas(value: u64) -> String {
        let digits = value.to_string();
        let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
        for (index, digit) in digits.chars().rev().enumerate() {
            if index > 0 && index % 3 == 0 {
                formatted.push(',');
            }
            formatted.push(digit);
        }
        formatted.chars().rev().collect()
    }

    fn format_timestamp(timestamp: u64) -> String {
        let timestamp = if timestamp > 10_000_000_000 {
            timestamp / 1_000
        } else {
            timestamp
        };
        let days = (timestamp / 86_400) as i64;
        let seconds = timestamp % 86_400;
        let hour = seconds / 3_600;
        let minute = (seconds % 3_600) / 60;
        let second = seconds % 60;
        let (year, month, day) = Self::civil_from_days(days);
        let suffix = if hour < 12 { "AM" } else { "PM" };
        let hour = match hour % 12 {
            0 => 12,
            value => value,
        };

        format!("{month}/{day}/{year}, {hour}:{minute:02}:{second:02} {suffix}")
    }

    fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
        let days = days_since_epoch + 719_468;
        let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
        let day_of_era = days - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let year = year_of_era + era * 400;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_param = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month_param + 2) / 5 + 1;
        let month = month_param + if month_param < 10 { 3 } else { -9 };
        let year = year + if month <= 2 { 1 } else { 0 };

        (year as i32, month as u32, day as u32)
    }
}

impl Default for P2PoolStatusView {
    fn default() -> Self {
        Self::new()
    }
}
