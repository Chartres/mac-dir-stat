use std::path::Path;
use std::process::Command;

pub fn move_to_trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|e| format!("Failed to move to trash: {}", e))
}

/// Empties the user's Trash by spawning `osascript` as a background process.
/// Returns immediately after spawn — success only means the process started,
/// not that the trash was emptied.  Finder performs the actual work.
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
