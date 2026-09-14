//! GH-277: render-pipeline forwards the compiler's diagnostic `code` and
//! `suggestion`. runtime-v1 JSON carries both; display-list-v2 carries the
//! code only (`additionalProperties: false` on diagnostics).

mod common;

use common::*;
use flashtex_compiler::json::{self, Value};
use flashtex_render_pipeline::display;
use flashtex_render_pipeline::v1::{self, Capabilities};

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\\begin{{document}}{body}\\end{{document}}")
}

fn by_needle<'a>(r: &'a flashtex_render_pipeline::Rendered, needle: &str) -> &'a display::Diagnostic {
    r.v2
        .diagnostics
        .iter()
        .find(|d| d.message.contains(needle))
        .unwrap_or_else(|| panic!("no diagnostic containing {needle:?} in {:?}", r.v2.diagnostics))
}

fn field<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

fn v2_diag_objects(r: &flashtex_render_pipeline::Rendered) -> Vec<Value> {
    let parsed = json::parse(&r.v2.write_json("fwd")).expect("v2 JSON");
    parsed
        .get("payload")
        .and_then(|p| p.get("diagnostics"))
        .and_then(Value::as_arr)
        .cloned()
        .unwrap_or_default()
}

fn v1_diag_objects(r: &flashtex_render_pipeline::Rendered) -> Vec<Value> {
    let v1 = v1_of(r, Capabilities::default());
    let parsed = json::parse(&v1.write_envelope("fwd")).expect("v1 JSON");
    parsed
        .get("payload")
        .and_then(|p| p.get("diagnostics"))
        .and_then(Value::as_arr)
        .cloned()
        .unwrap_or_default()
}

#[test]
fn typo_alpah_is_unknown_command_with_alpha_suggestion_in_v1_only() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(&doc(r"Text \alpah here."));
    let d = by_needle(&r, r"\alpah");
    assert_eq!(d.code, "unknown_command", "{d:?}");
    assert_eq!(d.suggestion.as_deref(), Some(r"\alpha"), "{d:?}");

    let v1j = v1::diagnostic_json(d);
    assert_eq!(field(&v1j, "code"), Some("unknown_command"), "{v1j:?}");
    assert_eq!(field(&v1j, "suggestion"), Some(r"\alpha"), "{v1j:?}");

    let v2j = display::diagnostic_json(d);
    assert_eq!(field(&v2j, "code"), Some("unknown_command"), "{v2j:?}");
    assert!(v2j.get("suggestion").is_none(), "v2 must omit suggestion: {v2j:?}");

    let v2_wire = v2_diag_objects(&r);
    let v2_row = v2_wire
        .iter()
        .find(|row| field(row, "message").is_some_and(|m| m.contains(r"\alpah")))
        .expect("v2 wire diagnostic for \\alpah");
    assert_eq!(field(v2_row, "code"), Some("unknown_command"));
    assert!(v2_row.get("suggestion").is_none(), "{v2_row:?}");

    let v1_wire = v1_diag_objects(&r);
    let v1_row = v1_wire
        .iter()
        .find(|row| field(row, "message").is_some_and(|m| m.contains(r"\alpah")))
        .expect("v1 wire diagnostic for \\alpah");
    assert_eq!(field(v1_row, "code"), Some("unknown_command"));
    assert_eq!(field(v1_row, "suggestion"), Some(r"\alpha"));
}

#[test]
fn unimplemented_tikz_is_unsupported_feature() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(&doc(r"Text \tikz here."));
    let d = by_needle(&r, r"\tikz");
    assert_eq!(d.code, "unsupported_feature", "{d:?}");
    assert_eq!(d.suggestion, None, "{d:?}");

    let v1j = v1::diagnostic_json(d);
    assert_eq!(field(&v1j, "code"), Some("unsupported_feature"));
    assert!(v1j.get("suggestion").is_none(), "absent, not null: {v1j:?}");

    let v2j = display::diagnostic_json(d);
    assert_eq!(field(&v2j, "code"), Some("unsupported_feature"));
    assert!(v2j.get("suggestion").is_none(), "{v2j:?}");
}

/// The `[help: …]` clause this crate appends to a compiler message (its
/// structured `help`/`notes`, which display-list-v2 has no field for) must not
/// stop `packages::supersede_message` recognising the message it is appended
/// to. It did: that function ends its match with `strip_suffix(" are
/// recognised but not implemented")`, so a clause on the end made every
/// superseded `\usepackage` diagnostic leak into the render output —
/// `amsmath`, `microtype`, `geometry` and the rest alike, each telling the
/// reader this pipeline does not implement something it does.
///
/// The unit tests in `packages` cover the splitting; this covers the whole
/// render path, which is where the clause actually gets appended.
#[test]
fn packages_this_crate_sets_stay_superseded_once_a_help_clause_is_appended() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(
        "\\documentclass{article}\\usepackage{amsmath}\\usepackage{amssymb}\
         \\usepackage{microtype}\\usepackage{geometry}\\usepackage{graphicx}\
         \\begin{document}Body $x$.\\end{document}",
    );
    let leaked: Vec<&display::Diagnostic> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("are recognised but not implemented"))
        .collect();
    assert!(leaked.is_empty(), "every one of these packages is set by this crate: {leaked:?}");
}

/// The mixed case: packages this crate does *not* set stay reported, and the
/// ones it does are trimmed out of the list — rather than the whole message
/// being passed through because a clause defeated the match.
#[test]
fn a_mixed_package_list_is_trimmed_to_the_ones_this_crate_does_not_set() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(
        "\\documentclass{article}\\usepackage{amsmath,cite}\
         \\begin{document}Body $x$.\\end{document}",
    );
    let d = by_needle(&r, "are recognised but not implemented");
    assert!(
        d.message.starts_with("packages cite are recognised but not implemented"),
        "{d:?}"
    );
    assert!(!d.message.contains("amsmath"), "amsmath is set by this crate: {d:?}");
}
