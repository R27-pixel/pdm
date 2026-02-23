// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::components::file_explorer::FileExplorer;
use crate::components::metrics::BitcoinMetrics;
use crate::config::BitcoinConfig;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CurrentScreen {
    Home,
    BitcoinStatus,
    BitcoinConfig,
    FileExplorer,
    Exiting,
}

pub struct App {
    pub current_screen: CurrentScreen,
    pub sidebar_index: usize,
    pub bitcoin_conf_path: Option<PathBuf>,
    pub explorer: FileExplorer,
    pub bitcoin_config: Option<BitcoinConfig>,
    pub bitcoin_metrics: Option<BitcoinMetrics>,
}

impl App {
    pub fn new() -> App {
        App {
            current_screen: CurrentScreen::Home,
            sidebar_index: 0,
            bitcoin_conf_path: None,
            explorer: FileExplorer::new(),
            bitcoin_config: None,
            bitcoin_metrics: None,
        }
    }

    pub fn toggle_menu(&mut self) {
        // Logic to switch between sidebar items
        match self.sidebar_index {
            0 => self.current_screen = CurrentScreen::Home,
            1 => self.current_screen = CurrentScreen::BitcoinConfig,
            2 => self.current_screen = CurrentScreen::BitcoinStatus,
            _ => {}
        }
    }
}
impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
