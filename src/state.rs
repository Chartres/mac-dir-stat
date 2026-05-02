//! Persisted UI state — last scan path and color mode are restored on
//! launch so reopening the app jumps back to where the user left off.

use crate::treemap::color::ColorMode;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct PersistedState {
    pub scan_root: Option<PathBuf>,
    pub color_mode: Option<ColorMode>,
}

fn state_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| {
        PathBuf::from(h).join("Library/Application Support/mac-dir-stat")
    })
}

fn state_file() -> Option<PathBuf> {
    state_dir().map(|d| d.join("state.txt"))
}

pub fn load() -> PersistedState {
    let Some(path) = state_file() else {
        return PersistedState::default();
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return PersistedState::default();
    };
    let mut s = PersistedState::default();
    for line in content.lines() {
        if let Some(v) = line.strip_prefix("scan_root=") {
            s.scan_root = Some(PathBuf::from(v));
        } else if let Some(v) = line.strip_prefix("color_mode=") {
            s.color_mode = match v {
                "Extension" => Some(ColorMode::Extension),
                "Depth" => Some(ColorMode::Depth),
                "Age" => Some(ColorMode::Age),
                _ => None,
            };
        }
    }
    s
}

pub fn save(scan_root: &Path, color_mode: ColorMode) {
    let Some(dir) = state_dir() else { return };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let Some(path) = state_file() else { return };
    let mode_str = match color_mode {
        ColorMode::Extension => "Extension",
        ColorMode::Depth => "Depth",
        ColorMode::Age => "Age",
    };
    let content = format!(
        "scan_root={}\ncolor_mode={}\n",
        scan_root.display(),
        mode_str,
    );
    let _ = std::fs::write(path, content);
}
