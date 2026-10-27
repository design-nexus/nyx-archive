//! This app's own preferences (`~/.config/nyx-archive/settings.toml`).

pub use nyx_theme::ThemeMode;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub mode: ThemeMode,
    pub theme: String,
    pub reduce_motion: bool,
    pub glow: bool,
    /// Extract into a new folder named after the archive.
    pub extract_into_folder: bool,
    /// Where to extract; empty means next to the archive.
    pub extract_dir: String,
    /// What to do when a file already exists: ask, skip, replace or rename.
    pub overwrite: String,
    /// Where the file pickers start.
    pub last_dir: String,
    pub last_format: String,
    pub last_level: u8,
    /// Open the result's folder when an extract or create finishes.
    pub open_when_done: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Omarchy,
            theme: "tokyo-night".into(),
            reduce_motion: false,
            glow: true,
            extract_into_folder: true,
            extract_dir: String::new(),
            overwrite: "ask".into(),
            last_dir: String::new(),
            last_format: "7z".into(),
            last_level: 5,
            open_when_done: false,
        }
    }
}

nyx_core::prefs_store!(Prefs);
