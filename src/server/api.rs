//! REST handlers. Each takes JSON params (query string or body) and returns
//! JSON; the MCP tools call the same functions.

use super::{lock, now, Shared};
use crate::plan;
use crate::scanner::tree::{FileTree, NodeId};
use crate::scanner::ScanProgress;
use crate::ui::theme::format_size;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
pub struct ApiError(pub u16, pub String);

fn err(status: u16, msg: impl Into<String>) -> ApiError {
    ApiError(status, msg.into())
}

/// Reads a param that may arrive as a JSON string (query) or number (body).
fn num(p: &Value, key: &str, default: u64, max: u64) -> u64 {
    let v = &p[key];
    v.as_u64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(default)
        .min(max)
}

fn text<'a>(p: &'a Value, key: &str) -> Option<&'a str> {
    p[key].as_str().filter(|s| !s.is_empty())
}

pub fn route(
    sh: &Arc<Shared>,
    method: &str,
    path: &str,
    p: &Value,
) -> Result<(u16, Value), ApiError> {
    let rest = path
        .strip_prefix("/api/v1")
        .ok_or_else(|| err(404, "not found"))?;
    let ok = |v: Value| Ok((200, v));
    match (method, rest) {
        ("GET", "/openapi.json") => ok(serde_json::from_str(include_str!("openapi.json")).unwrap()),
        ("GET", "/volumes") => ok(volumes()),
        ("GET", "/scan") => ok(scan_status(sh)),
        ("POST", "/scan") => {
            let root = text(p, "root").map(PathBuf::from);
            Ok((202, rescan(sh, root)?))
        }
        ("GET", "/tree") => ok(tree(sh, p)?),
        ("GET", "/top") => ok(top(sh, p)?),
        ("GET", "/find") => ok(find(sh, p)?),
        ("GET", "/cleanup") => ok(cleanup(sh)?),
        ("GET", "/summary") => ok(summary(sh)),
        // Admin only (see mcp_allowed): feeds the "claude mcp add" line.
        ("GET", "/mcp-token") => ok(json!({ "token": sh.mcp_token })),
        ("GET", "/plans") => ok(list_plans(sh)),
        ("POST", "/plans") => Ok((201, propose(sh, p, "web")?)),
        _ => {
            let tail = rest
                .strip_prefix("/plans/")
                .ok_or_else(|| err(404, "not found"))?;
            let (id, verb) = tail.split_once('/').unwrap_or((tail, ""));
            let id: u64 = id.parse().map_err(|_| err(404, "bad plan id"))?;
            match (method, verb) {
                ("GET", "") => ok(get_plan(sh, id)?),
                ("POST", "approve") => Ok((202, approve(sh, id)?)),
                ("POST", "reject") => ok(reject(sh, id)?),
                _ => Err(err(404, "not found")),
            }
        }
    }
}

pub fn volumes() -> Value {
    let v: Vec<Value> = crate::relocate::list_volumes()
        .into_iter()
        .map(|v| {
            json!({
                "name": v.name,
                "path": v.path.display().to_string(),
                "fs_type": v.fs.fs_type,
                "total_bytes": v.fs.total,
                "free_bytes": v.fs.free,
                "boot": v.boot,
                "can_offload_to": !v.boot,
            })
        })
        .collect();
    json!({ "volumes": v })
}

/// Starts a scan of `root` in the background (startup and POST /scan).
pub fn start_scan(sh: &Arc<Shared>, root: PathBuf) {
    {
        let mut s = lock(&sh.scan);
        s.root = root.clone();
        s.scanning = true;
        (s.files, s.dirs, s.bytes, s.errors) = (0, 0, 0, 0);
        s.started = now();
        s.finished = None;
    }
    let (tx, rx) = crossbeam_channel::unbounded();
    crate::scanner::scan(root, tx);
    let sh = sh.clone();
    std::thread::spawn(move || {
        for msg in rx {
            let mut s = lock(&sh.scan);
            match msg {
                ScanProgress::Counting {
                    files,
                    dirs,
                    bytes,
                    errors,
                    ..
                } => {
                    (s.files, s.dirs, s.bytes, s.errors) = (files, dirs, bytes, errors);
                }
                ScanProgress::Done(tree) => {
                    s.tree = Some(tree);
                    s.scanning = false;
                    s.finished = Some(now());
                }
                ScanProgress::Error(_) => {
                    s.scanning = false;
                    s.finished = Some(now());
                }
            }
        }
    });
}

pub fn rescan(sh: &Arc<Shared>, root: Option<PathBuf>) -> Result<Value, ApiError> {
    let root = match root {
        Some(r) => r,
        None => lock(&sh.scan).root.clone(),
    };
    if !root.is_dir() {
        return Err(err(400, format!("{} is not a folder", root.display())));
    }
    if lock(&sh.scan).scanning {
        return Err(err(409, "a scan is already running"));
    }
    start_scan(sh, root);
    Ok(scan_status(sh))
}

pub fn scan_status(sh: &Shared) -> Value {
    let s = lock(&sh.scan);
    json!({
        "root": s.root.display().to_string(),
        "scanning": s.scanning,
        "has_result": s.tree.is_some(),
        "files": s.files,
        "dirs": s.dirs,
        "bytes": s.bytes,
        "errors": s.errors,
        "started": s.started,
        "finished": s.finished,
        "total_bytes": s.tree.as_ref().map(|t| t.node(t.root()).size),
    })
}

fn with_tree<T>(sh: &Shared, f: impl FnOnce(&FileTree) -> T) -> Result<T, ApiError> {
    let s = lock(&sh.scan);
    match &s.tree {
        Some(t) => Ok(f(t)),
        None if s.scanning => Err(err(
            409,
            "the first scan is still running; check GET /api/v1/scan",
        )),
        None => Err(err(409, "no scan yet; POST /api/v1/scan")),
    }
}

/// Free-space and skipped-bytes blocks the scanner adds at a volume root.
fn synthetic(t: &FileTree, id: NodeId) -> bool {
    t.extension(id).is_some_and(|e| e.starts_with("__"))
}

fn node_json(t: &FileTree, id: NodeId) -> Value {
    let n = t.node(id);
    let modified = n
        .modified
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    json!({
        "path": t.full_path(id).display().to_string(),
        "name": t.name(id),
        "size": n.size,
        "is_dir": n.is_dir(),
        "items": n.items(),
        "modified": modified,
    })
}

pub fn tree(sh: &Shared, p: &Value) -> Result<Value, ApiError> {
    let limit = num(p, "limit", 100, 1000) as usize;
    let path = text(p, "path");
    with_tree(sh, |t| {
        let id = match path {
            Some(p) => t
                .find_path(Path::new(p))
                .ok_or_else(|| err(404, format!("{p} is not in the scan")))?,
            None => t.root(),
        };
        let kids: Vec<Value> = t
            .children_sorted(id)
            .into_iter()
            .take(limit)
            .map(|c| node_json(t, c))
            .collect();
        Ok(json!({ "node": node_json(t, id), "children": kids }))
    })?
}

pub fn top(sh: &Shared, p: &Value) -> Result<Value, ApiError> {
    let n = num(p, "n", 25, 500) as usize;
    let dirs = text(p, "kind") == Some("dir");
    with_tree(sh, |t| {
        let root = t.root();
        let mut ids: Vec<NodeId> = if dirs {
            // Folders whose size isn't mostly one subfolder: skips the chain
            // of parents (/, /Users, /Users/me …) that would top the list.
            let mut v = Vec::new();
            let mut stack = vec![root];
            while let Some(id) = stack.pop() {
                let kids = t.node(id).children();
                stack.extend(kids.iter().filter(|&&c| t.node(c).is_dir()));
                let biggest = kids
                    .iter()
                    .filter(|&&c| t.node(c).is_dir())
                    .map(|&c| t.node(c).size)
                    .max()
                    .unwrap_or(0);
                if id != root && biggest * 2 <= t.node(id).size {
                    v.push(id);
                }
            }
            v
        } else {
            t.collect_files(root)
                .into_iter()
                .filter(|&id| !synthetic(t, id))
                .collect()
        };
        ids.sort_by_key(|&id| std::cmp::Reverse(t.node(id).size));
        let items: Vec<Value> = ids.into_iter().take(n).map(|id| node_json(t, id)).collect();
        json!({ "kind": if dirs { "dir" } else { "file" }, "items": items })
    })
}

/// Case-insensitive glob over a single name: `*` any run, `?` one char.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let s: Vec<char> = name.to_lowercase().chars().collect();
    let (mut pi, mut si, mut star, mut mark) = (0, 0, None, 0);
    while si < s.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = si;
            pi += 1;
        } else if let Some(st) = star {
            pi = st + 1;
            mark += 1;
            si = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

pub fn find(sh: &Shared, p: &Value) -> Result<Value, ApiError> {
    let pattern = text(p, "pattern")
        .ok_or_else(|| err(400, "pattern is required (e.g. node_modules or *.dmg)"))?;
    let limit = num(p, "limit", 200, 2000) as usize;
    with_tree(sh, |t| {
        let mut hits = Vec::new();
        let mut stack = vec![t.root()];
        while let Some(id) = stack.pop() {
            if id != t.root() && !synthetic(t, id) && glob_match(pattern, &t.name(id)) {
                hits.push(id); // don't descend: report the outermost match
                continue;
            }
            stack.extend_from_slice(t.node(id).children());
        }
        hits.sort_by_key(|&id| std::cmp::Reverse(t.node(id).size));
        let total: u64 = hits.iter().map(|&id| t.node(id).size).sum();
        let items: Vec<Value> = hits
            .iter()
            .take(limit)
            .map(|&id| node_json(t, id))
            .collect();
        json!({ "pattern": pattern, "count": hits.len(), "total_bytes": total, "items": items })
    })
}

pub fn cleanup(sh: &Shared) -> Result<Value, ApiError> {
    with_tree(sh, |t| {
        let mut groups: Vec<(&str, &str, Vec<Value>, u64)> = Vec::new();
        for c in crate::cleanup::find_candidates(t, t.root()) {
            let idx = match groups.iter().position(|g| g.0 == c.category) {
                Some(i) => i,
                None => {
                    groups.push((c.category, c.description, vec![], 0));
                    groups.len() - 1
                }
            };
            groups[idx].2.push(node_json(t, c.node_id));
            groups[idx].3 += c.size;
        }
        groups.sort_by_key(|g| std::cmp::Reverse(g.3));
        let groups: Vec<Value> = groups
            .into_iter()
            .map(|(cat, desc, items, total)| {
                json!({ "category": cat, "description": desc, "count": items.len(), "total_bytes": total, "items": items })
            })
            .collect();
        json!({ "groups": groups })
    })
}

/// Plain-text digest for pasting into any chat, with instructions that make
/// the answer pasteable back as a plan.
pub fn summary(sh: &Shared) -> Value {
    let mut out = String::from(
        "I use MacDirStat to free disk space on my Mac. Below is a summary of a scan.\n\
         Suggest what to move to an external disk or trash, grouped by kind, and explain the risk of each group.\n\
         Then give me the plan as JSON I can paste back into MacDirStat:\n\
         [{\"action\":\"trash\",\"path\":\"/abs/path\"}, {\"action\":\"move\",\"path\":\"/abs/path\",\"volume\":\"/Volumes/NAME\",\"link\":true}]\n\
         Nothing runs until I approve it. External FAT/exFAT disks cannot hold apps, packages or symlinks.\n\n",
    );
    out.push_str("VOLUMES\n");
    for v in crate::relocate::list_volumes() {
        out.push_str(&format!(
            "- {} ({}, {}): {} free of {}{}\n",
            v.name,
            v.path.display(),
            v.fs.fs_type,
            format_size(v.fs.free),
            format_size(v.fs.total),
            if v.boot { ", startup disk" } else { "" }
        ));
    }
    let line = |n: &Value| {
        format!(
            "- {}  {}\n",
            format_size(n["size"].as_u64().unwrap_or(0)),
            n["path"].as_str().unwrap_or("")
        )
    };
    let st = scan_status(sh);
    out.push_str(&format!(
        "\nSCAN of {}: {}\n",
        st["root"].as_str().unwrap_or(""),
        format_size(st["total_bytes"].as_u64().unwrap_or(0))
    ));
    if let Ok(t) = top(sh, &json!({"n": 15, "kind": "dir"})) {
        out.push_str("\nLARGEST FOLDERS\n");
        t["items"]
            .as_array()
            .into_iter()
            .flatten()
            .for_each(|n| out.push_str(&line(n)));
    }
    if let Ok(t) = top(sh, &json!({"n": 15})) {
        out.push_str("\nLARGEST FILES\n");
        t["items"]
            .as_array()
            .into_iter()
            .flatten()
            .for_each(|n| out.push_str(&line(n)));
    }
    if let Ok(c) = cleanup(sh) {
        out.push_str("\nCLEANUP GROUPS (regenerable data)\n");
        for g in c["groups"].as_array().into_iter().flatten().take(12) {
            out.push_str(&format!(
                "- {}: {} in {} places. {}\n",
                g["category"].as_str().unwrap_or(""),
                format_size(g["total_bytes"].as_u64().unwrap_or(0)),
                g["count"],
                g["description"].as_str().unwrap_or("")
            ));
            g["items"]
                .as_array()
                .into_iter()
                .flatten()
                .take(3)
                .for_each(|n| out.push_str(&format!("  {}", line(n))));
        }
    }
    json!({ "text": out })
}

pub fn list_plans(sh: &Shared) -> Value {
    let plans: Vec<Value> = lock(&sh.plans)
        .list()
        .iter()
        .rev()
        .map(|p| p.to_json())
        .collect();
    json!({ "plans": plans })
}

pub fn get_plan(sh: &Shared, id: u64) -> Result<Value, ApiError> {
    lock(&sh.plans)
        .get(id)
        .map(|p| p.to_json())
        .ok_or_else(|| err(404, format!("no plan {id}")))
}

/// Validates the ops and stores a pending plan. Never runs anything.
pub fn propose(sh: &Shared, p: &Value, origin: &str) -> Result<Value, ApiError> {
    let ops = plan::parse_ops(&p["ops"]).map_err(|e| err(400, e))?;
    if ops.len() > 500 {
        return Err(err(400, "at most 500 ops per plan"));
    }
    let states: Vec<_> = ops.iter().map(plan::validate).collect();
    let note = p["note"].as_str().unwrap_or("");
    Ok(lock(&sh.plans).create(states, note, origin).to_json())
}

/// Human approval: runs the plan in the background and returns at once.
pub fn approve(sh: &Arc<Shared>, id: u64) -> Result<Value, ApiError> {
    let ops = lock(&sh.plans).approve(id).map_err(|e| err(409, e))?;
    let sh2 = sh.clone();
    std::thread::spawn(move || {
        // One plan at a time: overlapping moves must never race on a destination.
        let results: Vec<_> = {
            let _running = lock(&crate::relocate::EXEC_LOCK);
            ops.iter().map(|op| (sh2.executor)(op)).collect()
        };
        // Drop moved / trashed items from the scan, like the GUI does.
        if let Some(t) = lock(&sh2.scan).tree.as_mut() {
            for (op, r) in ops.iter().zip(&results) {
                if let (Ok(_), Some(id)) = (r, t.find_path(&op.path)) {
                    t.remove_node(id);
                }
            }
        }
        let _ = lock(&sh2.plans).finish(id, results);
    });
    get_plan(sh, id)
}

pub fn reject(sh: &Shared, id: u64) -> Result<Value, ApiError> {
    lock(&sh.plans).reject(id).map_err(|e| err(409, e))?;
    get_plan(sh, id)
}

#[cfg(test)]
mod tests {
    use super::glob_match;

    #[test]
    fn globbing() {
        assert!(glob_match("node_modules", "node_modules"));
        assert!(glob_match("*.DMG", "Xcode_16.dmg"));
        assert!(glob_match(".venv*", ".venv"));
        assert!(glob_match("t?rget", "target"));
        assert!(glob_match("*cache*", "Library_Caches_x"));
        assert!(!glob_match("target", "targets"));
        assert!(!glob_match("*.dmg", "dmg.zip"));
    }
}
