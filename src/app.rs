// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::components::file_explorer::FileExplorer;
use crate::config::ConfigEntry as BitcoinEntry;
use crate::p2poolv2_config_parser::ConfigEntry as P2PoolEntry;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CurrentScreen {
    Home,
    BitcoinConfig,
    P2PoolConfig,
    FileExplorer,
    Exiting,
}

/// Actions that components (Explorer, Editors) can trigger.
/// This decouples input handling from business logic.
#[derive(Debug, Clone)]
pub enum AppAction {
    None,
    Quit,
    ToggleMenu,
    Navigate(CurrentScreen),
    // Triggers the file explorer for a specific screen
    OpenExplorer(CurrentScreen),
    // Returned by the Explorer when user picks a file
    FileSelected(PathBuf),
    // Closes the explorer without selection
    CloseModal,
}

pub struct App {
    pub current_screen: CurrentScreen,
    pub sidebar_index: usize,
    pub explorer_trigger: Option<CurrentScreen>,
    pub bitcoin_conf_path: Option<PathBuf>,
    pub p2pool_conf_path: Option<PathBuf>,
    pub explorer: FileExplorer,
    pub p2pool_data: Vec<P2PoolEntry>,
    pub bitcoin_data: Vec<BitcoinEntry>,
}

impl App {
    pub fn new() -> App {
        App {
            current_screen: CurrentScreen::Home,
            sidebar_index: 0,
            explorer_trigger: None,
            bitcoin_conf_path: None,
            p2pool_conf_path: None,
            explorer: FileExplorer::new(),
            p2pool_data: Vec::new(),
            bitcoin_data: Vec::new(),
        }
    }

    pub fn toggle_menu(&mut self) {
        // Logic to switch between sidebar items
        match self.sidebar_index {
            0 => self.current_screen = CurrentScreen::Home,
            1 => self.current_screen = CurrentScreen::BitcoinConfig,
            2 => self.current_screen = CurrentScreen::P2PoolConfig,
            _ => {}
        }
    }
}
impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
