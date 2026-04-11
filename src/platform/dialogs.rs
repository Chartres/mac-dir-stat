use std::path::{Path, PathBuf};

pub fn pick_folder(start_dir: &Path) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Select directory to scan")
        .set_directory(start_dir)
        .pick_folder()
}
