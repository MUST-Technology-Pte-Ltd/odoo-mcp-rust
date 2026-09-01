//! The MCP tool surface.
//!
//! Read tools are always available. Write tools (`odoo_create`, `odoo_write`)
//! are refused unless `ODOO_ALLOW_WRITE=1`, and even then only for models in
//! `ODOO_WRITE_MODELS` (comma-separated) when that variable is set. This is
//! belt-and-suspenders: Odoo's own access rules already gate the credential,
//! and this adds an explicit, auditable server-side switch on top — a
//! read-only deployment cannot be talked into writing by a prompt.

use serde_json::{json, Value};

use crate::odoo::{Odoo, OdooError};

/// One tool's advertised definition (for `tools/list`).
pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub schema: Value,
}

fn writes_allowed() -> bool {
    std::env::var("ODOO_ALLOW_WRITE").ok().as_deref() == Some("1")
}

fn model_writable(model: &str) -> bool {
    match std::env::var("ODOO_WRITE_MODELS") {
        Err(_) => true, // no allowlist set -> any model (still gated by ALLOW_WRITE)
        Ok(list) => list.split(',').map(str::trim).any(|m| m == model),
    }
}

pub fn definitions() -> Vec<ToolDef> {
    let model = json!({"type": "string", "description": "Odoo model, e.g. res.partner or sale.order"});
    let mut defs = vec![
        ToolDef {
            name: "odoo_search_read",
            description: "Search records and read fields in one call. Returns matching records as JSON.",
            schema: json!({
                "type": "object",
                "properties": {
                    "model": model,
                    "domain": {"type": "array", "description": "Odoo domain, e.g. [[\"customer_rank\",\">\",0]]. Omit or [] for all.", "default": []},
                    "fields": {"type": "array", "items": {"type": "string"}, "description": "Field names to return. Omit for a sensible default set."},
                    "limit": {"type": "integer", "description": "Max records (default 50).", "default": 50},
                    "order": {"type": "string", "description": "Sort, e.g. \"name asc\"."}
                },
                "required": ["model"],
                "additionalProperties": false
            }),
        },
        ToolDef {
            name: "odoo_read",
            description: "Read given record ids of a model.",
            schema: json!({
                "type": "object",
                "properties": {
                    "model": model,
                    "ids": {"type": "array", "items": {"type": "integer"}},
                    "fields": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["model", "ids"],
                "additionalProperties": false
            }),
        },
        ToolDef {
            name: "odoo_search_count",
            description: "Count records matching a domain.",
            schema: json!({
                "type": "object",
                "properties": {"model": model, "domain": {"type": "array", "default": []}},
                "required": ["model"],
                "additionalProperties": false
            }),
        },
        ToolDef {
            name: "odoo_fields_get",
            description: "List a model's fields with their types and labels — use this to discover the schema before querying.",
            schema: json!({
                "type": "object",
                "properties": {"model": model},
                "required": ["model"],
                "additionalProperties": false
            }),
        },
        ToolDef {
            name: "odoo_name_search",
            description: "Fuzzy-find records by display name; returns [id, name] pairs. Good for resolving a customer or product the user named.",
            schema: json!({
                "type": "object",
                "properties": {
                    "model": model,
                    "name": {"type": "string"},
                    "limit": {"type": "integer", "default": 10}
                },
                "required": ["model", "name"],
                "additionalProperties": false
            }),
        },
    ];

    if writes_allowed() {
        defs.push(ToolDef {
            name: "odoo_create",
            description: "Create a record. Only available when the server is started with write access enabled.",
            schema: json!({
                "type": "object",
                "properties": {"model": model, "values": {"type": "object"}},
                "required": ["model", "values"],
                "additionalProperties": false
            }),
        });
        defs.push(ToolDef {
            name: "odoo_write",
            description: "Update existing records. Only available when the server is started with write access enabled.",
            schema: json!({
                "type": "object",
                "properties": {
                    "model": json!({"type": "string"}),
                    "ids": {"type": "array", "items": {"type": "integer"}},
                    "values": {"type": "object"}
                },
                "required": ["model", "ids", "values"],
                "additionalProperties": false
            }),
        });
    }
    defs
}

fn arg_str<'a>(a: &'a Value, k: &str) -> Result<&'a str, OdooError> {
    a.get(k).and_then(Value::as_str).ok_or(OdooError(format!("missing string argument '{k}'")))
}

/// Dispatch a `tools/call`. Returns the JSON payload to embed in the MCP result.
pub fn call(odoo: &mut Odoo, name: &str, args: &Value) -> Result<Value, OdooError> {
    let default_domain = json!([]);
    match name {
        "odoo_search_read" => {
            let model = arg_str(args, "model")?;
            let domain = args.get("domain").unwrap_or(&default_domain).clone();
            let mut kwargs = serde_json::Map::new();
            if let Some(f) = args.get("fields") {
                kwargs.insert("fields".into(), f.clone());
            }
            let limit = args.get("limit").and_then(Value::as_i64).unwrap_or(50);
            kwargs.insert("limit".into(), json!(limit));
            if let Some(o) = args.get("order").and_then(Value::as_str) {
                kwargs.insert("order".into(), json!(o));
            }
            odoo.execute_kw(model, "search_read", json!([domain]), Value::Object(kwargs))
        }
        "odoo_read" => {
            let model = arg_str(args, "model")?;
            let ids = args.get("ids").cloned().unwrap_or(json!([]));
            let mut kwargs = serde_json::Map::new();
            if let Some(f) = args.get("fields") {
                kwargs.insert("fields".into(), f.clone());
            }
            odoo.execute_kw(model, "read", json!([ids]), Value::Object(kwargs))
        }
        "odoo_search_count" => {
            let model = arg_str(args, "model")?;
            let domain = args.get("domain").unwrap_or(&default_domain).clone();
            odoo.execute_kw(model, "search_count", json!([domain]), json!({}))
        }
        "odoo_fields_get" => {
            let model = arg_str(args, "model")?;
            odoo.execute_kw(
                model,
                "fields_get",
                json!([]),
                json!({"attributes": ["string", "type", "required", "help"]}),
            )
        }
        "odoo_name_search" => {
            let model = arg_str(args, "model")?;
            let name = arg_str(args, "name")?;
            let limit = args.get("limit").and_then(Value::as_i64).unwrap_or(10);
            odoo.execute_kw(model, "name_search", json!([name]), json!({"limit": limit}))
        }
        "odoo_create" | "odoo_write" => {
            if !writes_allowed() {
                return Err(OdooError(
                    "write tools are disabled — start the server with ODOO_ALLOW_WRITE=1".into(),
                ));
            }
            let model = arg_str(args, "model")?;
            if !model_writable(model) {
                return Err(OdooError(format!(
                    "model '{model}' is not in ODOO_WRITE_MODELS"
                )));
            }
            let values = args.get("values").cloned().unwrap_or(json!({}));
            if name == "odoo_create" {
                odoo.execute_kw(model, "create", json!([values]), json!({}))
            } else {
                let ids = args.get("ids").cloned().unwrap_or(json!([]));
                odoo.execute_kw(model, "write", json!([ids, values]), json!({}))
            }
        }
        other => Err(OdooError(format!("unknown tool '{other}'"))),
    }
}
