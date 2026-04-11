use std::path::Path;
use std::process::Command;

pub fn reveal_in_finder(path: &Path) {
    let _ = Command::new("open").arg("-R").arg(path).spawn();
}
