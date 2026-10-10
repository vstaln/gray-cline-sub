//! `provider/usage`: Cline subscription card.
//!
//! Cline credits live behind the app.cline.bot account; the CLI (`cline
//! auth`, `cline config`) publishes no usage or quota counter. The card
//! reports the connection honestly with no fabricated windows.

use serde_json::{Value, json};

use crate::setup;

/// `provider/usage` entry: CLI presence is the whole signal.
pub fn handle(_params: &Value) -> Result<Value, String> {
    if setup::resolve_command().is_none() {
        return Err("cline CLI not installed".to_string());
    }
    Ok(json!({
        "available": true,
        "title": "Cline",
        "windows": [],
        "note": "credits live at app.cline.bot — the CLI publishes no usage feed",
    }))
}
