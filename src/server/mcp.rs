//! MCP over Streamable HTTP, stateless, JSON responses only.
//!
//! The tools read the scan and can *propose* plans. There is deliberately no
//! tool that approves, rejects or executes anything: the AI proposes, the
//! human approves in the web UI or GUI.

use super::{api, Shared};
use serde_json::{json, Value};
use std::sync::Arc;

const PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

pub fn tools() -> Value {
    let none = json!({"type": "object", "properties": {}});
    let op = json!({
        "type": "object",
        "required": ["action", "path"],
        "properties": {
            "action": {"type": "string", "enum": ["trash", "move"]},
            "path": {"type": "string", "description": "Absolute path of the file or folder"},
            "volume": {"type": "string", "description": "For move: destination volume root, e.g. /Volumes/External (see list_volumes)"},
            "link": {"type": "boolean", "description": "For move: leave a symlink at the original path"}
        }
    });
    json!([
        {"name": "list_volumes", "description": "Mounted volumes with filesystem type, free and total bytes. Non-boot volumes can be move destinations.", "inputSchema": none},
        {"name": "scan", "description": "Start a new scan (optionally of another root folder) and return scan status. Poll with get_tree or top_items once has_result is true.",
         "inputSchema": {"type": "object", "properties": {"root": {"type": "string"}}}},
        {"name": "get_tree", "description": "A folder from the last scan and its children, largest first.",
         "inputSchema": {"type": "object", "properties": {"path": {"type": "string", "description": "Absolute path; default is the scan root"}, "limit": {"type": "integer"}}}},
        {"name": "top_items", "description": "Largest files (kind=file) or largest content folders (kind=dir) in the last scan.",
         "inputSchema": {"type": "object", "properties": {"n": {"type": "integer"}, "kind": {"type": "string", "enum": ["file", "dir"]}}}},
        {"name": "find", "description": "Find items whose name matches a glob (* and ?), e.g. node_modules, .venv, target, *.dmg. Matches are not searched inside. Returns sizes, modified times and a total.",
         "inputSchema": {"type": "object", "required": ["pattern"], "properties": {"pattern": {"type": "string"}, "limit": {"type": "integer"}}}},
        {"name": "cleanup_suggestions", "description": "Known regenerable data (caches, build output, simulators...) grouped by category with totals.", "inputSchema": none},
        {"name": "propose_plan", "description": "Propose trash/move operations. Returns the plan with per-op validation errors and warnings. It does NOT run anything: the user reviews and approves it in MacDirStat.",
         "inputSchema": {"type": "object", "required": ["ops"], "properties": {"ops": {"type": "array", "items": op}, "note": {"type": "string", "description": "Why, in one or two sentences, shown to the user"}}}},
        {"name": "list_plans", "description": "All plans with their status (pending, approved, done, failed, rejected).", "inputSchema": none},
        {"name": "get_plan", "description": "One plan with per-op results.",
         "inputSchema": {"type": "object", "required": ["id"], "properties": {"id": {"type": "integer"}}}}
    ])
}

fn call_tool(sh: &Arc<Shared>, name: &str, args: &Value) -> Option<Result<Value, String>> {
    let r = match name {
        "list_volumes" => Ok(api::volumes()),
        "scan" => api::rescan(sh, args["root"].as_str().map(Into::into)),
        "get_tree" => api::tree(sh, args),
        "top_items" => api::top(sh, args),
        "find" => api::find(sh, args),
        "cleanup_suggestions" => api::cleanup(sh),
        "propose_plan" => api::propose(sh, args, "mcp"),
        "list_plans" => Ok(api::list_plans(sh)),
        "get_plan" => api::get_plan(sh, args["id"].as_u64().unwrap_or(0)),
        _ => return None,
    };
    Some(r.map_err(|e| e.1))
}

/// One JSON-RPC message in, `None` for notifications (HTTP 202, no body).
pub fn handle(sh: &Arc<Shared>, msg: &Value) -> Option<Value> {
    if !msg.is_object() {
        return Some(
            json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32600, "message": "send one JSON-RPC object per request (no batches)"}}),
        );
    }
    // Notifications (initialized, cancelled, …) have no id and get no reply.
    let id = msg.get("id").cloned()?;
    let method = msg["method"].as_str().unwrap_or("");
    let reply = |result: Result<Value, (i64, String)>| {
        Some(match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err((code, m)) => {
                json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": m}})
            }
        })
    };
    match method {
        "initialize" => {
            let asked = msg["params"]["protocolVersion"].as_str().unwrap_or("");
            let version = PROTOCOL_VERSIONS
                .iter()
                .find(|v| **v == asked)
                .unwrap_or(&PROTOCOL_VERSIONS[0]);
            reply(Ok(json!({
                "protocolVersion": version,
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "macdirstat", "version": env!("CARGO_PKG_VERSION")},
                "instructions": "Disk usage of this Mac from MacDirStat. Explore with top_items, find, cleanup_suggestions and get_tree; check destinations with list_volumes; then call propose_plan. You cannot run plans: the user approves them in MacDirStat."
            })))
        }
        "ping" => reply(Ok(json!({}))),
        "tools/list" => reply(Ok(json!({"tools": tools()}))),
        "tools/call" => {
            let name = msg["params"]["name"].as_str().unwrap_or("");
            let args = msg["params"].get("arguments").cloned().unwrap_or(json!({}));
            match call_tool(sh, name, &args) {
                None => reply(Err((-32602, format!("unknown tool: {name}")))),
                Some(r) => {
                    let (text, is_error) = match r {
                        Ok(v) => (serde_json::to_string_pretty(&v).unwrap_or_default(), false),
                        Err(e) => (e, true),
                    };
                    reply(Ok(
                        json!({"content": [{"type": "text", "text": text}], "isError": is_error}),
                    ))
                }
            }
        }
        _ => reply(Err((-32601, format!("method not found: {method}")))),
    }
}
