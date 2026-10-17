//! `mac-dir-stat serve`: a headless local server.
//!
//! - `/`            one static web page (no data in it; it reads the token from the URL fragment)
//! - `/api/v1/...`  REST + JSON, described by `/api/v1/openapi.json`
//! - `/mcp`         MCP (Streamable HTTP, JSON responses) for bring-your-own-AI clients
//!
//! Every `/api` and `/mcp` request needs `Authorization: Bearer <token>`.
//! AI clients can propose plans; only an approve (web UI or REST) runs one.

pub mod api;
pub mod mcp;

use crate::plan::{Op, PlanStore};
use crate::scanner::tree::FileTree;
use serde_json::{json, Value};
use std::io::Read;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use tiny_http::{Header, Request, Response};

pub type Executor = Arc<dyn Fn(&Op) -> Result<String, String> + Send + Sync>;

#[derive(Default)]
pub struct ScanState {
    pub root: PathBuf,
    pub tree: Option<FileTree>,
    pub scanning: bool,
    pub files: usize,
    pub dirs: usize,
    pub bytes: u64,
    pub errors: usize,
    pub started: u64,
    pub finished: Option<u64>,
}

/// Locks, ignoring poisoning: one panicked request must not take the server down.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct Shared {
    /// Admin token: everything, including approve/reject. For the human.
    pub token: String,
    /// Token for AI clients: /mcp, REST reads and proposing plans only.
    pub mcp_token: String,
    pub scan: Mutex<ScanState>,
    pub plans: Mutex<PlanStore>,
    pub executor: Executor,
}

pub struct Config {
    pub bind: IpAddr,
    pub port: u16,
    pub root: PathBuf,
    /// Override the stored tokens (tests).
    pub token: Option<String>,
    pub mcp_token: Option<String>,
    /// Overrides how approved ops run (tests).
    pub executor: Option<Executor>,
}

pub struct Handle {
    pub addr: SocketAddr,
    pub token: String,
    pub shared: Arc<Shared>,
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Binds, starts the first scan and serves on background threads.
pub fn start(cfg: Config) -> Result<Handle, String> {
    let token = match cfg.token {
        Some(t) => t,
        None => load_or_create_token(ADMIN_TOKEN_FILE)?,
    };
    let mcp_token = match cfg.mcp_token {
        Some(t) => t,
        None => load_or_create_token(MCP_TOKEN_FILE)?,
    };
    let server = tiny_http::Server::http(SocketAddr::new(cfg.bind, cfg.port))
        .map_err(|e| format!("cannot listen on {}:{}: {e}", cfg.bind, cfg.port))?;
    let addr = server.server_addr().to_ip().ok_or("no IP address")?;
    let shared = Arc::new(Shared {
        token: token.clone(),
        mcp_token,
        scan: Mutex::new(ScanState::default()),
        plans: Mutex::new(PlanStore::default()),
        executor: cfg
            .executor
            .unwrap_or_else(|| Arc::new(crate::plan::execute)),
    });
    api::start_scan(&shared, cfg.root);
    let sh = shared.clone();
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            let sh = sh.clone();
            // ponytail: thread per request; fine for one person and an AI client.
            std::thread::spawn(move || handle(&sh, req));
        }
    });
    Ok(Handle {
        addr,
        token,
        shared,
    })
}

pub const ADMIN_TOKEN_FILE: &str = "server-token";
pub const MCP_TOKEN_FILE: &str = "mcp-token";

/// `~/Library/Application Support/MacDirStat/<name>`.
pub fn token_path(name: &str) -> Result<PathBuf, String> {
    Ok(crate::relocate::home()
        .ok_or("no home folder for this user")?
        .join("Library/Application Support/MacDirStat")
        .join(name))
}

/// Reads the token file, creating it (0600, 32 random bytes) on first run.
pub fn load_or_create_token(name: &str) -> Result<String, String> {
    use std::os::unix::fs::OpenOptionsExt;
    let path = token_path(name)?;
    let dir = path.parent().unwrap().to_path_buf();
    if let Ok(t) = std::fs::read_to_string(&path) {
        if t.trim().len() >= 32 {
            return Ok(t.trim().to_string());
        }
    }
    let mut raw = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut raw))
        .map_err(|e| format!("reading /dev/urandom: {e}"))?;
    let token: String = raw.iter().map(|b| format!("{b:02x}")).collect();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let _ = std::fs::remove_file(&path);
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, token.as_bytes()))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(token)
}

#[derive(PartialEq)]
enum Role {
    Admin,
    Mcp,
}

fn same(given: &str, token: &str) -> bool {
    // Constant-time compare.
    given.len() == token.len()
        && given
            .bytes()
            .zip(token.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

fn role(req: &Request, sh: &Shared) -> Option<Role> {
    let h = req
        .headers()
        .iter()
        .find(|h| h.field.equiv("Authorization"))?;
    let given = h.value.as_str().strip_prefix("Bearer ")?;
    if same(given, &sh.token) {
        Some(Role::Admin)
    } else if same(given, &sh.mcp_token) {
        Some(Role::Mcp)
    } else {
        None
    }
}

/// What the AI token may do: everything on /mcp (including the `scan` tool,
/// which starts a rescan), REST reads, and proposing plans. Not approve or
/// reject, not the REST rescan (POST /api/v1/scan), and not GET /api/v1/mcp-token.
fn mcp_allowed(method: &str, path: &str) -> bool {
    path == "/mcp"
        || (method == "GET" && path != "/api/v1/mcp-token")
        || (method == "POST" && path == "/api/v1/plans")
}

fn header(k: &str, v: &str) -> Header {
    Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap()
}

fn respond_json(req: Request, status: u16, body: &Value) {
    let resp = Response::from_string(body.to_string())
        .with_status_code(status)
        .with_header(header("Content-Type", "application/json"))
        .with_header(header("Cache-Control", "no-store"));
    let _ = req.respond(resp);
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                let hex = |c: u8| (c as char).to_digit(16);
                match (hex(b[i + 1]), hex(b[i + 2])) {
                    (Some(h), Some(l)) => {
                        out.push((h * 16 + l) as u8);
                        i += 2;
                    }
                    _ => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `a=1&b=x%20y` → `{"a":"1","b":"x y"}`.
pub fn parse_query(q: &str) -> Value {
    let mut m = serde_json::Map::new();
    for pair in q.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        m.insert(percent_decode(k), Value::String(percent_decode(v)));
    }
    Value::Object(m)
}

fn handle(sh: &Arc<Shared>, mut req: Request) {
    let url = req.url().to_string();
    let (path, query) = url.split_once('?').unwrap_or((&url, ""));
    let method = req.method().as_str().to_string();

    if method == "GET" && (path == "/" || path == "/index.html") {
        let resp = Response::from_string(include_str!("index.html"))
            .with_header(header("Content-Type", "text/html; charset=utf-8"))
            .with_header(header("Cache-Control", "no-store"))
            .with_header(header("Referrer-Policy", "no-referrer"))
            .with_header(header("X-Frame-Options", "DENY"))
            .with_header(header("Content-Security-Policy", "frame-ancestors 'none'"));
        let _ = req.respond(resp);
        return;
    }
    if !(path.starts_with("/api/") || path == "/mcp") {
        return respond_json(req, 404, &json!({"error": "not found"}));
    }
    match role(&req, sh) {
        None => return respond_json(req, 401, &json!({"error": "missing or wrong bearer token"})),
        Some(Role::Mcp) if !mcp_allowed(&method, path) => {
            return respond_json(
                req,
                403,
                &json!({"error": "the MCP token can read and propose plans; approving needs the admin token"}),
            )
        }
        _ => {}
    }

    let mut body = String::new();
    if req
        .as_reader()
        .take(1 << 20)
        .read_to_string(&mut body)
        .is_err()
    {
        return respond_json(req, 400, &json!({"error": "body is not UTF-8"}));
    }
    let body: Value = if body.trim().is_empty() {
        json!({})
    } else {
        match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(e) => {
                return respond_json(req, 400, &json!({"error": format!("invalid JSON: {e}")}))
            }
        }
    };

    if path == "/mcp" {
        if method != "POST" {
            return respond_json(
                req,
                405,
                &json!({"error": "use POST (no SSE stream offered)"}),
            );
        }
        return match mcp::handle(sh, &body) {
            Some(resp) => respond_json(req, 200, &resp),
            None => {
                let _ = req.respond(Response::empty(202));
            }
        };
    }

    let params = if method == "GET" {
        parse_query(query)
    } else {
        body
    };
    match api::route(sh, &method, path, &params) {
        Ok((status, v)) => respond_json(req, status, &v),
        Err(e) => respond_json(req, e.0, &json!({"error": e.1})),
    }
}

const USAGE: &str = "usage: mac-dir-stat serve [--bind 127.0.0.1] [--port 7878] [--root /]";

/// Entry point for `mac-dir-stat serve …`. Blocks forever on success.
pub fn run_cli(args: &[String]) -> i32 {
    let mut bind: IpAddr = [127, 0, 0, 1].into();
    let mut port = 7878u16;
    let mut root = PathBuf::from("/");
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut val = || it.next().cloned().unwrap_or_default();
        match a.as_str() {
            "--bind" => match val().parse() {
                Ok(ip) => bind = ip,
                Err(_) => return fail("--bind needs an IP address, e.g. 127.0.0.1 or 0.0.0.0"),
            },
            "--port" => match val().parse() {
                Ok(p) => port = p,
                Err(_) => return fail("--port needs a number"),
            },
            "--root" => root = PathBuf::from(val()),
            "-h" | "--help" => {
                println!("{USAGE}");
                return 0;
            }
            other => return fail(&format!("unknown argument {other}\n{USAGE}")),
        }
    }
    if !root.is_dir() {
        return fail(&format!("--root {} is not a folder", root.display()));
    }
    let h = match start(Config {
        bind,
        port,
        root: root.clone(),
        token: None,
        mcp_token: None,
        executor: None,
    }) {
        Ok(h) => h,
        Err(e) => return fail(&e),
    };
    let tty = unsafe { libc::isatty(1) } == 1;
    print!("{}", banner(&root, h.addr, &h.token, tty));
    if !bind.is_loopback() {
        println!("  Listening beyond this machine: replace {} with an address you can reach (e.g. its Tailscale IP).", h.addr.ip());
    }
    loop {
        std::thread::park();
    }
}

/// Start-up text. The token goes into the URL only on an interactive
/// terminal, so it doesn't end up in launchd or other log files.
pub fn banner(root: &std::path::Path, addr: SocketAddr, token: &str, tty: bool) -> String {
    let base = format!("http://{addr}");
    let path = |n| {
        token_path(n)
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    };
    let ui = if tty {
        format!("{base}/#token={token}")
    } else {
        format!(
            "{base}/   (paste the token from {})",
            path(ADMIN_TOKEN_FILE)
        )
    };
    format!(
        "MacDirStat server, scanning {}\n  Web UI: {ui}\n  API:    {base}/api/v1/openapi.json\n  MCP:    {base}/mcp   (Bearer token for AI clients: {})\n  Admin token (approves plans): {}\n",
        root.display(),
        path(MCP_TOKEN_FILE),
        path(ADMIN_TOKEN_FILE)
    )
}

fn fail(msg: &str) -> i32 {
    eprintln!("mac-dir-stat serve: {msg}");
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_prints_the_token_only_on_a_terminal() {
        let addr: SocketAddr = "127.0.0.1:7878".parse().unwrap();
        let quiet = banner(std::path::Path::new("/"), addr, "SECRET123", false);
        assert!(!quiet.contains("SECRET123"));
        assert!(quiet.contains("server-token") && quiet.contains("mcp-token"));
        assert!(
            banner(std::path::Path::new("/"), addr, "SECRET123", true).contains("#token=SECRET123")
        );
    }

    #[test]
    fn mcp_token_scope() {
        assert!(mcp_allowed("POST", "/mcp"));
        assert!(mcp_allowed("GET", "/api/v1/plans/3"));
        assert!(mcp_allowed("POST", "/api/v1/plans"));
        assert!(!mcp_allowed("POST", "/api/v1/plans/3/approve"));
        assert!(!mcp_allowed("POST", "/api/v1/plans/3/reject"));
        assert!(!mcp_allowed("POST", "/api/v1/scan"));
        assert!(!mcp_allowed("GET", "/api/v1/mcp-token"));
    }

    #[test]
    fn query_parsing_decodes() {
        let q = parse_query("pattern=node_modules&path=%2FUsers%2Fme%20x&n=5&bad=%zz&e=");
        assert_eq!(q["pattern"], "node_modules");
        assert_eq!(q["path"], "/Users/me x");
        assert_eq!(q["n"], "5");
        assert_eq!(q["bad"], "%zz");
        assert_eq!(q["e"], "");
    }
}
