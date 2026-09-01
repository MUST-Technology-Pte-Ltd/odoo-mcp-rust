//! odoo-mcp — a Model Context Protocol server for Odoo ERP, in Rust.
//!
//! Speaks MCP over stdio, so an MCP client (Claude Desktop, a Tauri app, any
//! agent runtime) launches it as a subprocess and talks JSON-RPC over the
//! pipe. Configuration is entirely by environment variable; see `odoo.rs`.
//!
//! Read-first by design: querying is always available, writing only when
//! explicitly enabled. The server can never exceed what its Odoo user may do.

mod mcp;
mod odoo;
mod tools;

fn main() {
    mcp::serve();
}
