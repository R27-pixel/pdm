// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

// Render the parsed p2pool view until a config file is selected.
pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    if app.p2pool_conf_path.is_none() {
        let p = Paragraph::new("Press [Enter] to select a p2poolv2 config file").block(
            Block::default()
                .borders(Borders::ALL)
                .title(" P2Pool Config "),
        );
        f.render_widget(p, area);
        return;
    }

    let mut items: Vec<ListItem> = Vec::new();

    if let Some(cfg) = &app.p2pool_config {
        let blue = Style::default().fg(Color::Blue);

        let fields: &[(&str, String)] = &[
            ("[stratum] ", format!("hostname = {}", cfg.stratum.hostname)),
            ("[stratum] ", format!("port = {}", cfg.stratum.port)),
            (
                "[stratum] ",
                format!("start_difficulty = {}", cfg.stratum.start_difficulty),
            ),
            (
                "[stratum] ",
                format!("minimum_difficulty = {}", cfg.stratum.minimum_difficulty),
            ),
            ("[bitcoinrpc] ", format!("url = {}", cfg.bitcoinrpc.url)),
            (
                "[bitcoinrpc] ",
                format!("username = {}", cfg.bitcoinrpc.username),
            ),
            (
                "[network] ",
                format!("listen_address = {}", cfg.network.listen_address),
            ),
            (
                "[network] ",
                format!(
                    "max_established_incoming = {}",
                    cfg.network.max_established_incoming
                ),
            ),
            ("[store] ", format!("path = {}", cfg.store.path)),
            ("[api] ", format!("hostname = {}", cfg.api.hostname)),
            ("[api] ", format!("port = {}", cfg.api.port)),
        ];

        for (section, value) in fields {
            items.push(ListItem::new(Line::from(vec![
                Span::styled(*section, blue),
                Span::raw(value.clone()),
            ])));
        }
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" P2Pool Configuration "),
    );

    f.render_widget(list, area);
}
