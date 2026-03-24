// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::app::{App, CurrentScreen};
use ratatui::{prelude::*, widgets::Paragraph};

#[derive(Clone, Debug)]
pub struct StatusBar;

fn hint(key: &str, desc: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            format!(" {key} "),
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::styled(format!(" {desc}  "), Style::default().fg(Color::DarkGray)),
    ]
}

impl StatusBar {
    pub fn new() -> Self {
        Self
    }

    /// Renders the status bar with context-sensitive key hints
    /// based on the current screen and application state.
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let mut spans: Vec<Span> = Vec::new();

        match app.current_screen {
            CurrentScreen::FileExplorer => {
                spans.extend(hint("↑↓", "Navigate"));
                spans.extend(hint("Enter", "Select"));
                spans.extend(hint("⌫", "Go up")); // reflects the Backspace keybinding
                spans.extend(hint("Esc", "Cancel"));
            }
            CurrentScreen::BitcoinConfig if app.bitcoin_conf_path.is_some() => {
                spans.extend(hint("↑↓", "Navigate"));
                spans.extend(hint("Enter", "Open file"));
                spans.extend(hint("q", "Quit"));
            }
            CurrentScreen::P2PoolConfig if app.p2pool_conf_path.is_some() => {
                spans.extend(hint("↑↓", "Navigate"));
                spans.extend(hint("Enter", "Open file"));
                spans.extend(hint("q", "Quit"));
            }
            CurrentScreen::BitcoinStatus => {
                spans.extend(hint("↑↓", "Navigate sidebar"));
                spans.extend(hint("←→", "Switch tab"));
                spans.extend(hint("q", "Quit"));
            }
            _ => {
                spans.extend(hint("↑↓", "Navigate sidebar"));
                spans.extend(hint("Enter", "Select"));
                spans.extend(hint("q", "Quit"));
            }
        }

        let bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Black));
        f.render_widget(bar, area);
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}
