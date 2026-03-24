// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::app::{App, AppAction};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};
use std::fs;
use std::path::PathBuf;

/// Represents a single entry in the file explorer list.
///
/// - `ParentDir` is the virtual `..` entry for navigating upward.
/// - `Directory` is a subdirectory inside the current directory.
/// - `File` is a regular file inside the current directory.
#[derive(Debug, Clone, PartialEq)]
pub enum FileEntry {
    ParentDir,
    Directory(PathBuf),
    File(PathBuf),
}

impl FileEntry {
    /// Returns the path of the entry, or `None` for `ParentDir`.
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            FileEntry::Directory(path) | FileEntry::File(path) => Some(path),
            FileEntry::ParentDir => None,
        }
    }
}

/// `FileExplorer` maintains the current directory, a sorted list of entries,
/// and the currently selected index. It supports navigating directories,
/// moving the selection, and selecting files.
#[derive(Clone)]
pub struct FileExplorer {
    /// Current directory being explored.
    pub current_dir: PathBuf,
    /// Sorted list of entries in `current_dir`.
    /// Directories appear before files; `ParentDir` is always first when present.
    pub file_entries: Vec<FileEntry>,
    /// Index of the currently selected item.
    pub selected_index: usize,
}

impl Default for FileExplorer {
    fn default() -> Self {
        Self::new()
    }
}

impl FileExplorer {
    /// Creates a new `FileExplorer` starting at the process working directory.
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut explorer = Self {
            current_dir,
            file_entries: Vec::new(),
            selected_index: 0,
        };
        explorer.load_directory();
        explorer
    }

    /// Loads the contents of `current_dir` into `file_entries`.
    ///
    /// Directories are listed first, followed by files. If the directory
    /// has a parent, a `ParentDir` entry is added at the top to allow
    /// navigating upward.
    pub fn load_directory(&mut self) {
        self.file_entries.clear();
        self.selected_index = 0;

        // Add a ParentDir entry so the user can navigate up
        if self.current_dir.parent().is_some() {
            self.file_entries.push(FileEntry::ParentDir);
        }

        if let Ok(entries) = fs::read_dir(&self.current_dir) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else {
                    files.push(path);
                }
            }

            dirs.sort();
            files.sort();

            self.file_entries
                .extend(dirs.into_iter().map(FileEntry::Directory));
            self.file_entries
                .extend(files.into_iter().map(FileEntry::File));
        }
    }

    /// Moves the selection down by one entry, clamping at the last item.
    pub fn next(&mut self) {
        if self.selected_index + 1 < self.file_entries.len() {
            self.selected_index += 1;
        }
    }

    /// Moves the selection up by one entry, clamping at the first item.
    pub fn previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Navigates to the parent of `current_dir`, if one exists.
    /// This is also bound to `Backspace` in `handle_input`.
    pub fn go_up(&mut self) {
        if let Some(parent) = self.current_dir.parent().map(|p| p.to_path_buf()) {
            self.current_dir = parent;
            self.load_directory();
        }
    }

    /// Activates the currently selected entry.
    ///
    /// - `ParentDir` — navigates to the parent directory.
    /// - `Directory` — enters that directory.
    /// - `File` — returns its path to the caller.
    pub fn select(&mut self) -> Option<PathBuf> {
        match self.file_entries.get(self.selected_index)?.clone() {
            FileEntry::ParentDir => {
                self.go_up();
                None
            }
            FileEntry::Directory(path) => {
                self.current_dir = path;
                self.load_directory();
                None
            }
            FileEntry::File(path) => Some(path),
        }
    }

    /// Handles keyboard input for the file explorer.
    ///
    /// - `Up` / `Down` — move the selection.
    /// - `Enter` — activate the selected entry.
    /// - `Backspace` — go up to the parent directory.
    /// - `Esc` — close the explorer modal.
    pub fn handle_input(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Up => {
                self.previous();
                AppAction::None
            }
            KeyCode::Down => {
                self.next();
                AppAction::None
            }
            KeyCode::Enter => {
                if let Some(path) = self.select() {
                    return AppAction::FileSelected(path);
                }
                AppAction::None
            }
            KeyCode::Backspace => {
                self.go_up();
                AppAction::None
            }
            KeyCode::Esc => AppAction::CloseModal,
            _ => AppAction::None,
        }
    }

    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        let items: Vec<ListItem> = app
            .explorer
            .file_entries
            .iter()
            .map(|entry| {
                let label = match entry {
                    FileEntry::ParentDir => "📁 ..".to_string(),
                    FileEntry::Directory(p) => {
                        format!("📁 {}", p.file_name().unwrap_or_default().to_string_lossy())
                    }
                    FileEntry::File(p) => {
                        format!("📄 {}", p.file_name().unwrap_or_default().to_string_lossy())
                    }
                };
                ListItem::new(label)
            })
            .collect();

        let mut state = ListState::default();
        state.select(Some(app.explorer.selected_index));

        let title = format!(" Select File (Current: {:?}) ", app.explorer.current_dir);

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    fn setup_temp_fs() -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut base = std::env::temp_dir();

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        base.push(format!("pdm_file_explorer_test_{}", unique));

        fs::create_dir_all(&base).unwrap();
        fs::create_dir(base.join("folder")).unwrap();
        File::create(base.join("file.txt")).unwrap();

        base
    }

    #[test]
    fn loads_directory_entries() {
        let dir = setup_temp_fs();
        let mut explorer = FileExplorer {
            current_dir: dir,
            file_entries: vec![],
            selected_index: 0,
        };

        explorer.load_directory();
        // Expects at least the "folder" dir and "file.txt" created in setup
        assert!(explorer.file_entries.len() >= 2);
    }

    #[test]
    fn next_and_previous_clamp_at_boundaries() {
        let dir = setup_temp_fs();
        let mut explorer = FileExplorer {
            current_dir: dir,
            file_entries: vec![
                FileEntry::Directory(PathBuf::from("a")),
                FileEntry::Directory(PathBuf::from("b")),
            ],
            selected_index: 0,
        };

        explorer.next();
        assert_eq!(explorer.selected_index, 1);

        // At the last item — should not wrap
        explorer.next();
        assert_eq!(explorer.selected_index, 1);

        explorer.previous();
        assert_eq!(explorer.selected_index, 0);

        // At the first item — should not wrap
        explorer.previous();
        assert_eq!(explorer.selected_index, 0);
    }

    #[test]
    fn selecting_file_returns_path() {
        let dir = setup_temp_fs();
        let file = dir.join("file.txt");

        let mut explorer = FileExplorer {
            current_dir: dir,
            file_entries: vec![FileEntry::File(file.clone())],
            selected_index: 0,
        };

        let result = explorer.select();
        assert_eq!(result, Some(file));
    }

    #[test]
    fn selecting_parent_directory_moves_up() {
        let base = setup_temp_fs();
        let child = base.join("child");
        fs::create_dir(&child).unwrap();

        let mut explorer = FileExplorer {
            current_dir: child.clone(),
            file_entries: vec![],
            selected_index: 0,
        };

        explorer.load_directory();

        // First entry must be the ParentDir variant
        assert_eq!(explorer.file_entries[0], FileEntry::ParentDir);

        // Select the ParentDir entry
        let result = explorer.select();

        // Should move to the parent and not return a file path
        assert!(result.is_none());
        assert_eq!(explorer.current_dir, base);
        assert!(!explorer.file_entries.is_empty());
    }

    #[test]
    fn selecting_directory_enters_directory() {
        let base = setup_temp_fs();
        let folder = base.join("folder");

        let mut explorer = FileExplorer {
            current_dir: base.clone(),
            file_entries: vec![FileEntry::Directory(folder.clone())],
            selected_index: 0,
        };

        let result = explorer.select();
        assert!(result.is_none());
        assert_eq!(explorer.current_dir, folder);
    }

    #[test]
    fn default_constructs_explorer() {
        let explorer = FileExplorer::default();
        assert!(!explorer.current_dir.as_os_str().is_empty());
    }

    #[test]
    fn previous_decrements_when_not_zero() {
        let dir = setup_temp_fs();
        let mut explorer = FileExplorer {
            current_dir: dir,
            file_entries: vec![
                FileEntry::File(PathBuf::from("a")),
                FileEntry::File(PathBuf::from("b")),
                FileEntry::File(PathBuf::from("c")),
            ],
            selected_index: 2,
        };

        explorer.previous();
        assert_eq!(explorer.selected_index, 1);
    }
}
