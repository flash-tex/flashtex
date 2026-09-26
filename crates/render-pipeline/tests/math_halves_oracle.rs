//! The three renderer halves the `compiler`/`tex-expansion`/`math-layout`
//! re-pin brought in, against pdfTeX's own glyph origins: amsbsy `\pmb`,
//! `\smash`/`\smash[t]`/`\smash[b]`, and the nineteen `\ext@arrow`s
//! (amsmath's two, mathtools' seventeen).
//!
//! `fixtures/math-halves/generate.py` runs MacTeX's pdflatex once over the
//! three committed documents and writes `expected/<doc>.txt`; nothing here
//! runs TeX. For every formula (one per line, labelled `NNNN` in text before
//! it) each pdfTeX glyph needs an engine glyph of the same size within
//! [`TOL_BP`] in x and y of the same offset from the label's origin, painting
//! a character the declaration table gives that font slot, and the engine may
//! paint nothing pdfTeX did not.
//!
//! [`KNOWN`] lists the formulas that still differ, by document and name, so a
//! regression anywhere else fails the test and a fix has to remove its entry.
//!
//! The fonts and metrics are the repository's bundle only, never the ambient
//! `FLASHTEX_*` environment or a host TeX Live, exactly as
//! `declared_math_oracle` does it.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// The lane's tolerance: glyph identity and position within half a big point.
const TOL_BP: f64 = 0.5;
const SIZE_TOL_BP: f64 = 0.05;

/// `apps/mac/Fonts` and its `texmf` metrics, exactly what the app bundle and
/// CI provide, with no system font index: the same set on every machine.
fn bundled_fonts() -> flashtex_render_pipeline::fonts::FontSet {
    let fonts = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../apps/mac/Fonts"));
    let tfm = |sub: &str| fonts.join("texmf").join(sub);
    let tfm_dirs = ["fonts/tfm/public/lm", "fonts/tfm/jknappen/ec", "fonts/tfm/public/amsfonts/symbols", "fonts/tfm/public/amsfonts/euler", "fonts/tfm/public/cm"].map(tfm).to_vec();
    flashtex_render_pipeline::fonts::FontSet::with_dirs(vec![fonts.clone()], tfm_dirs).with_index_dirs(Vec::new())
}

/// (document, formula name or `*`): formulas known to differ, with the
/// reason. `*diagnostics` stands for the document's error diagnostics.
#[rustfmt::skip]
const KNOWN: &[(&str, &str, &str)] = &[
    // The same pre-existing `\mapstochar` difference `declared_math_oracle`
    // already lists for `\mapsto` and `\longmapsto`: Latin Modern's bar
    // starts 56/1000 em right of the origin where Computer Modern's starts
    // at it, so `mathtex::MAPSTOCHAR` is painted 0.56pt (0.558bp) further
    // right on purpose, to put the *ink* where pdfTeX's is. Every other
    // glyph of `\xmapsto` -- the `\relbar` the bar abuts, the leaders and
    // the arrowhead -- is within 0.003bp.
    ("xarrows", "xmapsto", "\\mapstochar: the bundled bar's ink is offset 0.056em from its origin (as for \\mapsto, declared_math_oracle)"),
    // Not a `\smash` difference: `\left(\frac{a}{b}\right)` in a script-size
    // formula picks a *text*-size `(` in pdfTeX (`var_delimiter` walks down
    // from `cur_size` through the text font, tex.web 706-707) and a
    // script-size one here. The smashed and unsmashed rows behave alike.
    ("smash", "paren", "\\left( in script style: var_delimiter's size walk picks the text-font variant in pdfTeX"),
];

/// (pdfTeX font, slot, em): glyphs the engine paints from a *different*
/// outline than pdfTeX's, moved off TeX's origin by a fixed amount so the
/// ink lands on pdfTeX's ink. The engine glyph must sit exactly that far
/// from pdfTeX's origin (within [`TOL_BP`]); the box and every other glyph
/// are checked as usual.
///
/// `\lhook`/`\rhook` (cmmi10 "2C/"2D) are painted from Latin Modern Math's
/// `uni21AA.lft`/`uni21A9.rt` with the hook bowl (x 0..220 and 287..507 per
/// mille) centred on cmmi10's hook ink (x 55..222):
/// 138.5 - 110 = +28.5 and 138.5 - 397 = -258.5 per mille
/// (`typeset::hook_paint_dx`). `\rhook`'s bowl is at the far end of a
/// 0.507em part, which is why no painted origin can be pdfTeX's there.
#[rustfmt::skip]
const PAINT_SHIFT: &[(&str, u32, f64)] = &[
    ("CMMI10", 44, 0.0285),
    ("CMMI10", 45, -0.2585),
];

fn paint_shift_bp(e: &ExpectedGlyph) -> f64 {
    PAINT_SHIFT
        .iter()
        .find(|(font, code, _)| *font == e.font && *code == e.code)
        .map_or(0.0, |(_, _, em)| em * e.size)
}

#[derive(Debug, Clone)]
struct ExpectedGlyph {
    font: String,
    code: u32,
    size: f64,
    dx: f64,
    dy: f64,
    /// The texts declarations give the slot; empty when none has one.
    text: Vec<String>,
}

#[derive(Debug, Clone)]
struct Formula {
    label: String,
    category: String,
    name: String,
    style: String,
    tex: String,
    glyphs: Vec<ExpectedGlyph>,
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/math-halves")
}

fn parse_expected(text: &str) -> Vec<Formula> {
    let mut out: Vec<Formula> = Vec::new();
    let mut anchor = (0.0, 0.0);
    for line in text.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("formula ") {
            let mut it = rest.splitn(9, ' ');
            let label = it.next().unwrap().to_string();
            let _page = it.next().unwrap();
            let category = it.next().unwrap().to_string();
            let name = it.next().unwrap().to_string();
            let style = it.next().unwrap().to_string();
            let _slot = it.next().unwrap();
            let ax: f64 = it.next().unwrap().parse().unwrap();
            let ay: f64 = it.next().unwrap().parse().unwrap();
            let tex = it.next().unwrap_or("").to_string();
            anchor = (ax, ay);
            out.push(Formula { label, category, name, style, tex, glyphs: Vec::new() });
        } else if let Some(rest) = line.strip_prefix("g ") {
            let f: Vec<&str> = rest.split(' ').collect();
            let text: Vec<String> = if f[5] == "-" {
                Vec::new()
            } else {
                f[5].split('|')
                    .map(|t| {
                        t.split("U+")
                            .filter(|s| !s.is_empty())
                            .map(|h| char::from_u32(u32::from_str_radix(h, 16).unwrap()).unwrap())
                            .collect()
                    })
                    .collect()
            };
            out.last_mut().unwrap().glyphs.push(ExpectedGlyph {
                font: f[0].to_string(),
                code: f[1].parse().unwrap(),
                size: f[2].parse().unwrap(),
                dx: f[3].parse::<f64>().unwrap() - anchor.0,
                dy: f[4].parse::<f64>().unwrap() - anchor.1,
                text,
            });
        }
    }
    out
}

/// Whether the character the engine paints is one the slot's declarations
/// spell (or the slot has no spelling at all).
fn identity_ok(engine: &str, declared: &[String]) -> bool {
    // Overprint marks (`\mapstochar` U+F8FE) are painted under or beside the
    // character they extend, whose text the engine's cluster carries.
    let mark = declared.iter().any(|t| t == "\u{0338}" || t == "\u{F8FE}");
    let phi_swap = |t: &str| match t {
        "\u{03C6}" => engine == "\u{03D5}",
        "\u{03D5}" => engine == "\u{03C6}",
        _ => false,
    };
    declared.is_empty() || mark || declared.iter().any(|t| t == engine || phi_swap(t))
}

#[derive(Debug, Clone)]
struct EngineGlyph {
    text: String,
    size: f64,
    x: f64,
    y: f64,
    line: usize,
    is_label: bool,
}

fn line_of(offsets: &[(usize, usize)], byte: usize) -> Option<usize> {
    offsets.iter().position(|&(s, e)| byte >= s && byte < e)
}

fn run_doc(doc: &str) -> (usize, Vec<String>, BTreeMap<String, Vec<String>>) {
    let tex = std::fs::read_to_string(fixture_dir().join(format!("{doc}.tex"))).unwrap();
    let expected = parse_expected(&std::fs::read_to_string(fixture_dir().join(format!("expected/{doc}.txt"))).unwrap());
    let mut offsets = Vec::new();
    let mut pos = 0;
    let mut label_lines = BTreeMap::new();
    for line in tex.split_inclusive('\n') {
        let range = (pos, pos + line.len());
        if line.len() > 5 && line.as_bytes()[4] == b' ' && line[..4].bytes().all(|b| b.is_ascii_digit()) {
            label_lines.insert(line[..4].to_string(), offsets.len());
        }
        offsets.push(range);
        pos += line.len();
    }
    let r = render_one_with(&tex, &bundled_fonts());
    let errors: Vec<String> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.severity == flashtex_render_pipeline::display::Severity::Error)
        .map(|d| d.message.clone())
        .collect();
    let mut glyphs: Vec<EngineGlyph> = Vec::new();
    for page in &r.v2.pages {
        for item in page.resident_items() {
            let Item::GlyphRun(run) = item else { continue };
            let is_label = run.role == RunRole::Text
                && run.text.len() == 4
                && run.text.bytes().all(|b| b.is_ascii_digit());
            for g in &run.glyphs {
                let c = &run.clusters[g.cluster as usize];
                let text = run.text[c.text_start_byte..c.text_end_byte].to_string();
                let line = c
                    .provenance
                    .sources()
                    .first()
                    .and_then(|s| line_of(&offsets, s.start_byte));
                let Some(line) = line else { continue };
                glyphs.push(EngineGlyph {
                    text,
                    size: run.font_size.to_bp(),
                    x: g.origin_x.to_bp(),
                    y: g.baseline_y.to_bp(),
                    line,
                    is_label,
                });
            }
        }
    }
    let mut failures: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut passed = Vec::new();
    for f in &expected {
        let Some(&line) = label_lines.get(&f.label) else {
            failures.entry(f.name.clone()).or_default().push(format!("{}: no source line", f.label));
            continue;
        };
        let mine: Vec<&EngineGlyph> = glyphs.iter().filter(|g| g.line == line).collect();
        let Some(anchor) = mine.iter().find(|g| g.is_label) else {
            failures.entry(f.name.clone()).or_default().push(format!("{} {} {}: label not painted", f.label, f.name, f.style));
            continue;
        };
        let (ax, ay) = (anchor.x, anchor.y);
        let mut used = vec![false; mine.len()];
        let mut problems = Vec::new();
        for e in &f.glyphs {
            // A cmex Type 1 glyph's origin sits at the top of its TFM box,
            // so only x is comparable (as in `declared_math_oracle`).
            let x_only = e.font.starts_with("CMEX");
            let want_dx = e.dx + paint_shift_bp(e);
            let hit = mine.iter().enumerate().find(|(i, g)| {
                !used[*i]
                    && !g.is_label
                    && (g.size - e.size).abs() <= SIZE_TOL_BP
                    && (g.x - ax - want_dx).abs() <= TOL_BP
                    && (x_only || (g.y - ay - e.dy).abs() <= TOL_BP)
            });
            match hit {
                Some((i, g)) => {
                    used[i] = true;
                    if !identity_ok(&g.text, &e.text) {
                        problems.push(format!(
                            "{} {} at ({:+.3},{:+.3}): engine paints {:?}, the slot is declared as {:?}",
                            e.font, e.code, e.dx, e.dy, g.text, e.text
                        ));
                    }
                }
                None => {
                    let near: Vec<String> = mine
                        .iter()
                        .filter(|g| !g.is_label && (g.x - ax - e.dx).abs() <= 3.0)
                        .map(|g| format!("{:?}@({:+.3},{:+.3},{:.2}pt)", g.text, g.x - ax, g.y - ay, g.size))
                        .collect();
                    problems.push(format!(
                        "{} {} {:?} at ({:+.3},{:+.3},{:.2}pt) unmatched; engine near: {}",
                        e.font, e.code, e.text, e.dx, e.dy, e.size, near.join(" ")
                    ));
                }
            }
        }
        for (i, g) in mine.iter().enumerate() {
            if !used[i] && !g.is_label {
                problems.push(format!("engine extra {:?} at ({:+.3},{:+.3},{:.2}pt)", g.text, g.x - ax, g.y - ay, g.size));
            }
        }
        if problems.is_empty() {
            passed.push(format!("{} {} {}", f.label, f.name, f.style));
        } else {
            failures
                .entry(f.name.clone())
                .or_default()
                .push(format!("{} {} {} `{}`:\n      {}", f.label, f.category, f.style, f.tex, problems.join("\n      ")));
        }
    }
    if !errors.is_empty() {
        failures.entry("*diagnostics".into()).or_default().extend(errors);
    }
    (expected.len(), passed, failures)
}

fn check(doc: &str) {
    if !lm_available() {
        return;
    }
    let (total, passed, failures) = run_doc(doc);
    let known: BTreeSet<&str> = KNOWN.iter().filter(|(d, _, _)| *d == doc).map(|(_, n, _)| *n).collect();
    let unexpected: Vec<&String> = failures.keys().filter(|n| !known.contains(n.as_str())).collect();
    let fixed: Vec<&&str> = known.iter().filter(|n| !failures.contains_key(**n)).collect();
    let mut report = String::new();
    for name in &unexpected {
        report.push_str(&format!("  {name}:\n    {}\n", failures[*name].join("\n    ")));
    }
    assert!(
        unexpected.is_empty(),
        "{doc}: {} of {total} formulas match pdfTeX; {} name(s) differ that KNOWN does not list:\n{report}",
        passed.len(),
        unexpected.len()
    );
    assert!(
        fixed.is_empty(),
        "{doc}: KNOWN lists {fixed:?}, which now match pdfTeX -- delete those entries"
    );
}

/// amsbsy `\pmb`: the nucleus overprinted at −0.8mu, at −0.4mu raised
/// `\pmbraise@` and unshifted, in the body's own width and depth and 0.5mu
/// more height, with the class `\binrel@` gives it.
#[test]
fn pmb_overprints_the_nucleus_where_pdflatex_does() {
    check("pmb");
}

/// `\smash`, `\smash[t]`, `\smash[b]`: the ink and the advance stay the
/// body's, only the box's height and/or depth go to zero.
#[test]
fn smash_keeps_the_ink_and_zeroes_the_box_like_pdflatex() {
    check("smash");
}

/// The nineteen `\ext@arrow`s: the arrow stretched to the wider `\scriptstyle`
/// label plus mathtools' kerns, with `\cleaders` fill pieces.
#[test]
fn extensible_arrows_stretch_to_their_labels_like_pdflatex() {
    check("xarrows");
}
