//! Full Disk Access (FDA) helpers.
//!
//! macOS won't let an app grant itself FDA — it can only detect whether it has
//! it and guide the user to the right settings pane. We prompt **once** (the
//! first launch that lacks access), remembering that we asked so we never nag
//! again. Without FDA the scanner still works; it just can't see TCC-protected
//! locations, which it already prunes to avoid a wall of permission prompts.

use std::path::PathBuf;
use std::process::Command;

fn state_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("Library/Application Support/mac-dir-stat"))
}

/// Open System Settings → Privacy & Security → Full Disk Access.
pub fn open_settings() {
    let _ = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
        .spawn();
}

/// Best-effort check for whether the app already has Full Disk Access: try to
/// read a path that is only readable with FDA. A successful read is a reliable
/// positive; a failure may be FDA-absent *or* the file not existing, so we treat
/// it as "unknown / probably not".
pub fn has_full_disk_access() -> bool {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return false;
    };
    // TCC's own database requires FDA to read. If it isn't there, fall back to
    // the Mail container, another FDA-gated location.
    let candidates = [
        home.join("Library/Application Support/com.apple.TCC/TCC.db"),
        home.join("Library/Mail"),
    ];
    for p in candidates {
        if p.exists() {
            // exists() following the FDA gate: if we can read metadata/dir, we
            // have access.
            if p.is_dir() {
                if std::fs::read_dir(&p).is_ok() {
                    return true;
                }
            } else if std::fs::File::open(&p).is_ok() {
                return true;
            }
        }
    }
    false
}

/// Whether we've already shown the one-time FDA prompt.
pub fn already_prompted() -> bool {
    state_dir()
        .map(|d| d.join("fda_prompted").exists())
        .unwrap_or(false)
}

/// Record that the one-time FDA prompt has been shown.
pub fn mark_prompted() {
    let Some(dir) = state_dir() else { return };
    if std::fs::create_dir_all(&dir).is_ok() {
        let _ = std::fs::write(dir.join("fda_prompted"), "1");
    }
}
