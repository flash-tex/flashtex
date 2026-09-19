//! `payload.fonts` on the runtime-v1 compile request: the project
//! manifest's `[fonts]` table, sent by the caller that read `flashtex.toml`
//! (`docs/proposals/packages-fonts-manifest.md` §2.4, `RenderOptions::fonts`).
//!
//! The worker's half of the contract: the object is accepted and applied
//! with the documented precedence (a document `\setmainfont` still wins), a
//! request without it is byte-identical to before the field existed, and a
//! malformed value is a `failed` reply naming the member, never a silent
//! fall-back to the class fonts.
//!
//! Hermetic over a staged copy of the bundled Latin Modern faces, whose
//! `name` tables call them "Latin Modern Roman"/"Latin Modern Sans"/"Latin
//! Modern Mono"; the Helvetica case uses this Mac's own font and skips (and
//! says so) where it is not installed.

mod common;

use std::path::{Path, PathBuf};

use flashtex_compiler::json::{self, Value};
use flashtex_font_engine::Face;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{protocol, FontSet, RenderOptions};

/// A staged directory of Latin Modern faces for the index (see
/// `fontspec_named_fonts.rs`).
struct Staged(PathBuf);

impl Staged {
    fn new(tag: &str) -> Staged {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts");
        let dir = std::env::temp_dir().join(format!("flashtex-request-fonts-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["lmroman10-regular.otf", "lmroman10-bold.otf", "lmsans10-regular.otf", "lmmono10-regular.otf"] {
            std::fs::copy(src.join(f), dir.join(f)).unwrap_or_else(|e| panic!("{f}: {e}"));
        }
        Staged(dir)
    }

    fn fonts(&self) -> FontSet {
        FontSet::with_default_dirs(&[]).with_index_dirs(vec![self.0.clone()])
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// One compile line; `fonts` is the raw JSON of the field, or `None` for no field.
fn compile_line(text: &str, fonts: Option<&str>) -> String {
    let doc = format!(r#"{{"path":"main.tex","text":{}}}"#, json::write(&json::str_(text)));
    let fonts_field = fonts.map_or(String::new(), |f| format!(r#","fonts":{f}"#));
    format!(
        r#"{{"protocol_version":1,"id":"f1","type":"compile","payload":{{"project_id":"p","revision":1,"entry_path":"main.tex","documents":[{doc}]{fonts_field}}}}}"#
    )
}

fn handle(fonts: &FontSet, text: &str, field: Option<&str>) -> protocol::Reply {
    protocol::handle_line(&compile_line(text, field), fonts, &RenderOptions::default(), None)
}

fn reply(fonts: &FontSet, text: &str, field: Option<&str>) -> Value {
    let line = handle(fonts, text, field).line;
    json::parse(&line).unwrap_or_else(|e| panic!("reply is not JSON: {}\n{line}", e.0))
}

fn doc(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{fontspec}}\n{preamble}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

/// `(text, PostScript name)` of every glyph run of the rendered v2 list.
fn runs(reply: &protocol::Reply) -> Vec<(String, String)> {
    let rendered = reply.rendered.as_ref().unwrap_or_else(|| panic!("no render: {}", reply.line));
    let mut out = Vec::new();
    for page in &rendered.v2.pages {
        for it in page.resident_items() {
            if let Item::GlyphRun(run) = it {
                let font = rendered.v2.fonts.iter().find(|f| f.font_id == run.font_id).expect("run's font is published");
                out.push((run.text.clone(), font.postscript_name.clone()));
            }
        }
    }
    out
}

fn font_of(runs: &[(String, String)], word: &str) -> String {
    runs.iter().find(|(t, _)| t == word).unwrap_or_else(|| panic!("no run {word:?} in {runs:?}")).1.clone()
}

#[test]
fn fonts_text_selects_the_body_family_and_the_document_still_overrides_it() {
    if !common::lm_available() {
        return;
    }
    let s = Staged::new("apply");
    let fonts = s.fonts();
    let plain = doc("", "Body \\texttt{mono} \\textsf{sans}");
    let field = Some(r#"{"text":"Latin Modern Sans","mono":"Latin Modern Roman"}"#);
    let r = handle(&fonts, &plain, field);
    assert!(r.line.contains(r#""status":"ok""#), "{}", r.line);
    let items = runs(&r);
    assert_eq!(font_of(&items, "Body"), "LMSans10-Regular");
    assert_eq!(font_of(&items, "mono"), "LMRoman10-Regular");
    // `\textsf` has no named sans: the class font's sans as before.
    assert_eq!(font_of(&items, "sans"), "LMSans10-Regular");
    // A document `\setmainfont` outranks the manifest (fontspec::apply).
    let over = doc("\\setmainfont{Latin Modern Mono}", "Body \\texttt{mono}");
    let items = runs(&handle(&fonts, &over, field));
    assert_eq!(font_of(&items, "Body"), "LMMono10-Regular");
    assert_eq!(font_of(&items, "mono"), "LMRoman10-Regular");
}

#[test]
fn a_request_without_fonts_is_byte_identical_to_one_with_an_empty_object() {
    if !common::lm_available() {
        return;
    }
    let s = Staged::new("absent");
    let fonts = s.fonts();
    let text = doc("", "A plain paragraph with \\textbf{bold} and \\texttt{mono}, and $x^2$.");
    let absent = protocol::handle_line(&compile_line(&text, None), &fonts, &RenderOptions::default(), None).line;
    let empty = protocol::handle_line(&compile_line(&text, Some("{}")), &fonts, &RenderOptions::default(), None).line;
    let nulls = protocol::handle_line(&compile_line(&text, Some(r#"{"text":null,"math":null,"mono":"","sans":null}"#)), &fonts, &RenderOptions::default(), None).line;
    assert_eq!(absent, empty);
    assert_eq!(absent, nulls);
    // And the class fonts: nothing named, so the body is Latin Modern Roman.
    let items = runs(&handle(&fonts, &text, None));
    assert_eq!(font_of(&items, "A"), "LMRoman10-Regular", "{items:?}");
}

#[test]
fn malformed_fonts_are_refused_naming_the_member() {
    if !common::lm_available() {
        return;
    }
    let s = Staged::new("refused");
    let fonts = s.fonts();
    let text = doc("", "x");
    for (field, expect) in [
        (r#""Latin Modern Sans""#, "'fonts' must be an object"),
        (r#"["Latin Modern Sans"]"#, "'fonts' must be an object"),
        (r#"{"text":3}"#, "'fonts.text' must be a family name"),
        (r#"{"serif":"x"}"#, "unknown member"),
    ] {
        let r = reply(&fonts, &text, Some(field));
        let payload = r.get("payload").unwrap();
        assert_eq!(payload.get("status").and_then(Value::as_str), Some("failed"), "{field}: {}", json::write(&r));
        let message = json::write(payload);
        assert!(message.contains(expect), "{field}: expected {expect:?} in {message}");
        assert!(message.contains("serif") || !field.contains("serif"), "{field}: the member is named in {message}");
    }
}

#[test]
fn fonts_text_helvetica_renders_the_paragraph_in_helvetica() {
    if !common::lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let Some(installed) = fonts.index().find("Helvetica", 400, false).cloned() else {
        eprintln!("skipping: Helvetica is not installed on this machine");
        return;
    };
    let face = flashtex_font_engine::load_from_path_index(&installed.path, installed.face_index).unwrap();
    let r = handle(&fonts, &doc("", "Hello Typography, AVAST!"), Some(r#"{"text":"Helvetica"}"#));
    let items = runs(&r);
    assert_eq!(font_of(&items, "Hello"), face.postscript_name(), "{items:?}");
    assert_eq!(font_of(&items, "AVAST!"), face.postscript_name());
    // The published v2 font is the installed face, not a Latin Modern outline.
    let published = r.rendered.as_ref().unwrap().v2.fonts.iter().find(|f| f.postscript_name == face.postscript_name()).expect("Helvetica is published");
    assert_eq!(published.format, "static-truetype");
    // Without the field the same document is Latin Modern.
    assert_eq!(font_of(&runs(&handle(&fonts, &doc("", "Hello"), None)), "Hello"), "LMRoman10-Regular");
}
