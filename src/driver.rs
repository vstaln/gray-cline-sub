//! cline ACP driver: wire-level quirks for the shared bridge.

use acp_bridge::acp::{AcpClient, SpawnSpec};
use acp_bridge::session::{Driver, PermissionDecision};
use serde_json::{json, Value};

pub struct ClineDriver;

impl Driver for ClineDriver {
    fn spawn_spec(&self, stage_dir: &std::path::Path) -> SpawnSpec {
        SpawnSpec {
            command: crate::setup::resolve_command().unwrap_or_else(|| "cline".to_string()),
            args: crate::setup::acp_argv(),
            cwd: stage_dir.to_path_buf(),
            env: Vec::new(),
        }
    }

    fn client_info(&self) -> (&'static str, &'static str) {
        ("gray-cline-sub", env!("CARGO_PKG_VERSION"))
    }

    fn client_capabilities(&self) -> Value {
        json!({
            "fs": {"readTextFile": false, "writeTextFile": false},
            "terminal": false,
            "_meta": {"parameterizedModelPicker": true},
        })
    }

    fn auth_method_id(&self) -> Option<&'static str> {
        None
    }

    /// Cline's "plan" mode is read-only Q&A; "act" is default. Model role
    /// wants the lightest mode; GRAY_CLINE_SUB_MODE overrides.
    fn session_mode(&self) -> Option<&'static str> {
        let mode = std::env::var("GRAY_CLINE_SUB_MODE").ok()?;
        Some(match mode.trim() {
            "plan" => "plan",
            "act" => "act",
            other => Box::leak(other.to_string().into_boxed_str()),
        })
    }

    /// No host user-question surface for a pure model provider: answer
    /// cline's question prompts with an empty map, accept plans silently.
    fn ext_request(&self, method: &str, _params: &Value) -> Option<Value> {
        match method {
            "cline/ask_question" | "ask_followup_question" => Some(json!({"answers": {}})),
            "cline/create_plan" | "cline/plan_response" => Some(json!({"accepted": true})),
            _ => None,
        }
    }

    /// Only the host's own tools (mcp__gray__*) may run. Cline's native
    /// tools are denied — the sidecar is a model, not an agent runtime.
    fn permission_option(&self, params: &Value) -> PermissionDecision {
        let title = params
            .get("toolCall")
            .and_then(|t| t.get("title"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let kind = params
            .get("toolCall")
            .and_then(|t| t.get("kind"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let allow = title.contains("mcp__gray__")
            || title.contains("mcp_gray__")
            || title.contains("gray/")
            || title.starts_with("gray ")
            || kind.contains("mcp__gray__");
        if !allow {
            return PermissionDecision::Deny;
        }
        let id = params
            .get("options")
            .and_then(Value::as_array)
            .and_then(|o| {
                o.iter()
                    .find(|opt| opt.get("kind").and_then(Value::as_str) == Some("allow_once"))
                    .or_else(|| {
                        o.iter().find(|opt| {
                            opt.get("kind").and_then(Value::as_str) == Some("allow_always")
                        })
                    })
            })
            .and_then(|opt| opt.get("optionId").and_then(Value::as_str))
            .unwrap_or("allow_once")
            .to_string();
        PermissionDecision::Allow(id)
    }

    fn apply_model(
        &self,
        client: &mut AcpClient,
        session_id: &str,
        session_new_result: &Value,
        model: &str,
        effort: Option<&str>,
        overrides: &[(String, String)],
    ) -> Result<(), String> {
        if let Err(e) = client.set_model(session_id, model, Value::Null) {
            if e.contains("-32601") || e.contains("not found") || e.contains("Invalid") {
                // set_model may not exist on this build — fall back to a
                // model-shaped configOption from session/new.
                if set_model_config_option(client, session_id, session_new_result, model) {
                    return apply_overrides(client, session_id, session_new_result, effort, overrides);
                }
                return Err(format!("unknown cline model `{model}`"));
            }
            return Err(format!("cline set_model: {e}"));
        }
        apply_overrides(client, session_id, session_new_result, effort, overrides)
    }
}

fn apply_overrides(
    client: &mut AcpClient,
    session_id: &str,
    new_result: &Value,
    effort: Option<&str>,
    overrides: &[(String, String)],
) -> Result<(), String> {
    for (key, value) in overrides {
        let Some(option_id) = find_config_option(new_result, key) else {
            continue;
        };
        let _ = client.set_config_option(session_id, &option_id, json!(value));
    }
    if let Some(e) = effort {
        if !overrides.iter().any(|(k, _)| k == "effort") {
            if let Some(option_id) = find_config_option(new_result, "effort") {
                let _ = client.set_config_option(session_id, &option_id, json!(e));
            }
        }
    }
    Ok(())
}

fn set_model_config_option(
    client: &mut AcpClient,
    session_id: &str,
    new_result: &Value,
    model: &str,
) -> bool {
    let Some(option_id) = find_config_option(new_result, "model") else {
        return false;
    };
    client
        .set_config_option(session_id, &option_id, json!(model))
        .is_ok()
}

/// Match a bracket override key ("model"/"effort"/"mode"/"thinking")
/// against the session/new configOptions by id-or-name.
fn find_config_option(new_result: &Value, key: &str) -> Option<String> {
    let opts = new_result.get("configOptions")?.as_array()?;
    let want = key.to_lowercase();
    opts.iter()
        .find(|o| {
            let id = o.get("id").and_then(Value::as_str).unwrap_or("");
            let name = o.get("name").and_then(Value::as_str).unwrap_or("");
            let id_l = id.to_lowercase();
            let name_l = name.to_lowercase();
            match want.as_str() {
                "model" => {
                    id_l.contains("model") || name_l.contains("model")
                }
                "effort" => {
                    ["effort", "reasoning_effort", "reasoning", "thought_level"]
                        .contains(&id_l.as_str())
                        || name_l.contains("effort")
                        || name_l.contains("reasoning")
                }
                "mode" => id_l == "mode" || name_l == "mode",
                "thinking" => id_l == "thinking" || name_l.contains("thinking"),
                other => id_l == other,
            }
        })
        .and_then(|o| o.get("id").and_then(Value::as_str).map(str::to_string))
}
