//! PROPOSAL `display-list-v2-compact` — producer side of
//! `protocol/proposals/display-list-v2-compact.md` (r1). Opt-in next to
//! `display-list-v2`: when accepted, every glyph run on the sibling line
//! (full `display_list` and the `changed_pages` of a `display_list_delta`)
//! is written in the compact cluster encoding — a run-level `sources` span,
//! per-cluster byte offsets as deltas from the previous cluster, and no
//! `carets`/`hit_rects` objects: both are derived from the cluster's glyphs
//! and the run's `hit_top`/`hit_height`/`end_caret`, with per-cluster
//! overrides wherever the derivation would not reproduce the model exactly.
//! The payload announces it as `"cluster_encoding":"compact-1"`.
//!
//! Nothing here changes the bytes of a line for a consumer that did not
//! request the capability: `display::write_page` only calls into this module
//! under `Wire::compact`.
//!
//! Correctness is by construction, not by invariant: the writer runs the
//! decoder's derivation rules ([`predict`]) on the model it is about to
//! write and emits an override for every field whose derived value differs
//! from the model, so `read` ∘ `write` is the identity on any `GlyphRun`
//! (`tests/display_list_compact.rs` proves it on the real-world corpus). The
//! derivation rules themselves are the FT-070 invariants of PR #232
//! (`carets.first` == `hit_rect` + `text_start_byte`, the end caret only on
//! a run's last cluster with the last cluster's `top`/`height`), measured
//! there over 1 991 552 corpus clusters with zero exceptions; on this corpus
//! the overrides are therefore confined to the cases #232 also names (the
//! TikZ end-caret `x`, math runs with per-glyph vertical extents).

use flashtex_compiler::json::{self, Value};

use crate::display::{Caret, Carets, Cluster, DisplayList, GlyphRun, Item, Page, Provenance, Rect, SourceRange, Tick};

pub const CAP: &str = "display-list-v2-compact";
/// `payload.cluster_encoding` of a compact line.
pub const ENCODING: &str = "compact-1";
/// The payload key, exactly as the writer places it (sorted: after
/// `changed_pages`, before `color_space`).
pub const ENCODING_KEY_BYTES: &str = "\"cluster_encoding\":\"compact-1\",";

// ---------------------------------------------------------------- run-level model predicates

/// The run's source path: that of the first range of the first cluster that
/// has one. `None` when no cluster carries a source range.
pub fn run_path(r: &GlyphRun) -> Option<&SourceRange> {
    r.clusters.iter().find_map(|c| c.provenance.sources().first())
}

/// Whether cluster `c` is encoded implicitly against the run chain: exactly
/// one range, on the run path.
pub fn implicit(c: &Cluster, path: &str) -> bool {
    match c.provenance.sources() {
        [only] => &*only.path == path,
        _ => false,
    }
}

/// The run-level `sources` span `(path, min start, max end)` over the
/// implicit clusters; `None` when there is none.
pub fn run_span(r: &GlyphRun) -> Option<(std::rc::Rc<str>, usize, usize)> {
    let path = run_path(r)?.path.clone();
    let mut span: Option<(usize, usize)> = None;
    for c in &r.clusters {
        if implicit(c, &path) {
            let s = &c.provenance.sources()[0];
            span = Some(match span {
                None => (s.start_byte, s.end_byte),
                Some((a, b)) => (a.min(s.start_byte), b.max(s.end_byte)),
            });
        }
    }
    span.map(|(a, b)| (path, a, b))
}

/// The run's default hit-rect vertical extent: the most common
/// `(top, height)` pair over its clusters (ties: the earliest).
pub fn hit_default(r: &GlyphRun) -> Option<(Tick, Tick)> {
    let mut counts: Vec<((Tick, Tick), usize)> = Vec::new();
    for c in &r.clusters {
        let k = (c.hit_rect.top, c.hit_rect.height);
        match counts.iter_mut().find(|(kk, _)| *kk == k) {
            Some((_, n)) => *n += 1,
            None => counts.push((k, 1)),
        }
    }
    let mut best: Option<((Tick, Tick), usize)> = None;
    for (k, n) in counts {
        if best.map_or(true, |(_, m)| n > m) {
            best = Some((k, n));
        }
    }
    best.map(|(k, _)| k)
}

/// The run's end caret `(text_byte, x)`: the second caret of its last
/// cluster, when there is one. Any other second caret is an override (`c`).
pub fn end_caret(r: &GlyphRun) -> Option<(usize, Tick)> {
    r.clusters.last().and_then(|c| c.carets.last).map(|k| (k.text_byte, k.x))
}

/// The UTF-8 length of the scalar starting at byte `at` of `text` (0 when
/// `at` is not inside the text or not on a scalar boundary).
pub fn scalar_len(text: &str, at: usize) -> usize {
    text.get(at..).and_then(|t| t.chars().next()).map_or(0, char::len_utf8)
}

/// The derived (default) values of cluster `i` of `r`: what a decoder
/// reconstructs when the cluster object carries no override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Predicted {
    pub hit_rect: Rect,
    pub carets: Carets,
}

/// The x/width part of the derived hit rect: the origin of the cluster's
/// first glyph and the sum of its glyphs' advances (0/0 without glyphs).
pub fn predict_x_width(r: &GlyphRun, i: usize) -> (Tick, Tick) {
    let mut x = None;
    let mut width = 0i64;
    for g in r.glyphs.iter().filter(|g| g.cluster as usize == i) {
        if x.is_none() {
            x = Some(g.origin_x);
        }
        width += g.advance_x.0;
    }
    (x.unwrap_or(Tick(0)), Tick(width))
}

pub fn predict(r: &GlyphRun, i: usize, hit_top: Tick, hit_height: Tick, end: Option<(usize, Tick)>) -> Predicted {
    let c = &r.clusters[i];
    let (x, width) = predict_x_width(r, i);
    let hit_rect = Rect { x, top: hit_top, width, height: hit_height };
    let first = Caret { text_byte: c.text_start_byte, x: hit_rect.x, top: hit_rect.top, height: hit_rect.height };
    let last = end.filter(|_| i + 1 == r.clusters.len()).map(|(text_byte, ex)| Caret { text_byte, x: ex, top: hit_rect.top, height: hit_rect.height });
    Predicted { hit_rect, carets: Carets { first, last } }
}

// ---------------------------------------------------------------- writer

fn sep(o: &mut String, first: &mut bool) {
    if !*first {
        o.push(',');
    }
    *first = false;
}

fn num(o: &mut String, n: i64) {
    json::write_number_into(n as f64, o);
}

fn write_sources(o: &mut String, sources: &[SourceRange]) {
    o.push('[');
    for (i, s) in sources.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        o.push_str("{\"end_byte\":");
        num(o, s.end_byte as i64);
        o.push_str(",\"path\":");
        json::write_string_into(&s.path, o);
        o.push_str(",\"start_byte\":");
        num(o, s.start_byte as i64);
        o.push('}');
    }
    o.push(']');
}

/// One compact glyph-run object, exactly as it sits in a page's `items`
/// (`paint` is written by the caller's paint writer so device colours stay
/// with `display.rs`).
pub fn write_glyph_run(o: &mut String, r: &GlyphRun, write_paint: &dyn Fn(&mut String)) {
    let span = run_span(r);
    let hit = hit_default(r);
    let end = end_caret(r);
    let (hit_top, hit_height) = hit.unwrap_or((Tick(0), Tick(0)));
    o.push_str("{\"clusters\":[");
    let mut prev_text_end = 0usize;
    let mut prev_src_end = span.as_ref().map_or(0, |(_, s, _)| *s);
    for (i, c) in r.clusters.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        o.push('{');
        let mut first = true;
        let p = predict(r, i, hit_top, hit_height, end);
        // c: explicit carets when the derived list is not the model's.
        // The derived carets use the FINAL hit rect (after h/hv), so
        // recompute the comparison against the actual rect.
        let actual_first = Caret { text_byte: c.text_start_byte, x: c.hit_rect.x, top: c.hit_rect.top, height: c.hit_rect.height };
        let derived_last = end.filter(|_| i + 1 == r.clusters.len()).map(|(tb, ex)| Caret { text_byte: tb, x: ex, top: c.hit_rect.top, height: c.hit_rect.height });
        let derived = Carets { first: actual_first, last: derived_last };
        if derived != c.carets {
            sep(o, &mut first);
            o.push_str("\"c\":[");
            for (k, caret) in c.carets.iter().enumerate() {
                if k > 0 {
                    o.push(',');
                }
                o.push('[');
                num(o, caret.text_byte as i64);
                o.push(',');
                num(o, caret.x.0);
                o.push(',');
                num(o, caret.top.0);
                o.push(',');
                num(o, caret.height.0);
                o.push(']');
            }
            o.push(']');
        }
        let text_len = c.text_end_byte.saturating_sub(c.text_start_byte);
        let explicit = !span.as_ref().is_some_and(|(path, _, _)| implicit(c, path));
        // e: source length when it is not the text length (implicit only).
        if !explicit {
            let s = &c.provenance.sources()[0];
            let src_len = s.end_byte as i64 - s.start_byte as i64;
            if src_len != text_len as i64 {
                sep(o, &mut first);
                o.push_str("\"e\":");
                num(o, src_len);
            }
        }
        // h / hv: the hit rect where the derivation differs.
        let vertical_differs = c.hit_rect.top != p.hit_rect.top || c.hit_rect.height != p.hit_rect.height;
        if c.hit_rect.x != p.hit_rect.x || c.hit_rect.width != p.hit_rect.width {
            sep(o, &mut first);
            o.push_str("\"h\":[");
            num(o, c.hit_rect.x.0);
            o.push(',');
            num(o, c.hit_rect.top.0);
            o.push(',');
            num(o, c.hit_rect.width.0);
            o.push(',');
            num(o, c.hit_rect.height.0);
            o.push(']');
        } else if vertical_differs {
            sep(o, &mut first);
            o.push_str("\"hv\":[");
            num(o, c.hit_rect.top.0);
            o.push(',');
            num(o, c.hit_rect.height.0);
            o.push(']');
        }
        // l: text length when it is not one scalar.
        if text_len != scalar_len(&r.text, c.text_start_byte) {
            sep(o, &mut first);
            o.push_str("\"l\":");
            num(o, text_len as i64);
        }
        // s: source start delta from the chain (implicit only).
        if !explicit {
            let s = &c.provenance.sources()[0];
            let delta = s.start_byte as i64 - prev_src_end as i64;
            if delta != 0 {
                sep(o, &mut first);
                o.push_str("\"s\":");
                num(o, delta);
            }
            prev_src_end = s.end_byte;
        } else {
            sep(o, &mut first);
            match &c.provenance {
                Provenance::Synthetic(reason) => {
                    o.push_str("\"synthetic_reason\":");
                    json::write_string_into(reason, o);
                }
                p => {
                    o.push_str("\"sources\":");
                    write_sources(o, p.sources());
                }
            }
        }
        // ts: text start when it is not the chain.
        if c.text_start_byte != prev_text_end {
            sep(o, &mut first);
            o.push_str("\"ts\":");
            num(o, c.text_start_byte as i64);
        }
        prev_text_end = c.text_end_byte;
        o.push('}');
    }
    o.push(']');
    if let Some((text_byte, x)) = end {
        o.push_str(",\"end_caret\":{\"text_byte\":");
        num(o, text_byte as i64);
        o.push_str(",\"x\":");
        num(o, x.0);
        o.push('}');
    }
    o.push_str(",\"font_id\":");
    json::write_string_into(&r.font_id, o);
    o.push_str(",\"font_size\":");
    num(o, r.font_size.0);
    // Glyphs as `[gid, origin_x, baseline_y, advance_x, advance_y, cluster]`.
    o.push_str(",\"glyphs\":[");
    for (j, g) in r.glyphs.iter().enumerate() {
        if j > 0 {
            o.push(',');
        }
        o.push('[');
        num(o, i64::from(g.gid));
        o.push(',');
        num(o, g.origin_x.0);
        o.push(',');
        num(o, g.baseline_y.0);
        o.push(',');
        num(o, g.advance_x.0);
        o.push(',');
        num(o, g.advance_y.0);
        o.push(',');
        num(o, i64::from(g.cluster));
        o.push(']');
    }
    o.push(']');
    if let Some((top, height)) = hit {
        o.push_str(",\"hit_height\":");
        num(o, height.0);
        o.push_str(",\"hit_top\":");
        num(o, top.0);
    }
    o.push_str(",\"kind\":\"glyph_run\",\"paint\":");
    write_paint(o);
    if let Some((path, start, end)) = &span {
        o.push_str(",\"sources\":[{\"end_byte\":");
        num(o, *end as i64);
        o.push_str(",\"path\":");
        json::write_string_into(path, o);
        o.push_str(",\"start_byte\":");
        num(o, *start as i64);
        o.push_str("}]");
    }
    o.push_str(",\"text\":");
    json::write_string_into(&r.text, o);
    o.push('}');
}

// ---------------------------------------------------------------- delta accounting

fn digits(n: usize) -> usize {
    if n == 0 {
        1
    } else {
        n.ilog10() as usize + 1
    }
}

/// `display-list-v2-delta` under the compact encoding: the byte-length change
/// of a glyph run's source fields when `base` is relocated to `new`
/// (`unchanged_after_relocation` has already established that `new` is
/// `base` relocated as a model). Only absolute numbers move: the run-level
/// span and the ranges of explicit clusters; the per-cluster deltas are
/// invariant as long as the run's implicit ranges all move together, which
/// this returns `None` for otherwise (the producer then sends the page as
/// changed). `relocated` says whether one range of a document moved.
pub fn source_width_delta(base: &GlyphRun, new: &GlyphRun, relocated: &dyn Fn(&SourceRange) -> Option<bool>) -> Option<isize> {
    let mut delta = 0isize;
    let (bs, ns) = (run_span(base), run_span(new));
    match (&bs, &ns) {
        (Some((_, s0, e0)), Some((_, s1, e1))) => {
            delta += digits(*s1) as isize - digits(*s0) as isize;
            delta += digits(*e1) as isize - digits(*e0) as isize;
        }
        (None, None) => {}
        _ => return None,
    }
    let path = bs.as_ref().map(|(p, _, _)| p.clone());
    let mut moved: Option<bool> = None;
    for (b, n) in base.clusters.iter().zip(&new.clusters) {
        let is_implicit = path.as_ref().is_some_and(|p| implicit(b, p));
        if is_implicit {
            if let Some(m) = relocated(&b.provenance.sources()[0]) {
                match moved {
                    None => moved = Some(m),
                    Some(prev) if prev != m => return None,
                    _ => {}
                }
            }
        } else {
            for (rb, rn) in b.provenance.sources().iter().zip(n.provenance.sources()) {
                delta += digits(rn.start_byte) as isize - digits(rb.start_byte) as isize;
                delta += digits(rn.end_byte) as isize - digits(rb.end_byte) as isize;
            }
        }
    }
    Some(delta)
}

// ---------------------------------------------------------------- reader (gate + tools)

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError(pub String);

fn err<T>(m: impl Into<String>) -> Result<T, ReadError> {
    Err(ReadError(m.into()))
}

fn int(v: &Value, what: &str) -> Result<i64, ReadError> {
    v.as_i64().ok_or_else(|| ReadError(format!("{what}: not an integer")))
}

fn uint(v: &Value, what: &str) -> Result<usize, ReadError> {
    usize::try_from(int(v, what)?).map_err(|_| ReadError(format!("{what}: negative")))
}

fn read_sources(v: &Value) -> Result<Vec<SourceRange>, ReadError> {
    let arr = v.as_arr().ok_or_else(|| ReadError("sources: not an array".into()))?;
    arr.iter()
        .map(|s| {
            Ok(SourceRange {
                path: std::rc::Rc::from(s.get("path").and_then(Value::as_str).ok_or_else(|| ReadError("sources.path".into()))?),
                start_byte: uint(s.get("start_byte").ok_or_else(|| ReadError("sources.start_byte".into()))?, "start_byte")?,
                end_byte: uint(s.get("end_byte").ok_or_else(|| ReadError("sources.end_byte".into()))?, "end_byte")?,
            })
        })
        .collect()
}

fn tick_list(v: &Value, n: usize, what: &str) -> Result<Vec<Tick>, ReadError> {
    let arr = v.as_arr().ok_or_else(|| ReadError(format!("{what}: not an array")))?;
    if arr.len() != n {
        return err(format!("{what}: {} entries, expected {n}", arr.len()));
    }
    arr.iter().map(|t| int(t, what).map(Tick)).collect()
}

/// Reads the clusters of a compact glyph run (a parsed `glyph_run` item
/// object whose `clusters` are compact). `glyphs` are the run's already-read
/// glyphs, `text` its text.
pub fn read_clusters(item: &Value, glyphs: &[crate::display::Glyph], text: &str) -> Result<Vec<Cluster>, ReadError> {
    let arr = item.get("clusters").and_then(Value::as_arr).ok_or_else(|| ReadError("clusters".into()))?;
    let hit_top = item.get("hit_top").map(|v| int(v, "hit_top").map(Tick)).transpose()?;
    let hit_height = item.get("hit_height").map(|v| int(v, "hit_height").map(Tick)).transpose()?;
    let end = item
        .get("end_caret")
        .map(|e| Ok::<_, ReadError>((uint(e.get("text_byte").ok_or_else(|| ReadError("end_caret.text_byte".into()))?, "end_caret.text_byte")?, Tick(int(e.get("x").ok_or_else(|| ReadError("end_caret.x".into()))?, "end_caret.x")?))))
        .transpose()?;
    let span = match item.get("sources") {
        None => None,
        Some(v) => {
            let s = read_sources(v)?;
            if s.len() != 1 {
                return err("run sources: exactly one span");
            }
            Some(s.into_iter().next().unwrap())
        }
    };
    let n = arr.len();
    let mut out = Vec::with_capacity(n);
    let mut prev_text_end = 0usize;
    let mut prev_src_end = span.as_ref().map_or(0, |s| s.start_byte);
    for (i, c) in arr.iter().enumerate() {
        if c.get("hit_rects").is_some() {
            return err("full cluster object inside a compact run");
        }
        let ts = match c.get("ts") {
            Some(v) => uint(v, "ts")?,
            None => prev_text_end,
        };
        let l = match c.get("l") {
            Some(v) => uint(v, "l")?,
            None => scalar_len(text, ts),
        };
        let te = ts + l;
        prev_text_end = te;
        let (px, pw) = {
            let mut x = None;
            let mut w = 0i64;
            for g in glyphs.iter().filter(|g| g.cluster as usize == i) {
                if x.is_none() {
                    x = Some(g.origin_x);
                }
                w += g.advance_x.0;
            }
            (x.unwrap_or(Tick(0)), Tick(w))
        };
        let hit_rect = match c.get("h") {
            Some(h) => {
                let t = tick_list(h, 4, "h")?;
                Rect { x: t[0], top: t[1], width: t[2], height: t[3] }
            }
            None => {
                let (top, height) = match c.get("hv") {
                    Some(hv) => {
                        let t = tick_list(hv, 2, "hv")?;
                        (t[0], t[1])
                    }
                    None => (hit_top.ok_or_else(|| ReadError("hit_top missing".into()))?, hit_height.ok_or_else(|| ReadError("hit_height missing".into()))?),
                };
                Rect { x: px, top, width: pw, height }
            }
        };
        let carets = match c.get("c") {
            Some(v) => {
                let list = v.as_arr().ok_or_else(|| ReadError("c: not an array".into()))?;
                let mut ks = Vec::with_capacity(list.len());
                for k in list {
                    let t = tick_list(k, 4, "c")?;
                    ks.push(Caret { text_byte: usize::try_from(t[0].0).map_err(|_| ReadError("c: text_byte".into()))?, x: t[1], top: t[2], height: t[3] });
                }
                if ks.is_empty() || ks.len() > 2 {
                    return err("c: one or two carets");
                }
                Carets { first: ks[0], last: ks.get(1).copied() }
            }
            None => Carets {
                first: Caret { text_byte: ts, x: hit_rect.x, top: hit_rect.top, height: hit_rect.height },
                last: end.filter(|_| i + 1 == n).map(|(tb, x)| Caret { text_byte: tb, x, top: hit_rect.top, height: hit_rect.height }),
            },
        };
        let provenance = if let Some(reason) = c.get("synthetic_reason") {
            Provenance::Synthetic(reason.as_str().ok_or_else(|| ReadError("synthetic_reason".into()))?.to_string())
        } else if let Some(v) = c.get("sources") {
            let s = read_sources(v)?;
            if s.len() == 1 {
                Provenance::Source(s.into_iter().next().unwrap())
            } else {
                Provenance::Sources(s)
            }
        } else {
            let span = span.as_ref().ok_or_else(|| ReadError("implicit cluster in a run without sources".into()))?;
            let start = prev_src_end as i64 + c.get("s").map(|v| int(v, "s")).transpose()?.unwrap_or(0);
            let len = c.get("e").map(|v| int(v, "e")).transpose()?.unwrap_or(l as i64);
            let end_byte = start + len;
            if start < 0 || end_byte < start {
                return err("implicit cluster: negative range");
            }
            prev_src_end = end_byte as usize;
            Provenance::Source(SourceRange { path: span.path.clone(), start_byte: start as usize, end_byte: end_byte as usize })
        };
        out.push(Cluster { text_start_byte: ts, text_end_byte: te, hit_rect, carets, provenance });
    }
    Ok(out)
}

/// The cluster-independent part of a glyph-run object, shared by the full
/// and the compact reader.
fn read_run_head(item: &Value) -> Result<(std::rc::Rc<str>, Tick, String, Vec<crate::display::Glyph>, crate::display::Paint), ReadError> {
    let font_id = std::rc::Rc::from(item.get("font_id").and_then(Value::as_str).ok_or_else(|| ReadError("font_id".into()))?);
    let font_size = Tick(int(item.get("font_size").ok_or_else(|| ReadError("font_size".into()))?, "font_size")?);
    let text = item.get("text").and_then(Value::as_str).ok_or_else(|| ReadError("text".into()))?.to_string();
    let glyphs = item
        .get("glyphs")
        .and_then(Value::as_arr)
        .ok_or_else(|| ReadError("glyphs".into()))?
        .iter()
        .map(|g| {
            // Compact: `[gid, origin_x, baseline_y, advance_x, advance_y, cluster]`; full: an object.
            let (gid, ox, by, ax, ay, cl) = if g.as_arr().is_some() {
                let t = tick_list(g, 6, "glyph")?;
                (t[0].0, t[1].0, t[2].0, t[3].0, t[4].0, t[5].0)
            } else {
                let f = |k: &str| int(g.get(k).ok_or_else(|| ReadError(format!("glyph.{k}")))?, k);
                (f("gid")?, f("origin_x")?, f("baseline_y")?, f("advance_x")?, f("advance_y")?, f("cluster")?)
            };
            Ok(crate::display::Glyph {
                gid: u16::try_from(gid).map_err(|_| ReadError("gid".into()))?,
                origin_x: Tick(ox),
                baseline_y: Tick(by),
                advance_x: Tick(ax),
                advance_y: Tick(ay),
                cluster: u32::try_from(cl).map_err(|_| ReadError("cluster".into()))?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let p = item.get("paint").ok_or_else(|| ReadError("paint".into()))?;
    let comp = |k: &str| match p.get(k) {
        Some(Value::Num(n)) => Ok(*n),
        _ => err(format!("paint.{k}")),
    };
    let paint = crate::display::Paint { r: comp("r")?, g: comp("g")?, b: comp("b")?, a: comp("a")?, device: None };
    Ok((font_id, font_size, text, glyphs, paint))
}

/// Reads the glyph runs of a full `display_list` line (compact or not) back
/// into the model: the page list, with every non-run item dropped
/// (`read_pages` compares runs; rules/paths/images are written by the
/// unchanged writer and are not this module's concern). Paint device colours
/// and the run role are not on the wire and come back as defaults.
pub fn read_runs(line: &str) -> Result<Vec<(u32, Vec<GlyphRun>)>, ReadError> {
    let v = json::parse(line).map_err(|e| ReadError(e.0))?;
    let payload = v.get("payload").ok_or_else(|| ReadError("payload".into()))?;
    let compact = match payload.get("cluster_encoding").and_then(Value::as_str) {
        None => false,
        Some(ENCODING) => true,
        Some(other) => return err(format!("unknown cluster_encoding {other}")),
    };
    let pages = payload.get("pages").and_then(Value::as_arr).ok_or_else(|| ReadError("pages".into()))?;
    let mut out = Vec::with_capacity(pages.len());
    for p in pages {
        let number = uint(p.get("number").ok_or_else(|| ReadError("number".into()))?, "number")? as u32;
        let mut runs = Vec::new();
        for item in p.get("items").and_then(Value::as_arr).ok_or_else(|| ReadError("items".into()))? {
            if item.get("kind").and_then(Value::as_str) != Some("glyph_run") {
                continue;
            }
            let (font_id, font_size, text, glyphs, paint) = read_run_head(item)?;
            let clusters = if compact { read_clusters(item, &glyphs, &text)? } else { read_full_clusters(item)? };
            runs.push(GlyphRun { font_id, font_size, text, glyphs, clusters, paint, role: crate::display::RunRole::Text });
        }
        out.push((number, runs));
    }
    Ok(out)
}

fn read_full_clusters(item: &Value) -> Result<Vec<Cluster>, ReadError> {
    let arr = item.get("clusters").and_then(Value::as_arr).ok_or_else(|| ReadError("clusters".into()))?;
    arr.iter()
        .map(|c| {
            let rects = c.get("hit_rects").and_then(Value::as_arr).ok_or_else(|| ReadError("hit_rects".into()))?;
            if rects.len() != 1 {
                return err("hit_rects: exactly one");
            }
            let r = &rects[0];
            let f = |o: &Value, k: &str| int(o.get(k).ok_or_else(|| ReadError(k.to_string()))?, k);
            let hit_rect = Rect { x: Tick(f(r, "x")?), top: Tick(f(r, "top")?), width: Tick(f(r, "width")?), height: Tick(f(r, "height")?) };
            let ks = c.get("carets").and_then(Value::as_arr).ok_or_else(|| ReadError("carets".into()))?;
            let caret = |k: &Value| Ok::<_, ReadError>(Caret { text_byte: uint(k.get("text_byte").ok_or_else(|| ReadError("text_byte".into()))?, "text_byte")?, x: Tick(f(k, "x")?), top: Tick(f(k, "top")?), height: Tick(f(k, "height")?) });
            if ks.is_empty() || ks.len() > 2 {
                return err("carets: one or two");
            }
            let carets = Carets { first: caret(&ks[0])?, last: ks.get(1).map(caret).transpose()? };
            let provenance = if let Some(reason) = c.get("synthetic_reason") {
                Provenance::Synthetic(reason.as_str().ok_or_else(|| ReadError("synthetic_reason".into()))?.to_string())
            } else {
                let s = read_sources(c.get("sources").ok_or_else(|| ReadError("sources".into()))?)?;
                if s.len() == 1 {
                    Provenance::Source(s.into_iter().next().unwrap())
                } else {
                    Provenance::Sources(s)
                }
            };
            Ok(Cluster {
                text_start_byte: uint(c.get("text_start_byte").ok_or_else(|| ReadError("text_start_byte".into()))?, "text_start_byte")?,
                text_end_byte: uint(c.get("text_end_byte").ok_or_else(|| ReadError("text_end_byte".into()))?, "text_end_byte")?,
                hit_rect,
                carets,
                provenance,
            })
        })
        .collect()
}

/// The glyph runs of `list` in wire order, with provenance normalised the
/// way a reader returns it (`Sources` of one range is `Source`), for
/// comparison with [`read_runs`].
pub fn model_runs(list: &DisplayList) -> Vec<(u32, Vec<GlyphRun>)> {
    list.pages
        .iter()
        .map(|p: &Page| {
            let runs = p
                .items
                .iter()
                .filter_map(|it| match it {
                    Item::GlyphRun(r) => Some(r),
                    _ => None,
                })
                .map(|r| {
                    let mut r = r.clone();
                    r.paint.device = None;
                    r.role = crate::display::RunRole::Text;
                    for c in &mut r.clusters {
                        if let Provenance::Sources(v) = &c.provenance {
                            if v.len() == 1 {
                                c.provenance = Provenance::Source(v[0].clone());
                            }
                        }
                    }
                    r
                })
                .collect();
            (p.number, runs)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_len_is_one_scalar() {
        assert_eq!(scalar_len("aé", 0), 1);
        assert_eq!(scalar_len("aé", 1), 2);
        assert_eq!(scalar_len("aé", 3), 0);
        assert_eq!(scalar_len("aé", 2), 0); // not a boundary
    }

    #[test]
    fn digits_counts_decimal_width() {
        assert_eq!(digits(0), 1);
        assert_eq!(digits(9), 1);
        assert_eq!(digits(10), 2);
        assert_eq!(digits(1000), 4);
    }
}
