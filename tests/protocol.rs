// Protocol-shape tests that need no live Odoo: they exercise the tool
// definitions and the write-gating logic, which are the parts most likely to
// regress silently.
use std::process::{Command, Stdio};
use std::io::Write;

/// Drive the built binary over stdio with a real initialize + tools/list and
/// assert the handshake and tool advertisement are well-formed.
#[test]
fn initialize_and_tools_list_over_stdio() {
    let bin = env!("CARGO_BIN_EXE_odoo-mcp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .env_remove("ODOO_ALLOW_WRITE")
        .spawn()
        .expect("spawn");

    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{}}}}"#).unwrap();
    writeln!(stdin, r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#).unwrap();
    writeln!(stdin, r#"{{"jsonrpc":"2.0","id":2,"method":"tools/list"}}"#).unwrap();
    drop(stdin); // EOF ends the server loop

    let out = child.wait_with_output().expect("wait");
    let text = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 2, "expected 2 responses, got: {text}");

    let init: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["name"], "odoo-mcp");

    let list: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    let names: Vec<String> = list["result"]["tools"]
        .as_array().unwrap().iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"odoo_search_read".to_string()));
    assert!(names.contains(&"odoo_fields_get".to_string()));
    // Writes disabled by default -> not advertised.
    assert!(!names.contains(&"odoo_create".to_string()), "write tools must be hidden by default");
}

#[test]
fn write_tools_appear_only_when_enabled() {
    let bin = env!("CARGO_BIN_EXE_odoo-mcp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped()).stdout(Stdio::piped())
        .env("ODOO_ALLOW_WRITE", "1")
        .spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"tools/list"}}"#).unwrap();
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| !l.trim().is_empty()).unwrap();
    let v: serde_json::Value = serde_json::from_str(line).unwrap();
    let names: Vec<String> = v["result"]["tools"].as_array().unwrap().iter()
        .map(|t| t["name"].as_str().unwrap().to_string()).collect();
    assert!(names.contains(&"odoo_create".to_string()));
    assert!(names.contains(&"odoo_write".to_string()));
}
