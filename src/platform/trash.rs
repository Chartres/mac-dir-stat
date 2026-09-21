use std::path::Path;
use std::process::Command;

pub fn move_to_trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|e| format!("Failed to move to trash: {}", e))
}

/// Empties the user's Trash via Finder in a background thread so the UI
/// frame is not blocked while Finder moves files.
pub fn empty_trash() -> Result<(), String> {
    Command::new("osascript")
        .args([
            "-e",
            "tell application \"Finder\" to empty trash without warning",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("osascript failed to start: {}", e))
}
