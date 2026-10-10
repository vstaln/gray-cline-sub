//! Binary resolution + credential probing for the cline CLI.

use serde_json::{json, Value};
use std::process::{Command, Stdio};
use std::time::Duration;

pub const INSTALL_HINT: &str = "Cline CLI not found — install with `npm install -g cline`";
pub const LOGIN_HINT: &str =
    "Cline not signed in — run `cline auth` to log in, or `/cline` to connect";

const BINARY_ENV: &str = "GRAY_CLINE_SUB_COMMAND";
const DATA_DIR_ENV: &str = "GRAY_CLINE_SUB_DATA_DIR";

pub fn resolve_command() -> Option<String> {
    if let Ok(explicit) = std::env::var(BINARY_ENV) {
        let explicit = explicit.trim();
        if !explicit.is_empty() {
            return Some(explicit.to_string());
        }
    }
    which_on_path("cline")
}

/// `cline --version` as a liveness probe.
pub fn cli_version() -> Option<String> {
    let bin = resolve_command()?;
    let out = Command::new(&bin)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

/// Some(json) when the CLI reports a signed-in account — metadata for the
/// credential payload. `cline auth status` is the probe; a non-zero/empty
/// answer means logged out.
pub fn logged_in() -> Option<Value> {
    let bin = resolve_command()?;
    let out = Command::new(&bin)
        .args(["auth", "status"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let lower = text.to_lowercase();
    if lower.contains("not") || lower.contains("no ") || lower.contains("unauth") {
        return None;
    }
    // Try JSON first (email/account fields), else treat non-empty output
    // as signed-in.
    if let Ok(v) = serde_json::from_str::<Value>(&text) {
        let signed_in = v
            .get("authenticated")
            .or_else(|| v.get("loggedIn"))
            .or_else(|| v.get("signedIn"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if signed_in || v.get("email").and_then(Value::as_str).is_some() {
            return Some(json!({
                "raw": v,
            }));
        }
        return None;
    }
    if text.trim().is_empty() {
        return None;
    }
    Some(json!({
        "email": extract_email(&text).unwrap_or_default(),
        "raw": text.trim(),
    }))
}

fn extract_email(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|w| w.contains('@') && w.contains('.'))
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '@' && c != '.' && c != '-' && c != '_'))
        .map(str::to_string)
}

/// Spawn argv for the ACP child.
pub fn acp_argv() -> Vec<String> {
    let mut args = Vec::new();
    if let Ok(dir) = std::env::var(DATA_DIR_ENV) {
        let dir = dir.trim();
        if !dir.is_empty() {
            args.push("--data-dir".into());
            args.push(dir.into());
        }
    }
    args.push("--acp".into());
    args
}

/// How long `cline auth` may need for a browser sign-in.
pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(900);

/// PATH lookup for an executable name (no new deps).
fn which_on_path(name: &str) -> Option<String> {
    let path = std::env::var("PATH").ok()?;
    for dir in path.split(':') {
        let candidate = format!("{dir}/{name}");
        if std::fs::metadata(&candidate)
            .map(|m| m.is_file())
            .unwrap_or(false)
        {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(m) = std::fs::metadata(&candidate) {
                    if m.permissions().mode() & 0o111 == 0 {
                        continue;
                    }
                }
            }
            return Some(candidate);
        }
    }
    None
}
