//! Preview-side contract: a JSON export of exactly the numbers the engine
//! used, so the Swift preview draws glyph ids at positions computed from the
//! SAME advances instead of asking CoreText to shape the text again.
//!
//! `face_metrics.json` shape (all integers are font units unless noted):
//!
//! ```json
//! {
//!   "schema_version": 1,
//!   "font": { "font_id": "<sha256 hex>", "postscript_name": "...", "family": "...",
//!             "units_per_em": 1000, "source_path": "...", "face_index": 0,
//!             "ascender": 806, "descender": -194, "line_gap": 0,
//!             "cap_height": 683, "x_height": 431 },
//!   "size_pt": 12.0,
//!   "advances": { "27": 750, "28": 500 },          // per used ORIGINAL gid, unkerned
//!   "runs": [ { "text": "…", "clusters": [
//!       { "source_range": [0, 2], "text": "fi", "font": 0,
//!         "glyphs": [ { "gid": 125, "advance": 556, "x_offset": 0, "y_offset": 0,
//!                       "x_pt": 0.0, "y_pt": 0.0 } ] } ],
//!       "missing": [ { "char": "U+4E2D", "byte_offset": 5 } ],
//!       "width_pt": 26.664 } ]
//! }
//! ```
//!
//! `x_pt`/`y_pt` are pen-relative glyph origins in points at `size_pt`
//! (kerning and mark offsets included), so a renderer only needs the font
//! file, the gid and these two numbers.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::shape::Shaped;
use crate::{Face, FontSource};

fn json_str(out: &mut String, s: &str) {
    out.push('"');
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

/// One shaped run to export, with the text it came from.
pub struct ExportRun<'a> {
    pub text: &'a str,
    pub shaped: &'a Shaped,
}

/// Serialises the export described in the module docs. Deterministic for
/// fixed inputs (BTreeMap ordering, fixed key order, `{:.4}` points).
pub fn face_metrics_json(face: &dyn Face, size_pt: f64, runs: &[ExportRun<'_>]) -> String {
    let id = face.id();
    let vm = face.vertical_metrics();
    let (path, index) = match &id.source {
        FontSource::File { path, face_index } => (path.display().to_string(), *face_index),
        FontSource::Memory { face_index } => (String::new(), *face_index),
        FontSource::Core14 { afm_name } => (format!("core14:{afm_name}"), 0),
    };
    let mut advances: BTreeMap<u16, u16> = BTreeMap::new();
    for r in runs {
        for g in r.shaped.glyphs() {
            if let Ok(a) = face.advance(g.gid) {
                advances.insert(g.gid.0, a);
            }
        }
    }
    let mut o = String::new();
    o.push_str("{\n  \"schema_version\": 1,\n  \"font\": {\n    \"font_id\": ");
    json_str(&mut o, &id.content_hex());
    o.push_str(",\n    \"postscript_name\": ");
    json_str(&mut o, face.postscript_name());
    o.push_str(",\n    \"family\": ");
    json_str(&mut o, &id.family);
    let _ = write!(
        o,
        ",\n    \"units_per_em\": {},\n    \"source_path\": ",
        face.units_per_em()
    );
    json_str(&mut o, &path);
    let _ = write!(
        o,
        ",\n    \"face_index\": {index},\n    \"ascender\": {},\n    \"descender\": {},\n    \"line_gap\": {},\n    \"cap_height\": {},\n    \"x_height\": {}\n  }},\n  \"size_pt\": {size_pt},\n  \"advances\": {{",
        vm.ascender, vm.descender, vm.line_gap, vm.cap_height, vm.x_height
    );
    for (i, (gid, a)) in advances.iter().enumerate() {
        let _ = write!(o, "{}\"{gid}\": {a}", if i == 0 { " " } else { ", " });
    }
    o.push_str(" },\n  \"runs\": [");
    for (ri, r) in runs.iter().enumerate() {
        o.push_str(if ri == 0 {
            "\n    {\n      \"text\": "
        } else {
            ",\n    {\n      \"text\": "
        });
        json_str(&mut o, r.text);
        o.push_str(",\n      \"clusters\": [");
        let mut pen = 0.0f64;
        for (ci, c) in r.shaped.clusters.iter().enumerate() {
            let _ = write!(
                o,
                "{}\n        {{ \"source_range\": [{}, {}], \"text\": ",
                if ci == 0 { "" } else { "," },
                c.source_range.start,
                c.source_range.end
            );
            json_str(&mut o, &c.text);
            let _ = write!(o, ", \"font\": {}, \"glyphs\": [", c.font);
            for (gi, g) in c.glyphs.iter().enumerate() {
                let x_pt = pen + face.to_points(i64::from(g.x_offset), size_pt);
                let y_pt = face.to_points(i64::from(g.y_offset), size_pt);
                let _ = write!(
                    o,
                    "{}{{ \"gid\": {}, \"advance\": {}, \"x_offset\": {}, \"y_offset\": {}, \"x_pt\": {x_pt:.4}, \"y_pt\": {y_pt:.4} }}",
                    if gi == 0 { " " } else { ", " },
                    g.gid.0,
                    g.advance,
                    g.x_offset,
                    g.y_offset
                );
                pen += face.to_points(i64::from(g.advance), size_pt);
            }
            o.push_str(" ] }");
        }
        o.push_str("\n      ],\n      \"missing\": [");
        for (mi, m) in r.shaped.missing.iter().enumerate() {
            let _ = write!(
                o,
                "{}{{ \"char\": \"U+{:04X}\", \"byte_offset\": {} }}",
                if mi == 0 { " " } else { ", " },
                m.ch as u32,
                m.byte_offset
            );
        }
        let _ = write!(
            o,
            " ],\n      \"width_pt\": {:.4}\n    }}",
            r.shaped.width_pt(size_pt)
        );
    }
    o.push_str("\n  ]\n}\n");
    o
}
