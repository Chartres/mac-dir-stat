use std::path::Path;
use std::process::Command;

fn reap(child: std::process::Child) {
    std::thread::spawn(move || {
        let mut c = child;
        let _ = c.wait();
    });
}

/// Reveal a path in Finder (selects it in its containing folder).
pub fn reveal_in_finder(path: &Path) {
    if let Ok(child) = Command::new("open").arg("-R").arg(path).spawn() {
        reap(child);
    }
}

/// Open a file in its default app, or a folder in Finder.
pub fn open_path(path: &Path) {
    if let Ok(child) = Command::new("open").arg(path).spawn() {
        reap(child);
    }
}

/// Open a Terminal window at `dir` (or a file's containing folder).
pub fn open_terminal_at(dir: &Path) {
    if let Ok(child) = Command::new("open")
        .arg("-a")
        .arg("Terminal")
        .arg(dir)
        .spawn()
    {
        reap(child);
    }
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
    if let Ok(child) = Command::new("osascript").arg("-e").arg(script).spawn() {
        reap(child);
    }
}
