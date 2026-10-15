//! End-to-end checks of `mac-dir-stat serve` on an ephemeral port against a
//! temp directory. Approved ops run through a test executor that deletes the
//! file directly, so tests never touch the real Trash or other disks.

use mac_dir_stat::plan::{Action, Op};
use mac_dir_stat::server::{self, Config, Handle};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const TOKEN: &str = "test-token-0123456789abcdef0123456789abcdef";

fn fixture(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mds-srv-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("proj/node_modules/pkg")).unwrap();
    std::fs::write(d.join("proj/node_modules/pkg/index.js"), vec![1u8; 2 << 20]).unwrap();
    std::fs::write(d.join("big.dmg"), vec![2u8; 3 << 20]).unwrap();
    std::fs::write(d.join("small.txt"), b"hello").unwrap();
    d.canonicalize().unwrap()
}

fn start(root: &Path, runs: Arc<AtomicUsize>) -> Handle {
    let h = server::start(Config {
        bind: [127, 0, 0, 1].into(),
        port: 0,
        root: root.to_path_buf(),
        token: Some(TOKEN.into()),
        executor: Some(Arc::new(move |op: &Op| {
            runs.fetch_add(1, Ordering::SeqCst);
            match op.action {
                Action::Trash => std::fs::remove_file(&op.path)
                    .map(|_| "deleted".into())
                    .map_err(|e| e.to_string()),
                Action::Move { .. } => Err("moves are not run in tests".into()),
            }
        })),
    })
    .unwrap();
    for _ in 0..200 {
        if call(&h, "GET", "/api/v1/scan", None).1["has_result"] == true {
            return h;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    panic!("scan never finished");
}

fn raw(
    h: &Handle,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<&Value>,
) -> (u16, String) {
    let mut s = TcpStream::connect(h.addr).unwrap();
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    let auth = token
        .map(|t| format!("Authorization: Bearer {t}\r\n"))
        .unwrap_or_default();
    write!(
        s,
        "{method} {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n{auth}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    let status = out[9..12].parse().unwrap();
    let body = out
        .split_once("\r\n\r\n")
        .map(|x| x.1)
        .unwrap_or("")
        .to_string();
    (status, body)
}

fn call(h: &Handle, method: &str, path: &str, body: Option<&Value>) -> (u16, Value) {
    let (st, b) = raw(h, method, path, Some(TOKEN), body);
    (st, serde_json::from_str(&b).unwrap_or(Value::Null))
}

fn mcp(h: &Handle, msg: Value) -> (u16, Value) {
    call(h, "POST", "/mcp", Some(&msg))
}

fn wait_plan(h: &Handle, id: u64) -> Value {
    for _ in 0..200 {
        let p = call(h, "GET", &format!("/api/v1/plans/{id}"), None).1;
        if p["status"] != "approved" {
            return p;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    panic!("plan {id} never finished");
}

#[test]
fn token_is_required() {
    let root = fixture("auth");
    let h = start(&root, Arc::default());
    assert_eq!(raw(&h, "GET", "/api/v1/volumes", None, None).0, 401);
    assert_eq!(
        raw(&h, "GET", "/api/v1/volumes", Some("wrong"), None).0,
        401
    );
    assert_eq!(
        raw(&h, "POST", "/api/v1/plans", None, Some(&json!({"ops": []}))).0,
        401
    );
    assert_eq!(
        raw(
            &h,
            "POST",
            "/mcp",
            None,
            Some(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}))
        )
        .0,
        401
    );
    assert_eq!(raw(&h, "GET", "/api/v1/volumes", Some(TOKEN), None).0, 200);
    // The page itself is public; it holds no data.
    let (st, html) = raw(&h, "GET", "/", None, None);
    assert_eq!(st, 200);
    assert!(html.contains("MacDirStat"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rest_reads_the_scan() {
    let root = fixture("rest");
    let h = start(&root, Arc::default());

    let (_, spec) = call(&h, "GET", "/api/v1/openapi.json", None);
    assert_eq!(spec["openapi"], "3.1.0");
    assert!(spec["paths"]["/plans/{id}/approve"].is_object());

    let (_, v) = call(&h, "GET", "/api/v1/volumes", None);
    assert!(v["volumes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["boot"] == true && v["path"] == "/"));

    let (_, t) = call(&h, "GET", "/api/v1/tree", None);
    assert_eq!(t["node"]["size"], (5 << 20) + 5);
    assert_eq!(t["children"][0]["name"], "big.dmg");
    let proj = root.join("proj").display().to_string().replace(' ', "%20");
    let (_, t) = call(&h, "GET", &format!("/api/v1/tree?path={proj}"), None);
    assert_eq!(t["children"][0]["name"], "node_modules");
    assert_eq!(call(&h, "GET", "/api/v1/tree?path=/nowhere", None).0, 404);

    let (_, top) = call(&h, "GET", "/api/v1/top?n=1", None);
    assert_eq!(top["items"].as_array().unwrap().len(), 1);
    assert_eq!(top["items"][0]["name"], "big.dmg");
    let (_, top) = call(&h, "GET", "/api/v1/top?kind=dir", None);
    assert_eq!(top["items"][0]["name"], "pkg");

    let (_, f) = call(&h, "GET", "/api/v1/find?pattern=NODE_*", None);
    assert_eq!(f["count"], 1);
    assert_eq!(f["total_bytes"], 2 << 20);
    assert!(f["items"][0]["modified"].as_u64().unwrap() > 0);
    assert_eq!(call(&h, "GET", "/api/v1/find", None).0, 400);

    let (_, c) = call(&h, "GET", "/api/v1/cleanup", None);
    assert_eq!(c["groups"][0]["category"], "node_modules");
    assert_eq!(c["groups"][0]["total_bytes"], 2 << 20);

    let (_, s) = call(&h, "GET", "/api/v1/summary", None);
    let text = s["text"].as_str().unwrap();
    assert!(
        text.contains("big.dmg") && text.contains("node_modules") && text.contains("\"action\"")
    );

    assert_eq!(call(&h, "POST", "/api/v1/scan", None).0, 202);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn plans_run_only_when_approved() {
    let root = fixture("plans");
    let runs = Arc::new(AtomicUsize::new(0));
    let h = start(&root, runs.clone());
    let dmg = root.join("big.dmg").display().to_string();

    // Propose → pending, nothing runs.
    let (st, p) = call(
        &h,
        "POST",
        "/api/v1/plans",
        Some(&json!({"note": "t", "ops": [{"action": "trash", "path": dmg}]})),
    );
    assert_eq!(st, 201);
    assert_eq!(p["status"], "pending");
    assert_eq!(p["valid"], true);
    assert_eq!(p["total_bytes"], 3 << 20);
    assert_eq!(runs.load(Ordering::SeqCst), 0);
    let id = p["id"].as_u64().unwrap();

    // Approve → approved → done; the item leaves the scan.
    assert_eq!(
        call(&h, "POST", &format!("/api/v1/plans/{id}/approve"), None).0,
        202
    );
    let p = wait_plan(&h, id);
    assert_eq!(p["status"], "done", "{p}");
    assert_eq!(p["ops"][0]["result"]["ok"], true);
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    assert!(!Path::new(&dmg).exists());
    assert_eq!(
        call(&h, "GET", "/api/v1/find?pattern=*.dmg", None).1["count"],
        0
    );
    assert_eq!(
        call(&h, "POST", &format!("/api/v1/plans/{id}/approve"), None).0,
        409
    );

    // A failing op fails the plan.
    let txt = root.join("small.txt").display().to_string();
    let p = call(
        &h,
        "POST",
        "/api/v1/plans",
        Some(&json!({"ops": [{"action": "trash", "path": txt}]})),
    )
    .1;
    std::fs::remove_file(&txt).unwrap(); // vanishes after validation
    let id = p["id"].as_u64().unwrap();
    call(&h, "POST", &format!("/api/v1/plans/{id}/approve"), None);
    assert_eq!(wait_plan(&h, id)["status"], "failed");

    // Reject → rejected, cannot be approved.
    let nm = root.join("proj/node_modules").display().to_string();
    let p = call(
        &h,
        "POST",
        "/api/v1/plans",
        Some(&json!({"ops": [{"action": "trash", "path": nm}]})),
    )
    .1;
    let id = p["id"].as_u64().unwrap();
    assert_eq!(
        call(&h, "POST", &format!("/api/v1/plans/{id}/reject"), None).1["status"],
        "rejected"
    );
    assert_eq!(
        call(&h, "POST", &format!("/api/v1/plans/{id}/approve"), None).0,
        409
    );

    // Invalid ops are reported and block approval.
    let p = call(
        &h,
        "POST",
        "/api/v1/plans",
        Some(&json!({"ops": [
            {"action": "trash", "path": "/System"},
            {"action": "move", "path": nm, "volume": root.display().to_string()}
        ]})),
    )
    .1;
    assert_eq!(p["valid"], false);
    assert!(!p["ops"][0]["errors"].as_array().unwrap().is_empty());
    assert!(
        p["ops"][1]["errors"][0]
            .as_str()
            .unwrap()
            .contains("volume"),
        "{p}"
    );
    let id = p["id"].as_u64().unwrap();
    assert_eq!(
        call(&h, "POST", &format!("/api/v1/plans/{id}/approve"), None).0,
        409
    );
    assert_eq!(
        call(&h, "POST", "/api/v1/plans", Some(&json!({"ops": "x"}))).0,
        400
    );

    assert_eq!(
        call(&h, "GET", "/api/v1/plans", None).1["plans"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(runs.load(Ordering::SeqCst), 2);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn mcp_proposes_but_never_executes() {
    let root = fixture("mcp");
    let runs = Arc::new(AtomicUsize::new(0));
    let h = start(&root, runs.clone());

    let (_, r) = mcp(
        &h,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t", "version": "1"}}}),
    );
    assert_eq!(r["result"]["protocolVersion"], "2025-06-18");
    assert!(r["result"]["capabilities"]["tools"].is_object());
    assert_eq!(
        mcp(
            &h,
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
        )
        .0,
        202
    );

    let (_, r) = mcp(
        &h,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
    );
    let names: Vec<&str> = r["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "list_volumes",
            "scan",
            "get_tree",
            "top_items",
            "find",
            "cleanup_suggestions",
            "propose_plan",
            "list_plans",
            "get_plan"
        ]
    );
    for n in &names {
        for banned in [
            "approve", "reject", "execute", "run", "delete", "trash", "move",
        ] {
            assert!(!n.contains(banned), "MCP tool {n} looks like it can act");
        }
    }

    let (_, r) = mcp(
        &h,
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "top_items", "arguments": {"n": 1}}}),
    );
    assert!(r["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("big.dmg"));

    let dmg = root.join("big.dmg").display().to_string();
    let (_, r) = mcp(
        &h,
        json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "propose_plan", "arguments": {"note": "old installer", "ops": [{"action": "trash", "path": dmg}]}}}),
    );
    assert_eq!(r["result"]["isError"], false);
    let plan: Value =
        serde_json::from_str(r["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(plan["status"], "pending");
    assert_eq!(plan["origin"], "mcp");
    let id = plan["id"].as_u64().unwrap();

    // Asking for an execute tool is a protocol error; nothing runs.
    for name in ["approve_plan", "execute_plan", "run_plan"] {
        let (_, r) = mcp(
            &h,
            json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": name, "arguments": {"id": id}}}),
        );
        assert_eq!(r["error"]["code"], -32602);
    }
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(runs.load(Ordering::SeqCst), 0);
    assert!(Path::new(&dmg).exists());
    assert_eq!(
        call(&h, "GET", &format!("/api/v1/plans/{id}"), None).1["status"],
        "pending"
    );

    let (_, r) = mcp(
        &h,
        json!({"jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": {"name": "find", "arguments": {}}}),
    );
    assert_eq!(r["result"]["isError"], true);
    assert_eq!(
        mcp(&h, json!({"jsonrpc": "2.0", "id": 7, "method": "nope"})).1["error"]["code"],
        -32601
    );
    assert_eq!(raw(&h, "GET", "/mcp", Some(TOKEN), None).0, 405);
    std::fs::remove_dir_all(root).unwrap();
}
