//! Model catalog: probe a throwaway `cline --acp` child — session/new may
//! carry the picker (`models` / `availableModels` / a model configOption).

use acp_bridge::acp::{AcpClient, SpawnSpec};
use acp_bridge::session::Driver;
use serde_json::{json, Value};

use crate::driver::ClineDriver;

/// Probe one session/new for whatever model surface this cline build has.
pub fn list_available() -> Option<Vec<Value>> {
    let bin = crate::setup::resolve_command()?;
    let spec = SpawnSpec {
        command: bin,
        args: crate::setup::acp_argv(),
        cwd: std::env::temp_dir(),
        env: Vec::new(),
    };
    let driver = ClineDriver;
    let mut client = AcpClient::spawn(&spec).ok()?;
    let _ = client
        .initialize(driver.client_info(), driver.client_capabilities())
        .ok()?;
    if let Some(method_id) = driver.auth_method_id() {
        let _ = client.authenticate(method_id);
    }
    let sess = client
        .session_new(&std::env::temp_dir().to_string_lossy(), json!([]))
        .ok()?;

    if let Some(models) = sess
        .get("models")
        .or_else(|| sess.get("availableModels"))
        .and_then(Value::as_array)
    {
        if !models.is_empty() {
            return Some(models.clone());
        }
    }
    // configOptions: a "model" picker whose options are the catalog.
    if let Some(opts) = sess.get("configOptions").and_then(Value::as_array) {
        for o in opts {
            let id = o.get("id").and_then(Value::as_str).unwrap_or("").to_lowercase();
            let name = o.get("name").and_then(Value::as_str).unwrap_or("").to_lowercase();
            if !id.contains("model") && !name.contains("model") {
                continue;
            }
            if let Some(options) = o.get("options").and_then(Value::as_array) {
                if !options.is_empty() {
                    return Some(options.clone());
                }
            }
        }
    }
    None
}

/// Env-provided fallback: GRAY_CLINE_SUB_MODELS="id1,id2,…".
fn env_catalog() -> Vec<Value> {
    std::env::var("GRAY_CLINE_SUB_MODELS")
        .ok()
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|id| json!({"id": id, "name": id}))
                .collect()
        })
        .unwrap_or_default()
}

/// catalog rows for provider/models: `{id, name}`.
pub fn catalog() -> Vec<Value> {
    let rows: Vec<Value> = list_available()
        .unwrap_or_default()
        .iter()
        .filter_map(|m| {
            let id = m
                .get("value")
                .or_else(|| m.get("modelId"))
                .or_else(|| m.get("id"))
                .and_then(Value::as_str)?;
            Some(json!({
                "id": id,
                "name": m.get("name").and_then(Value::as_str).unwrap_or(id),
            }))
        })
        .collect();
    if rows.is_empty() {
        env_catalog()
    } else {
        rows
    }
}
