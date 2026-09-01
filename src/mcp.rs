//! Minimal MCP server over stdio.
//!
//! MCP's stdio transport is newline-delimited JSON-RPC 2.0: one JSON object per
//! line on stdin, one response per line on stdout. That is small enough to
//! implement directly without an async runtime or an SDK, which keeps the
//! binary tiny and the dependency surface (and thus the audit surface) minimal.
//! The handshake and the three methods a tool server must answer —
//! `initialize`, `tools/list`, `tools/call` — are handled here; `ping` and the
//! `notifications/*` are acknowledged or ignored per spec.
//!
//! Migration note: swapping this file for the official `rmcp` SDK is a
//! localised change — `odoo.rs` and `tools.rs` do not depend on it.

use std::io::{BufRead, Write};

use serde_json::{json, Value};

use crate::odoo::Odoo;
use crate::tools;

const PROTOCOL_VERSION: &str = "2025-06-18";

pub fn serve() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    // The Odoo connection is lazy: built once, authenticated on first tool use,
    // so `initialize`/`tools/list` work even before credentials are reachable
    // and a connection problem surfaces as a tool error, not a dead server.
    let mut odoo = Odoo::from_env().ok();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if !l.trim().is_empty() => l,
            Ok(_) => continue,
            Err(_) => break,
        };
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => {
                write_msg(&mut out, error(Value::Null, -32700, "parse error"));
                continue;
            }
        };

        // A message with no "id" is a notification: act if needed, never reply.
        let id = req.get("id").cloned();
        let method = req.get("method").and_then(Value::as_str).unwrap_or("");

        if id.is_none() {
            // notifications/initialized, notifications/cancelled, ... — ignore.
            continue;
        }
        let id = id.unwrap();
        let params = req.get("params").cloned().unwrap_or(Value::Null);

        let response = match method {
            "initialize" => ok(id, initialize_result()),
            "ping" => ok(id, json!({})),
            "tools/list" => ok(id, tools_list()),
            "tools/call" => tools_call(&mut odoo, id, &params),
            other => error(id, -32601, &format!("method not found: {other}")),
        };
        write_msg(&mut out, response);
    }
}

fn write_msg(out: &mut impl Write, msg: Value) {
    // One compact line per message — no embedded newlines in the JSON.
    let _ = writeln!(out, "{}", serde_json::to_string(&msg).unwrap());
    let _ = out.flush();
}

fn ok(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": {
            "name": "odoo-mcp",
            "title": "Odoo MCP (Rust) by MUST Technology",
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": "Query an Odoo ERP. Start with odoo_fields_get to learn a model's schema, then odoo_search_read. Domains use Odoo's list syntax, e.g. [[\"customer_rank\",\">\",0]]."
    })
}

fn tools_list() -> Value {
    let tools: Vec<Value> = tools::definitions()
        .into_iter()
        .map(|d| json!({"name": d.name, "description": d.description, "inputSchema": d.schema}))
        .collect();
    json!({ "tools": tools })
}

fn tools_call(odoo: &mut Option<Odoo>, id: Value, params: &Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let client = match odoo {
        Some(c) => c,
        None => return tool_error(id, "Odoo is not configured — set ODOO_URL, ODOO_DB, ODOO_USERNAME and a credential"),
    };

    match tools::call(client, name, &args) {
        Ok(value) => {
            // MCP wraps tool output as content blocks. We return the data as a
            // pretty JSON text block — agents parse it, humans can read it.
            let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
            ok(id, json!({"content": [{"type": "text", "text": text}], "isError": false}))
        }
        Err(e) => tool_error(id, &e.0),
    }
}

/// A tool-level failure is a *successful* JSON-RPC response with isError=true,
/// per MCP — so the agent sees the message and can react, rather than the whole
/// call collapsing into a protocol error.
fn tool_error(id: Value, message: &str) -> Value {
    ok(id, json!({"content": [{"type": "text", "text": message}], "isError": true}))
}
