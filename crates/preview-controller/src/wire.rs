//! Move large validated results into helper envelopes without reserializing them to Value.
use flashtex_preview_controller::Preview;
use serde_json::{json, Value};

pub fn envelope(session: &str, id: Value, kind: &str, payload: Value) -> Value {
    let mut value = json!({"protocol_version":1,"session_id":session,"type":kind});
    value["id"] = id;
    value["payload"] = payload;
    value
}

pub fn preview_payload(preview: Preview) -> Value {
    let mut payload = json!({"kind":"preview","request_id":preview.request_id,
        "compile_revision":preview.compile_revision,
        "source_versions":preview.source_versions.documents,
        "missing_layout_capabilities":preview.missing_layout_capabilities,
        "runtime_total_ms":preview.runtime_total_ms,
        "controller_total_ms":preview.controller_total_ms});
    payload["result"] = preview.result;
    payload
}
