//! Alternating helper wrapper benchmark; reads a captured compiler result, no provider calls.
#[path = "../src/wire.rs"]
mod wire;
use flashtex_preview_controller::Preview;
use flashtex_project_index::VersionSnapshot;
use serde_json::{json, Value};
use std::{collections::BTreeMap, hint::black_box, time::Instant};
fn preview(result: Value) -> Preview {
    Preview {
        missing_layout_capabilities: vec![],
        request_id: "fixture-request".into(),
        compile_revision: 7,
        source_versions: VersionSnapshot {
            project_id: "fixture".into(),
            generation: 7,
            documents: BTreeMap::from([("main.tex".into(), 7)]),
        },
        result,
        runtime_total_ms: 123.25,
        controller_total_ms: 124.5,
    }
}
fn legacy(preview: Preview) -> Value {
    let payload = json!({"kind":"preview","request_id":preview.request_id,"compile_revision":preview.compile_revision,"source_versions":preview.source_versions.documents,"result":preview.result,"missing_layout_capabilities":preview.missing_layout_capabilities,"runtime_total_ms":preview.runtime_total_ms,"controller_total_ms":preview.controller_total_ms});
    json!({"protocol_version":1,"session_id":"fixture-session","id":null,"type":"update","payload":payload})
}
fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("captured compiler result JSON");
    let input = std::fs::read(path).unwrap();
    let result: Value = serde_json::from_slice(&input).unwrap();
    let expected = serde_json::to_vec(&legacy(preview(result.clone()))).unwrap();
    for round in 0..20 {
        for moved in if round % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let preview = preview(result.clone()); // Fixture preparation outside timing.
            let start = Instant::now();
            let value = if moved {
                wire::envelope(
                    "fixture-session",
                    Value::Null,
                    "update",
                    wire::preview_payload(preview),
                )
            } else {
                legacy(preview)
            };
            let wrap_ms = start.elapsed().as_secs_f64() * 1000.0;
            let start = Instant::now();
            let bytes = serde_json::to_vec(&value).unwrap();
            let serialize_ms = start.elapsed().as_secs_f64() * 1000.0;
            assert_eq!(bytes, expected);
            println!(
                "{}",
                json!({"round":round,"moved":moved,"wrap_ms":wrap_ms,"serialize_ms":serialize_ms,"input_bytes":input.len(),"wire_bytes":bytes.len(),"exact_wire_equal":true})
            );
            black_box(value);
        }
    }
}
