//! NEP-297 events: standard "factor", version "1.0.0".
use near_sdk::env;
use near_sdk::serde_json::{Value, json};

pub const STANDARD: &str = "factor";
pub const VERSION: &str = "1.0.0";

pub fn emit(event: &str, data: Value) {
    let ev = json!({"standard": STANDARD, "version": VERSION, "event": event, "data": [data]});
    env::log_str(&format!("EVENT_JSON:{ev}"));
}
