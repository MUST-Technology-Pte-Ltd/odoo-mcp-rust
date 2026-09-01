//! A thin Odoo JSON-RPC client.
//!
//! Odoo exposes an external API at `POST /jsonrpc`. Two services matter:
//! `common.authenticate` (login -> numeric uid) and `object.execute_kw`
//! (call any model method the authenticated user is allowed to). This module
//! wraps exactly those two and nothing more; the MCP tools in `tools.rs`
//! decide which model methods to expose.
//!
//! Credentials come from the environment so they never touch the wire log or a
//! config file in a repo:
//!   ODOO_URL       e.g. https://gmf.cloud.must.com.sg
//!   ODOO_DB        database name, e.g. gmf
//!   ODOO_USERNAME  login
//!   ODOO_API_KEY   an API key from Settings > Account Security (preferred), or
//!   ODOO_PASSWORD  a password (fallback)
//!
//! The credential used for execute_kw is whatever authenticated, so **the
//! server can only ever do what that Odoo user is allowed to do**. Odoo's own
//! access rules are the security boundary — this client adds no privilege.

use serde_json::{json, Value};

pub struct Odoo {
    url: String,
    db: String,
    username: String,
    secret: String,
    uid: Option<i64>,
}

#[derive(Debug)]
pub struct OdooError(pub String);

impl std::fmt::Display for OdooError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Odoo {
    /// Build from environment. Returns an error naming the first missing var.
    pub fn from_env() -> Result<Self, OdooError> {
        let get = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let url = get("ODOO_URL").ok_or(OdooError("ODOO_URL is not set".into()))?;
        let db = get("ODOO_DB").ok_or(OdooError("ODOO_DB is not set".into()))?;
        let username =
            get("ODOO_USERNAME").ok_or(OdooError("ODOO_USERNAME is not set".into()))?;
        let secret = get("ODOO_API_KEY")
            .or_else(|| get("ODOO_PASSWORD"))
            .ok_or(OdooError("neither ODOO_API_KEY nor ODOO_PASSWORD is set".into()))?;
        Ok(Odoo { url: url.trim_end_matches('/').to_string(), db, username, secret, uid: None })
    }

    fn rpc(&self, service: &str, method: &str, args: Value) -> Result<Value, OdooError> {
        let body = json!({
            "jsonrpc": "2.0",
            "method": "call",
            "params": { "service": service, "method": method, "args": args },
            "id": 1
        });
        let resp: Value = ureq::post(&format!("{}/jsonrpc", self.url))
            .set("Content-Type", "application/json")
            .send_json(body)
            .map_err(|e| OdooError(format!("HTTP call to Odoo failed: {e}")))?
            .into_json()
            .map_err(|e| OdooError(format!("Odoo returned non-JSON: {e}")))?;

        if let Some(err) = resp.get("error") {
            // Surface Odoo's own message (access errors, bad model, etc.).
            let msg = err
                .pointer("/data/message")
                .and_then(Value::as_str)
                .or_else(|| err.get("message").and_then(Value::as_str))
                .unwrap_or("unknown Odoo error");
            return Err(OdooError(format!("Odoo error: {msg}")));
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Authenticate once and cache the uid.
    pub fn authenticate(&mut self) -> Result<i64, OdooError> {
        if let Some(uid) = self.uid {
            return Ok(uid);
        }
        let res = self.rpc(
            "common",
            "authenticate",
            json!([self.db, self.username, self.secret, {}]),
        )?;
        match res.as_i64() {
            Some(uid) if uid > 0 => {
                self.uid = Some(uid);
                Ok(uid)
            }
            _ => Err(OdooError(
                "authentication failed — check ODOO_DB / ODOO_USERNAME / credential".into(),
            )),
        }
    }

    /// `execute_kw(model, method, args, kwargs)` as the authenticated user.
    pub fn execute_kw(
        &mut self,
        model: &str,
        method: &str,
        args: Value,
        kwargs: Value,
    ) -> Result<Value, OdooError> {
        let uid = self.authenticate()?;
        self.rpc(
            "object",
            "execute_kw",
            json!([self.db, uid, self.secret, model, method, args, kwargs]),
        )
    }
}
