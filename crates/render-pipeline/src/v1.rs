//! runtime-v1 `compile_result` fallback derived from the v2 display list,
//! with the negotiated layout capabilities of
//! `docs/contracts/runtime-v1-layout-capabilities.md`.
//!
//! One text item per glyph run (a styled word segment) with the exact
//! source span of its bytes, including the document path; one item per
//! glyph for math (each glyph has its own position). Rules become typed
//! `rule` items only when `rules-v1` was requested and accepted; otherwise
//! the legacy U+2500 approximation (a run of box-drawing characters at a
//! size whose 0.0857 em height equals the rule) is emitted, as the current
//! compiler does. Font hints ride on text items only when `font-hints-v1`
//! was accepted. Coordinates are PDF points (612x792 for US Letter), y
//! downward, exactly as the compiler's v1 output.

use flashtex_compiler::json::{self, Value};

use crate::display::{self, DisplayList, Severity, SourceRange};

pub const CAP_RULES: &str = "rules-v1";
pub const CAP_FONT_HINTS: &str = "font-hints-v1";
/// `docs/contracts/runtime-v1-display-list-v2.md` (mac-preview-v2 proposal,
/// ACKed by this lane): the rendering-v2 `display_list` envelope follows
/// the `compile_result` as one sibling line.
pub const CAP_DISPLAY_LIST: &str = "display-list-v2";
/// PROPOSAL (FT-063, `protocol/proposals/display-list-v2-image.md`): image
/// items on the `display_list` line. Accepted only together with
/// `display-list-v2`; without it image items are never serialised.
pub const CAP_IMAGES: &str = "display-list-v2-images";
/// PROPOSAL (`protocol/proposals/display-list-v2-device-color.md`): paints
/// carry `device_color` (pdfTeX's exact colour operands). Accepted only
/// together with `display-list-v2`.
pub const CAP_DEVICE_COLOR: &str = "display-list-v2-device-color";
/// PROPOSAL (`docs/proposals/display-list-v2-delta.md` r5): the sibling
/// line may be one `display_list_delta` against the consumer's acknowledged
/// installed base. Negotiated only next to `display-list-v2`; echoed only
/// on the replies that actually carry a delta (`protocol.rs`).
pub const CAP_DELTA: &str = crate::delta::CAP;
/// PROPOSAL (`protocol/proposals/display-list-v2-only.md`): when the
/// sibling (`display_list` or `display_list_delta`) is emitted, the
/// `compile_result` omits its `pages` items (status, diagnostics, revision
/// and the echoed capabilities stay). Negotiated only next to
/// `display-list-v2`; echoed only when the pages were actually elided.
pub const CAP_V2_ONLY: &str = "display-list-v2-only";
/// PROPOSAL (`protocol/proposals/display-list-v2-window.md`): the producer
/// materialises only the requested window of pages; every other page is
/// present with its frame and `"resident": false` and carries no `items`
/// key. Negotiated only next to `display-list-v2`, and only meaningful with
/// the request's `display_list_window`; echoed only on the replies that
/// actually carry a window, exactly as `-only` is echoed only when the pages
/// were actually elided.
///
/// Accepting it **declines** `display-list-v2-delta` (§7): a delta binds a
/// digest for every page and a windowed producer has none for a page it did
/// not materialise. The consumer reads `-delta`'s absence from the echo as
/// the decline it is and does not send `display_list_base`.
pub const CAP_WINDOW: &str = "display-list-v2-window";

/// Capabilities the producer accepted for one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Capabilities {
    pub rules: bool,
    pub font_hints: bool,
    pub display_list: bool,
    pub images: bool,
    pub device_color: bool,
    pub delta: bool,
    pub v2_only: bool,
    pub window: bool,
}

impl Capabilities {
    /// Accepts the known subset of a request's `layout_capabilities`, in
    /// request order, and returns the accepted list to echo. Unknown names
    /// are never accepted; nothing is accepted that was not requested.
    pub fn negotiate(requested: &[String]) -> (Capabilities, Vec<String>) {
        let mut caps = Capabilities::default();
        let mut accepted = Vec::new();
        // `-window` and `-delta` are mutually exclusive in r1 and the window
        // wins, so whether the window was asked for has to be known before
        // the request-order walk reaches `-delta`.
        let with_display_list = requested.iter().any(|c| c == CAP_DISPLAY_LIST);
        let windowing = with_display_list && requested.iter().any(|c| c == CAP_WINDOW);
        for r in requested {
            match r.as_str() {
                CAP_RULES if !caps.rules => {
                    caps.rules = true;
                    accepted.push(r.clone());
                }
                CAP_FONT_HINTS if !caps.font_hints => {
                    caps.font_hints = true;
                    accepted.push(r.clone());
                }
                CAP_DISPLAY_LIST if !caps.display_list => {
                    caps.display_list = true;
                    accepted.push(r.clone());
                }
                CAP_DEVICE_COLOR if !caps.device_color && requested.iter().any(|c| c == CAP_DISPLAY_LIST) => {
                    caps.device_color = true;
                    accepted.push(r.clone());
                }
                CAP_IMAGES if !caps.images && requested.iter().any(|c| c == CAP_DISPLAY_LIST) => {
                    caps.images = true;
                    accepted.push(r.clone());
                }
                CAP_DELTA if !caps.delta && with_display_list && !windowing => {
                    caps.delta = true;
                    accepted.push(r.clone());
                }
                CAP_WINDOW if !caps.window && with_display_list => {
                    caps.window = true;
                    accepted.push(r.clone());
                }
                CAP_V2_ONLY if !caps.v2_only && requested.iter().any(|c| c == CAP_DISPLAY_LIST) => {
                    caps.v2_only = true;
                    accepted.push(r.clone());
                }
                _ => {}
            }
        }
        (caps, accepted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontHint {
    pub family: std::rc::Rc<str>,
    pub weight: &'static str,
    pub style: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum V1Item {
    Text {
        text: String,
        x_pt: f64,
        baseline_y_pt: f64,
        font_size_pt: f64,
        source: SourceRange,
        font: Option<FontHint>,
    },
    Rule {
        x_pt: f64,
        y_pt: f64,
        width_pt: f64,
        height_pt: f64,
        source: SourceRange,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct V1Page {
    pub number: u32,
    pub width_pt: f64,
    pub height_pt: f64,
    pub items: Vec<V1Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct V1Payload {
    pub project_id: String,
    pub revision: u64,
    pub status: &'static str,
    pub pages: Vec<V1Page>,
    pub diagnostics: Vec<display::Diagnostic>,
    pub accepted: Option<Vec<String>>,
}

/// Height of the U+2500 glyph box relative to the font size, as the current
/// compiler and the visual harness define the legacy rule convention.
pub const LEGACY_RULE_HEIGHT_EM: f64 = 0.0857;
pub const LEGACY_RULE_ADVANCE_EM: f64 = 0.5;

fn hint_for(font: &display::FontResource) -> FontHint {
    let ps = font.postscript_name.as_str();
    let lower = ps.to_ascii_lowercase();
    let family = if ps.starts_with("LatinModernMath") {
        "Latin Modern Math"
    } else if ps.starts_with("LM") {
        "Latin Modern Roman"
    } else if lower.starts_with("times") {
        "Times"
    } else if lower.starts_with("symbol") {
        "Symbol"
    } else {
        ps
    };
    FontHint {
        family: std::rc::Rc::from(family),
        weight: if lower.contains("bold") { "bold" } else { "normal" },
        style: if lower.contains("italic") || lower.contains("oblique") { "italic" } else { "normal" },
    }
}

fn union(sources: &[SourceRange]) -> Option<SourceRange> {
    union_of(sources.iter())
}

/// The smallest range covering every source in the first source's document.
fn union_of<'a>(mut sources: impl Iterator<Item = &'a SourceRange>) -> Option<SourceRange> {
    let first = sources.next()?;
    let mut out = first.clone();
    for s in sources.filter(|s| s.path == first.path) {
        out.start_byte = out.start_byte.min(s.start_byte);
        out.end_byte = out.end_byte.max(s.end_byte);
    }
    Some(out)
}

/// Builds the v1 payload. `accepted` is `None` when the request carried no
/// `layout_capabilities` field (the field is then omitted in the reply).
pub fn fallback(v2: &DisplayList, caps: Capabilities, accepted: Option<Vec<String>>) -> V1Payload {
    let mut pages = Vec::with_capacity(v2.pages.len());
    // One hint per font resource, shared by every run that uses it.
    let hints: Vec<Option<FontHint>> = v2.fonts.iter().map(|f| caps.font_hints.then(|| hint_for(f))).collect();
    for page in &v2.pages {
        // A v1 payload is the product preview of a whole document; there is no
        // v1 shape for "this page was not built", so an elided page is skipped
        // rather than sent as a blank one — a v1 consumer must never be handed a
        // page-shaped object that says the page is empty when it was simply not
        // built. A windowed list reaches here only because the consumer asked for
        // a window, was told so in the echo, and can read the sibling's `window`
        // object for the coverage; the pages that are here carry their real
        // `number`, so the subset is unambiguous rather than a shorter document.
        // A consumer that needs the v1 payload to stand for the whole document
        // does not negotiate `display-list-v2-window`.
        let Some(page_items) = page.items() else { continue };
        let mut items = Vec::with_capacity(page_items.len());
        for item in page_items {
            match item {
                display::Item::GlyphRun(run) => {
                    let hint = v2.fonts.iter().position(|f| f.font_id == run.font_id).and_then(|i| hints[i].as_ref());
                    let size = run.font_size.to_bp();
                    match run.role {
                        display::RunRole::Text => {
                            let (Some(source), Some(first)) = (union_of(run.clusters.iter().flat_map(|c| c.provenance.sources())), run.glyphs.first()) else { continue };
                            items.push(V1Item::Text {
                                text: run.text.clone(),
                                x_pt: first.origin_x.to_bp(),
                                baseline_y_pt: first.baseline_y.to_bp(),
                                font_size_pt: size,
                                source,
                                font: hint.cloned(),
                            });
                        }
                        display::RunRole::Math => {
                            for g in &run.glyphs {
                                let Some(c) = run.clusters.get(g.cluster as usize) else { continue };
                                let Some(source) = union(c.provenance.sources()) else { continue };
                                items.push(V1Item::Text {
                                    text: run.text[c.text_start_byte..c.text_end_byte].to_string(),
                                    x_pt: g.origin_x.to_bp(),
                                    baseline_y_pt: g.baseline_y.to_bp(),
                                    font_size_pt: size,
                                    source,
                                    font: hint.cloned(),
                                });
                            }
                        }
                    }
                }
                // Runtime-v1 has no vector items: TikZ paths exist only in the
                // display list v2 (the typesetter warns when it emits them).
                display::Item::Path(_) => {}
                // runtime-v1 has no image item; the v2 image proposal carries them.
                display::Item::Image(_) => {}
                display::Item::Rule(rule) => {
                    let Some(source) = union(rule.provenance.sources()) else { continue };
                    let (x, top, w, h) = (rule.x.to_bp(), rule.top.to_bp(), rule.width.to_bp(), rule.height.to_bp());
                    if caps.rules {
                        items.push(V1Item::Rule {
                            x_pt: x,
                            y_pt: top,
                            width_pt: w,
                            height_pt: h,
                            source,
                        });
                    } else {
                        let size = h / LEGACY_RULE_HEIGHT_EM;
                        let n = ((w / (LEGACY_RULE_ADVANCE_EM * size)).round() as usize).max(1);
                        items.push(V1Item::Text {
                            text: "\u{2500}".repeat(n),
                            x_pt: x,
                            baseline_y_pt: top + h,
                            font_size_pt: size,
                            source,
                            font: None,
                        });
                    }
                }
            }
        }
        pages.push(V1Page {
            number: page.number,
            width_pt: page.width.to_bp(),
            height_pt: page.height.to_bp(),
            items,
        });
    }
    // `display-list-v2-window` §4.1: the window does not change `status`. The
    // `pages` built above are a windowed list's resident ones only, so "did the
    // compile produce anything paintable" is asked of the whole-document harvest
    // assembly made over every block, not of the window.
    let has_content = pages.iter().any(|p| !p.items.is_empty())
        || (v2.window.is_some() && v2.document_features.is_some_and(|d| d.any_items));
    let has_error = v2.diagnostics.iter().any(|d| d.severity == Severity::Error);
    let status = if v2.diagnostics.is_empty() {
        "ok"
    } else if has_content || !has_error {
        "recovered"
    } else {
        "failed"
    };
    V1Payload {
        project_id: v2.project_id.clone(),
        revision: v2.revision,
        status,
        pages,
        diagnostics: v2.diagnostics.clone(),
        accepted,
    }
}

fn source_json(s: &SourceRange) -> Value {
    let mut o = Value::obj();
    o.set("path", json::str_(s.path.to_string()));
    o.set("start_byte", json::num(s.start_byte as f64));
    o.set("end_byte", json::num(s.end_byte as f64));
    o
}

/// Rounds to 1/1000 pt so the JSON is stable across platforms' float
/// formatting while keeping sub-pixel positions.
fn pt(v: f64) -> Value {
    json::num((v * 1000.0).round() / 1000.0)
}

pub fn diagnostic_json(d: &display::Diagnostic) -> Value {
    let mut v = Value::obj();
    v.set(
        "severity",
        json::str_(match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }),
    );
    v.set("message", json::str_(d.message.clone()));
    v.set("source", d.sources.first().map(source_json).unwrap_or(Value::Null));
    v.set("recovery", d.recovery.clone().map(json::str_).unwrap_or(Value::Null));
    v.set("code", json::str_(d.code.clone()));
    v
}

impl V1Payload {
    pub fn to_json(&self) -> Value {
        let mut p = Value::obj();
        p.set("project_id", json::str_(self.project_id.clone()));
        p.set("revision", json::num(self.revision as f64));
        p.set("status", json::str_(self.status));
        p.set(
            "pages",
            Value::Arr(
                self.pages
                    .iter()
                    .map(|pg| {
                        let mut o = Value::obj();
                        o.set("number", json::num(f64::from(pg.number)));
                        o.set("width_pt", pt(pg.width_pt));
                        o.set("height_pt", pt(pg.height_pt));
                        o.set(
                            "items",
                            Value::Arr(
                                pg.items
                                    .iter()
                                    .map(|it| {
                                        let mut o = Value::obj();
                                        match it {
                                            V1Item::Text {
                                                text,
                                                x_pt,
                                                baseline_y_pt,
                                                font_size_pt,
                                                source,
                                                font,
                                            } => {
                                                o.set("kind", json::str_("text"));
                                                o.set("text", json::str_(text.clone()));
                                                o.set("x_pt", pt(*x_pt));
                                                o.set("baseline_y_pt", pt(*baseline_y_pt));
                                                o.set("font_size_pt", pt(*font_size_pt));
                                                o.set("source", source_json(source));
                                                if let Some(f) = font {
                                                    let mut fo = Value::obj();
                                                    fo.set("family", json::str_(f.family.to_string()));
                                                    fo.set("weight", json::str_(f.weight));
                                                    fo.set("style", json::str_(f.style));
                                                    o.set("font", fo);
                                                }
                                            }
                                            V1Item::Rule {
                                                x_pt,
                                                y_pt,
                                                width_pt,
                                                height_pt,
                                                source,
                                            } => {
                                                o.set("kind", json::str_("rule"));
                                                o.set("x_pt", pt(*x_pt));
                                                o.set("y_pt", pt(*y_pt));
                                                o.set("width_pt", pt(*width_pt));
                                                o.set("height_pt", pt(*height_pt));
                                                o.set("source", source_json(source));
                                            }
                                        }
                                        o
                                    })
                                    .collect(),
                            ),
                        );
                        o
                    })
                    .collect(),
            ),
        );
        p.set("diagnostics", Value::Arr(self.diagnostics.iter().map(diagnostic_json).collect()));
        p.set("pdf_path", Value::Null);
        if let Some(acc) = &self.accepted {
            p.set("layout_capabilities", Value::Arr(acc.iter().cloned().map(json::str_).collect()));
        }
        p
    }
}


// ---------------------------------------------------------------------------
// Direct JSON Lines writer: the same bytes `json::write(&self.to_json())`
// produces (BTreeMap key order, the compiler's number and string rules)
// without building a `Value` tree, which was a third of a large request's
// time. `writer_matches_value_tree` pins the equivalence.

use std::fmt::Write as _;

fn js(out: &mut String, s: &str) {
    out.push('"');
    if s.bytes().all(|b| b >= 0x20 && b != b'"' && b != b'\\') {
        // Nothing to escape (the common case: paths, words, codes).
        out.push_str(s);
        out.push('"');
        return;
    }
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn jn(out: &mut String, n: f64) {
    if n.is_finite() && n == n.trunc() && n.abs() < 1e15 {
        ji(out, n as i64);
    } else if n.is_finite() {
        let _ = write!(out, "{}", n);
    } else {
        out.push_str("null");
    }
}

/// `i` in decimal, byte-identical to `{}` without going through `fmt`.
fn ji(out: &mut String, i: i64) {
    let mut buf = [0u8; 20];
    let mut n = i.unsigned_abs();
    let mut pos = buf.len();
    loop {
        pos -= 1;
        buf[pos] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    if i < 0 {
        out.push('-');
    }
    out.push_str(std::str::from_utf8(&buf[pos..]).unwrap());
}

/// A point value rounded to three decimals, written as the shortest
/// round-trip decimal — byte-identical to `jn(out, (v * 1000).round() /
/// 1000)`: for |v| < 10^11 the rounded value has at most 15 significant
/// digits, so its shortest representation is the milli-unit integer with
/// trailing zeros trimmed, and `{}` prints exactly that.
fn jpt(out: &mut String, v: f64) {
    let m = (v * 1000.0).round();
    if !m.is_finite() || m.abs() >= 1e11 {
        jn(out, m / 1000.0);
        return;
    }
    let m = m as i64;
    if m == 0 {
        out.push('0');
        return;
    }
    if m < 0 {
        out.push('-');
    }
    let a = m.unsigned_abs();
    ji(out, (a / 1000) as i64);
    let frac = a % 1000;
    if frac != 0 {
        out.push('.');
        let digits = [b'0' + (frac / 100) as u8, b'0' + (frac / 10 % 10) as u8, b'0' + (frac % 10) as u8];
        let keep = if digits[2] != b'0' {
            3
        } else if digits[1] != b'0' {
            2
        } else {
            1
        };
        out.push_str(std::str::from_utf8(&digits[..keep]).unwrap());
    }
}

fn jsource(out: &mut String, s: &SourceRange) {
    out.push_str("{\"end_byte\":");
    jn(out, s.end_byte as f64);
    out.push_str(",\"path\":");
    js(out, &s.path);
    out.push_str(",\"start_byte\":");
    jn(out, s.start_byte as f64);
    out.push('}');
}

fn jdiag(out: &mut String, d: &display::Diagnostic) {
    out.push_str("{\"code\":");
    js(out, &d.code);
    out.push_str(",\"message\":");
    js(out, &d.message);
    out.push_str(",\"recovery\":");
    match &d.recovery {
        Some(r) => js(out, r),
        None => out.push_str("null"),
    }
    out.push_str(",\"severity\":");
    js(
        out,
        match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        },
    );
    out.push_str(",\"source\":");
    match d.sources.first() {
        Some(s) => jsource(out, s),
        None => out.push_str("null"),
    }
    out.push('}');
}

impl V1Payload {
    /// The complete `compile_result` envelope line for request `id`
    /// (protocol version 1), byte-identical to
    /// `json::write(&result_envelope(id, self.to_json()))`.
    pub fn write_envelope(&self, id: &str) -> String {
        let mut out = String::with_capacity(64 + 96 * self.pages.iter().map(|p| p.items.len()).sum::<usize>());
        out.push_str("{\"id\":");
        js(&mut out, id);
        out.push_str(",\"payload\":");
        self.write_payload(&mut out);
        out.push_str(",\"protocol_version\":1,\"type\":\"compile_result\"}");
        out
    }

    fn write_payload(&self, out: &mut String) {
        out.push_str("{\"diagnostics\":[");
        for (i, d) in self.diagnostics.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            jdiag(out, d);
        }
        out.push(']');
        if let Some(acc) = &self.accepted {
            out.push_str(",\"layout_capabilities\":[");
            for (i, c) in acc.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                js(out, c);
            }
            out.push(']');
        }
        out.push_str(",\"pages\":[");
        for (pi, pg) in self.pages.iter().enumerate() {
            if pi > 0 {
                out.push(',');
            }
            out.push_str("{\"height_pt\":");
            jpt(out, pg.height_pt);
            out.push_str(",\"items\":[");
            for (ii, it) in pg.items.iter().enumerate() {
                if ii > 0 {
                    out.push(',');
                }
                match it {
                    V1Item::Text {
                        text,
                        x_pt,
                        baseline_y_pt,
                        font_size_pt,
                        source,
                        font,
                    } => {
                        out.push_str("{\"baseline_y_pt\":");
                        jpt(out, *baseline_y_pt);
                        if let Some(f) = font {
                            out.push_str(",\"font\":{\"family\":");
                            js(out, &f.family);
                            out.push_str(",\"style\":");
                            js(out, f.style);
                            out.push_str(",\"weight\":");
                            js(out, f.weight);
                            out.push('}');
                        }
                        out.push_str(",\"font_size_pt\":");
                        jpt(out, *font_size_pt);
                        out.push_str(",\"kind\":\"text\",\"source\":");
                        jsource(out, source);
                        out.push_str(",\"text\":");
                        js(out, text);
                        out.push_str(",\"x_pt\":");
                        jpt(out, *x_pt);
                        out.push('}');
                    }
                    V1Item::Rule {
                        x_pt,
                        y_pt,
                        width_pt,
                        height_pt,
                        source,
                    } => {
                        out.push_str("{\"height_pt\":");
                        jpt(out, *height_pt);
                        out.push_str(",\"kind\":\"rule\",\"source\":");
                        jsource(out, source);
                        out.push_str(",\"width_pt\":");
                        jpt(out, *width_pt);
                        out.push_str(",\"x_pt\":");
                        jpt(out, *x_pt);
                        out.push_str(",\"y_pt\":");
                        jpt(out, *y_pt);
                        out.push('}');
                    }
                }
            }
            out.push_str("],\"number\":");
            jn(out, f64::from(pg.number));
            out.push_str(",\"width_pt\":");
            jpt(out, pg.width_pt);
            out.push('}');
        }
        out.push_str("],\"pdf_path\":null,\"project_id\":");
        js(out, &self.project_id);
        out.push_str(",\"revision\":");
        jn(out, self.revision as f64);
        out.push_str(",\"status\":");
        js(out, self.status);
        out.push('}');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negotiation_accepts_only_known_requested_capabilities() {
        let (c, acc) = Capabilities::negotiate(&[]);
        assert_eq!(c, Capabilities::default());
        assert!(acc.is_empty());
        let (c, acc) = Capabilities::negotiate(&["font-hints-v1".into(), "rules-v2".into(), "rules-v1".into()]);
        assert!(c.rules && c.font_hints && !c.display_list);
        assert_eq!(acc, vec!["font-hints-v1".to_string(), "rules-v1".to_string()]);
        let (c, acc) = Capabilities::negotiate(&["display-list-v2".into(), "display-list-v2".into()]);
        assert!(c.display_list && !c.rules);
        assert_eq!(acc, vec!["display-list-v2".to_string()]);
        let (c, acc) = Capabilities::negotiate(&["unknown".into()]);
        assert_eq!(c, Capabilities::default());
        assert!(acc.is_empty());
    }

    #[test]
    fn writer_matches_value_tree() {
        let src = |path: &str, a: usize, b: usize| SourceRange {
            path: std::rc::Rc::from(path),
            start_byte: a,
            end_byte: b,
        };
        let payload = V1Payload {
            project_id: "p\"q".into(),
            revision: 7,
            status: "recovered",
            pages: vec![V1Page {
                number: 1,
                width_pt: 612.0,
                height_pt: 792.0,
                items: vec![
                    V1Item::Text {
                        text: "wörld\t\"x\"".into(),
                        x_pt: 72.0004,
                        baseline_y_pt: 83.955,
                        font_size_pt: 11.955,
                        source: src("a/b.tex", 3, 10),
                        font: Some(FontHint {
                            family: "Latin Modern Roman".into(),
                            weight: "bold",
                            style: "italic",
                        }),
                    },
                    V1Item::Text {
                        text: "x".into(),
                        x_pt: 1.0,
                        baseline_y_pt: 2.0,
                        font_size_pt: 3.0,
                        source: src("main.tex", 0, 1),
                        font: None,
                    },
                    V1Item::Rule {
                        x_pt: 302.386,
                        y_pt: 101.4675,
                        width_pt: 43.351,
                        height_pt: 0.3985,
                        source: src("main.tex", 37, 78),
                    },
                ],
            }],
            diagnostics: vec![
                display::Diagnostic::warning("w", "m\n", vec![src("main.tex", 1, 2)]),
                display::Diagnostic::error("e", "boom", Vec::new()),
            ],
            accepted: Some(vec!["rules-v1".into(), "font-hints-v1".into()]),
        };
        let mut env = Value::obj();
        env.set("protocol_version", json::num(1.0));
        env.set("id", json::str_("r-1"));
        env.set("type", json::str_("compile_result"));
        env.set("payload", payload.to_json());
        assert_eq!(payload.write_envelope("r-1"), json::write(&env));
        let mut none = payload.clone();
        none.accepted = None;
        none.diagnostics.clear();
        env.set("payload", none.to_json());
        assert_eq!(none.write_envelope("r-1"), json::write(&env));
    }

    /// `jpt`/`js` fast paths print exactly what `fmt` printed before.
    #[test]
    fn scalar_fast_paths_match_fmt() {
        let reference = |v: f64| {
            let n = (v * 1000.0).round() / 1000.0;
            if n == n.trunc() && n.abs() < 1e15 {
                format!("{}", n as i64)
            } else {
                format!("{}", n)
            }
        };
        let mut values: Vec<f64> = vec![
            0.0,
            -0.0,
            -0.0001,
            0.0004,
            0.0005,
            0.0015,
            0.5,
            -0.5,
            1.0,
            -1.0,
            12.3,
            612.0,
            791.999,
            1e10,
            -99999.001,
            84362432.0 / 1048576.0 * 0.75,
        ];
        let mut x: u64 = 0x9e3779b97f4a7c15;
        for _ in 0..20000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let mag = (x % 100_000_000) as f64 / 1000.0 - 50_000.0;
            values.push(mag);
            values.push(mag / 7.0);
            values.push(mag / 1024.0);
        }
        for v in values {
            let mut out = String::new();
            jpt(&mut out, v);
            assert_eq!(out, reference(v), "{v}");
        }
        for s in ["", "main.tex", "a\"b", "back\\slash", "tab\there", "\u{1}", "ünïcode—ok"] {
            let mut out = String::new();
            js(&mut out, s);
            assert_eq!(out, json::write(&Value::Str(s.to_string())), "{s}");
        }
    }
}
