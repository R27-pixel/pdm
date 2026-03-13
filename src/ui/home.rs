// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

// Render the default home screen shown before the user opens a config.
pub fn render(f: &mut Frame, _app: &mut App, area: Rect) {
    let p = Paragraph::new("Welcome to PDM.\n\nSelect a config from the sidebar to edit.")
        .block(Block::default().borders(Borders::ALL).title(" Home "))
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}
