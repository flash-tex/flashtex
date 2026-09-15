//! Cross-language gate for the `dl2-canon-1` header digest of a v2
//! diagnostic's `suggestion` (GH-277): the Rust producer
//! (`delta::header_digest`) must produce the `expected_header_digest`
//! recorded in `protocol/fixtures/display-list-v2-delta-digest.json` for
//! each case, and the Swift consumer (`DisplayListDelta.headerDigest`)
//! asserts the same digests from the same file. The four cases share one
//! minimal display list and vary only `diagnostics`, so this isolates
//! exactly the diagnostic/suggestion behavior under
//! `Wire { diagnostics: true, .. }`.

use flashtex_compiler::json::{self, Value};
use flashtex_render_pipeline::delta::header_digest;
use flashtex_render_pipeline::display::{Diagnostic, DisplayList, DocumentResource, FontResource, Severity, SourceRange, Wire};

fn str_field(v: &Value, key: &str, at: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or_else(|| panic!("{at}: missing string field `{key}`")).to_string()
}

fn u64_field(v: &Value, key: &str, at: &str) -> u64 {
    u64::try_from(v.get(key).and_then(|x| x.as_i64()).unwrap_or_else(|| panic!("{at}: missing integer field `{key}`")))
        .unwrap_or_else(|_| panic!("{at}: field `{key}` is negative"))
}

fn arr_field<'a>(v: &'a Value, key: &str, at: &str) -> &'a Vec<Value> {
    v.get(key).and_then(|x| x.as_arr()).unwrap_or_else(|| panic!("{at}: missing array field `{key}`"))
}

fn source_range(v: &Value, at: &str) -> SourceRange {
    SourceRange {
        path: str_field(v, "path", at).into(),
        start_byte: u64_field(v, "start_byte", at) as usize,
        end_byte: u64_field(v, "end_byte", at) as usize,
    }
}

fn diagnostic(v: &Value, at: &str) -> Diagnostic {
    let severity = match str_field(v, "severity", at).as_str() {
        "warning" => Severity::Warning,
        "error" => Severity::Error,
        other => panic!("{at}: unknown severity `{other}`"),
    };
    Diagnostic {
        code: str_field(v, "code", at),
        message: str_field(v, "message", at),
        severity,
        sources: arr_field(v, "sources", at).iter().map(|s| source_range(s, at)).collect(),
        recovery: None,
        suggestion: v.get("suggestion").and_then(|x| x.as_str()).map(String::from),
    }
}

fn hex(digest: &[u8; 32]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn header_digests_match_fixture_for_diagnostics_wire() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../protocol/fixtures/display-list-v2-delta-digest.json");
    let text = std::fs::read_to_string(path).expect("read delta-digest fixture");
    let root = json::parse(&text).expect("parse delta-digest fixture");
    let wire_json = root.get("wire").expect("fixture `wire`");
    assert!(matches!(wire_json.get("diagnostics"), Some(Value::Bool(true))), "fixture must record Wire.diagnostics = true");
    let wire = Wire { diagnostics: true, ..Default::default() };
    let cases = root.get("cases").and_then(|x| x.as_arr()).expect("fixture `cases`");
    assert_eq!(cases.len(), 4, "fixture must carry the four diagnostic/suggestion cases");
    for case in cases {
        let name = str_field(case, "name", "<case>");
        let at = format!("case `{name}`");
        let payload = case
            .get("display_list")
            .and_then(|d| d.get("payload"))
            .unwrap_or_else(|| panic!("{at}: missing display_list.payload"));
        let pages = arr_field(payload, "pages", &at);
        assert!(pages.is_empty(), "{at}: fixture pages must stay empty so required_features cannot vary");
        let list = DisplayList {
            project_id: str_field(payload, "project_id", &at),
            revision: u64_field(payload, "revision", &at),
            documents: arr_field(payload, "documents", &at)
                .iter()
                .map(|d| DocumentResource {
                    path: str_field(d, "path", &at),
                    revision: u64_field(d, "revision", &at),
                    sha256: str_field(d, "sha256", &at),
                    byte_length: u64_field(d, "byte_length", &at),
                })
                .collect(),
            fonts: arr_field(payload, "fonts", &at)
                .iter()
                .map(|f| FontResource {
                    font_id: str_field(f, "font_id", &at).into(),
                    sha256: str_field(f, "sha256", &at),
                    byte_length: u64_field(f, "byte_length", &at),
                    format: str_field(f, "format", &at),
                    face_index: u64_field(f, "face_index", &at) as u32,
                    units_per_em: u64_field(f, "units_per_em", &at) as u32,
                    glyph_count: u64_field(f, "glyph_count", &at) as u32,
                    postscript_name: str_field(f, "postscript_name", &at),
                    path: None,
                })
                .collect(),
            pages: Vec::new(),
            diagnostics: arr_field(payload, "diagnostics", &at).iter().map(|d| diagnostic(d, &at)).collect(),
        };
        // The recorded feature sequence must be exactly what the producer hashes.
        let recorded: Vec<String> =
            arr_field(payload, "required_features", &at).iter().map(|f| f.as_str().expect("feature string").to_string()).collect();
        let computed: Vec<String> = list.required_features_wire(wire).into_iter().map(String::from).collect();
        assert_eq!(computed, recorded, "{at}: required_features drifted from what header_digest hashes");
        let expected = str_field(case, "expected_header_digest", &at);
        assert_eq!(hex(&header_digest(&list, wire)), expected, "{at}: header_digest mismatch");
    }
}
