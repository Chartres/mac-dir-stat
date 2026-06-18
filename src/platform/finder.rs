use std::path::Path;
use std::process::Command;

/// Reveal a path in Finder (selects it in its containing folder).
pub fn reveal_in_finder(path: &Path) {
    let _ = Command::new("open").arg("-R").arg(path).spawn();
}

/// Open a file in its default app, or a folder in Finder.
pub fn open_path(path: &Path) {
    let _ = Command::new("open").arg(path).spawn();
}

/// Open a Terminal window at `dir` (or a file's containing folder).
pub fn open_terminal_at(dir: &Path) {
    let _ = Command::new("open")
        .arg("-a")
        .arg("Terminal")
        .arg(dir)
        .spawn();
}

/// Open the Finder "Get Info" window for a path (WinDirStat's Properties).
pub fn get_info(path: &Path) {
    let p = path.to_string_lossy();
    // Escape for embedding inside an AppleScript double-quoted string.
    let escaped = p.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        "tell application \"Finder\"\n\
         activate\n\
         open information window of (POSIX file \"{}\" as alias)\n\
         end tell",
        escaped
    );
    let _ = Command::new("osascript").arg("-e").arg(script).spawn();
}
