// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};

// Render the file picker with the current directory and active selection.
pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
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
