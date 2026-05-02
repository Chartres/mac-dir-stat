use std::path::Path;
use std::process::Command;

pub fn move_to_trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|e| format!("Failed to move to trash: {}", e))
}

/// Empties the user's Trash via Finder. We've already shown our own
/// confirmation, so we use `without warning` to skip Finder's.
pub fn empty_trash() -> Result<(), String> {
    let status = Command::new("osascript")
        .args([
            "-e",
            "tell application \"Finder\" to empty trash without warning",
        ])
        .status()
        .map_err(|e| format!("osascript failed to start: {}", e))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("osascript exited with status {}", status))
    }
}
