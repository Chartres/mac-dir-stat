use std::path::Path;
use std::process::Command;

/// Spawn `cmd` and wait for it on a background thread so it doesn't linger as a zombie.
fn spawn_reaped(cmd: &mut Command) {
    if let Ok(mut child) = cmd.spawn() {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

/// Reveal a path in Finder (selects it in its containing folder).
pub fn reveal_in_finder(path: &Path) {
    spawn_reaped(Command::new("open").arg("-R").arg(path));
}

/// Open a file in its default app, or a folder in Finder.
pub fn open_path(path: &Path) {
    spawn_reaped(Command::new("open").arg(path));
}

/// Open a Terminal window at `dir` (or a file's containing folder).
pub fn open_terminal_at(dir: &Path) {
    spawn_reaped(Command::new("open").arg("-a").arg("Terminal").arg(dir));
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
    spawn_reaped(Command::new("osascript").arg("-e").arg(script));
}
