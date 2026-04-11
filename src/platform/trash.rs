use std::path::Path;

pub fn move_to_trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|e| format!("Failed to move to trash: {}", e))
}
