//! runtime-v1 `compile_result` emission: one text item per word with exact
//! source spans, so the preview and `flashtex-pdf` consume identical
//! positions. No new item kinds are introduced.
//!
//! Coordinates: runtime-v1 uses PDF points (bp), origin top-left. Layout done
//! in TeX points is converted with `scale = 72/72.27` ([`crate::BP_PER_TEX_PT`]);
//! pass `1.0` when the layout already ran in bp.

use std::ops::Range;

use crate::linebreak::PositionedRun;
use crate::pages::{Page, Pages};

/// A word as consumers see it: source-contiguous runs merged (a kern between
/// fragments does not split a word) and a discretionary hyphen appended.
#[derive(Debug, Clone, PartialEq)]
pub struct WordItem {
    pub text: String,
    pub x: f64,
    pub baseline_y: f64,
    pub width: f64,
    pub size: f64,
    /// Source span of the word's glyph runs (the hyphen adds its marker range,
    /// which may be empty).
    pub source: Range<usize>,
}

/// Groups a page's runs into words. `text` is the source the runs' spans index.
pub fn words(page: &Page, text: &str) -> Vec<WordItem> {
    let mut out: Vec<WordItem> = Vec::new();
    let mut last: Option<(f64, usize)> = None; // (baseline, source end)
    for r in &page.runs {
        let same_line = last.is_some_and(|(b, _)| b == r.baseline_y);
        // Fragments of one word are separated by nothing, a kern, or an
        // unused `\-` marker — never by whitespace.
        let contiguous = last.is_some_and(|(_, e)| {
            e <= r.source.start && !text[e..r.source.start].chars().any(char::is_whitespace)
        });
        let piece = if r.is_hyphen {
            "-".to_string()
        } else {
            text[r.source.clone()].to_string()
        };
        if same_line && (contiguous || r.is_hyphen) {
            let w = out.last_mut().unwrap();
            w.text.push_str(&piece);
            w.width = r.x + r.width - w.x;
            w.source.end = r.source.end.max(w.source.end);
        } else {
            out.push(WordItem {
                text: piece,
                x: r.x,
                baseline_y: r.baseline_y,
                width: r.width,
                size: r.size,
                source: r.source.clone(),
            });
        }
        last = Some((r.baseline_y, r.source.end));
    }
    out
}

fn json_string(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn num(v: f64) -> String {
    // Fixed 4 decimals keep the JSON deterministic and well inside 0.001 bp.
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Serialises `pages` as a runtime-v1 `compile_result` envelope.
pub fn compile_result_json(
    pages: &Pages,
    text: &str,
    path: &str,
    project_id: &str,
    revision: u64,
    scale: f64,
) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "{{\"protocol_version\":1,\"id\":{},\"type\":\"compile_result\",\"payload\":{{\"project_id\":{},\"revision\":{revision},\"status\":\"ok\",\"pdf_path\":null,\"diagnostics\":[],\"pages\":[",
        json_string(&format!("{project_id}-{revision}")),
        json_string(project_id)
    ));
    for (pi, page) in pages.pages.iter().enumerate() {
        if pi > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "{{\"number\":{},\"width_pt\":{},\"height_pt\":{},\"items\":[",
            page.number,
            num(page.width * scale),
            num(page.height * scale)
        ));
        for (wi, w) in words(page, text).iter().enumerate() {
            if wi > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "{{\"kind\":\"text\",\"text\":{},\"x_pt\":{},\"baseline_y_pt\":{},\"font_size_pt\":{},\"source\":{{\"path\":{},\"start_byte\":{},\"end_byte\":{}}}}}",
                json_string(&w.text),
                num(w.x * scale),
                num(w.baseline_y * scale),
                num(w.size * scale),
                json_string(path),
                w.source.start,
                w.source.end
            ));
        }
        s.push_str("]}");
    }
    s.push_str("]}}");
    s
}

/// Runs of a page as (x, baseline) pairs in output units — handy for tests.
pub fn run_origins(page: &Page, scale: f64) -> Vec<(f64, f64)> {
    page.runs
        .iter()
        .map(|r: &PositionedRun| (r.x * scale, r.baseline_y * scale))
        .collect()
}
