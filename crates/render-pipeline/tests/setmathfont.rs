//! The math half of the font system (`docs/proposals/font-system-math.md`):
//! `\setmathfont{Family}`, `\usepackage{unicode-math}` and the manifest's
//! `[fonts] math` set mathematics from an OpenType `MATH` face through
//! `MathProvider::Otf`, with LuaTeX's geometry for the constants Appendix G
//! has no parameter for (`mlist.c`; the Commander's 2026-09-19 ruling).
//!
//! The oracle numbers quoted below come from LuaLaTeX + unicode-math on this
//! machine (`/Library/TeX/texbin/lualatex`, oracle only, never in the
//! product path): a `\directlua` walk of `\setbox0\hbox{$...$}` printing
//! every glyph's origin relative to the box, for Latin Modern Math and STIX
//! Two Math. Nothing here runs TeX.
//!
//! Three kinds of test:
//!
//! * the bundled Latin Modern Math (`apps/mac/Fonts`, staged for the index):
//!   selection, precedence, diagnostics, alphabets, the hw1 envelope;
//! * this Mac's STIX Two Math (`/System/Library/Fonts/Supplemental`): the
//!   `MathKernInfo` cut-ins, the font's script percentages and a glyph
//!   assembly -- what Latin Modern Math (no kern tables, TeX-shaped
//!   constants) cannot exercise; skipped with a note where it is absent;
//! * the control: a document naming no math font renders exactly as before.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, Provenance};
use flashtex_render_pipeline::{render, FontSet, FontSettings, RenderOptions, Rendered};

/// A staged directory of faces (by file name) for the index, so the tests
/// depend on no installed font beyond the bundle.
struct Staged(PathBuf);

impl Staged {
    fn new(tag: &str, files: &[&str]) -> Staged {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts");
        let dir = std::env::temp_dir().join(format!("flashtex-mathfont-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for f in files {
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

const LM_MATH_SET: [&str; 6] = [
    "latinmodern-math.otf",
    "lmroman10-regular.otf",
    "lmroman10-bold.otf",
    "lmroman10-italic.otf",
    "lmsans10-regular.otf",
    "lmmono10-regular.otf",
];

const STIX: &str = "/System/Library/Fonts/Supplemental/STIXTwoMath.otf";

fn render_with(text: &str, fonts: &FontSet, options: &RenderOptions) -> Rendered {
    let sources = [SourceDocument { path: "main.tex", text }];
    render(&sources, "main.tex", 7, "mathfont", fonts, options)
}

fn doc(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{amsmath,amssymb}}\n{preamble}\n\\begin{{document}}\nA\n\n{body}\n\\end{{document}}\n")
}

/// One placed glyph: cluster text, PostScript name, origin (pt, y up from
/// the page top negated) and advance (pt).
#[derive(Debug, Clone)]
struct Placed {
    text: String,
    font: String,
    x: f64,
    y: f64,
    advance: f64,
    size: f64,
    /// The cluster's source range.
    src: (usize, usize),
}

const BP_TO_PT: f64 = 72.27 / 72.0;

fn glyphs(r: &Rendered) -> Vec<Placed> {
    let mut out = Vec::new();
    for page in &r.v2.pages {
        for it in page.resident_items() {
            let Item::GlyphRun(run) = it else { continue };
            let font = r.v2.fonts.iter().find(|f| f.font_id == run.font_id).expect("run's font is published");
            for (ci, c) in run.clusters.iter().enumerate() {
                let src = match &c.provenance {
                    Provenance::Source(s) => (s.start_byte, s.end_byte),
                    _ => (0, 0),
                };
                let text = String::from_utf8_lossy(&run.text.as_bytes()[c.text_start_byte..c.text_end_byte]).into_owned();
                for g in run.glyphs.iter().filter(|g| g.cluster as usize == ci) {
                    out.push(Placed {
                        text: text.clone(),
                        font: font.postscript_name.clone(),
                        x: g.origin_x.to_bp() * BP_TO_PT,
                        y: -g.baseline_y.to_bp() * BP_TO_PT,
                        advance: g.advance_x.to_bp() * BP_TO_PT,
                        size: run.font_size.to_bp() * BP_TO_PT,
                        src,
                    });
                }
            }
        }
    }
    out
}

/// The glyphs of the formula whose source contains `needle`, left to right.
fn formula<'a>(text: &str, gs: &'a [Placed], needle: &str) -> Vec<&'a Placed> {
    let at = text.find(needle).unwrap_or_else(|| panic!("{needle:?} in the source"));
    let mut v: Vec<&Placed> = gs.iter().filter(|g| g.src.0 <= at && at < g.src.1).collect();
    v.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    v
}

fn find<'a>(gs: &[&'a Placed], text: &str) -> &'a Placed {
    gs.iter().find(|g| g.text == text).unwrap_or_else(|| panic!("no glyph {text:?} in {:?}", gs.iter().map(|g| &g.text).collect::<Vec<_>>()))
}

fn codes(r: &Rendered) -> Vec<(String, String)> {
    r.v2.diagnostics.iter().map(|d| (d.code.clone(), d.message.clone())).collect()
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn setmathfont_latin_modern_math_sets_every_math_glyph_from_the_math_face() {
    if !common::lm_available() {
        return;
    }
    let s = Staged::new("select", &LM_MATH_SET);
    let fonts = s.fonts();
    let text = doc("\\setmathfont{Latin Modern Math}", "$f(x) = 2\\sqrt{x} + 10 - x$ and $\\mathbf{x}\\mathsf{y}\\mathtt{z}\\mathit{d}\\mathcal{A}\\mathfrak{B}$");
    let r = render_with(&text, &fonts, &RenderOptions::default());
    let d = codes(&r);
    assert!(!d.iter().any(|(c, _)| c == "math_font_unavailable" || c == "math_font_not_implemented"), "{d:?}");
    let gs = glyphs(&r);
    // Digits, parentheses and the relation come from the math face too (the
    // TeX route sets them from the roman text face), and `-` is the minus.
    let f = formula(&text, &gs, "f(x)");
    for t in ["𝑓", "(", "𝑥", ")", "=", "2", "√", "+", "1", "0", "−"] {
        assert_eq!(find(&f, t).font, "LatinModernMath-Regular", "{t}: {f:?}");
    }
    // LuaLaTeX: `f` 4.9 pt wide with a 0.9 pt italic correction before `(`.
    let (ff, paren) = (find(&f, "𝑓"), find(&f, "("));
    assert!(close(paren.x - ff.x, 5.8, 0.01), "{}", paren.x - ff.x);
    // The text math alphabets are the text faces (unicode-math), the
    // script and fraktur ones the math face's own blocks.
    let a = formula(&text, &gs, "\\mathbf{x}");
    assert_eq!(find(&a, "𝐱").font, "LMRoman10-Bold");
    assert_eq!(find(&a, "𝗒").font, "LMSans10-Regular");
    assert_eq!(find(&a, "𝚣").font, "LMMono10-Regular");
    assert_eq!(find(&a, "𝑑").font, "LMRoman10-Italic");
    assert_eq!(find(&a, "𝒜").font, "LatinModernMath-Regular");
    assert_eq!(find(&a, "𝔅").font, "LatinModernMath-Regular");
}

#[test]
fn unicode_math_alone_selects_latin_modern_math_and_the_manifest_sits_below_the_document() {
    if !common::lm_available() {
        return;
    }
    let s = Staged::new("precedence", &LM_MATH_SET);
    let fonts = s.fonts();
    // `\usepackage{unicode-math}` with no `\setmathfont`: the package's
    // default face, and its load is no longer "not implemented".
    let text = doc("\\usepackage{unicode-math}", "$f(x)=1$");
    let r = render_with(&text, &fonts, &RenderOptions::default());
    let gs = glyphs(&r);
    let f = formula(&text, &gs, "f(x)");
    assert!(f.iter().all(|g| g.font == "LatinModernMath-Regular"), "{f:?}");
    assert!(!codes(&r).iter().any(|(_, m)| m.contains("unicode-math")), "{:?}", codes(&r));
    // The manifest alone selects the face as well ...
    let plain = doc("", "$f(x)=1$");
    let manifest = RenderOptions {
        fonts: Some(FontSettings { math: Some("Latin Modern Math".into()), ..FontSettings::default() }),
        ..RenderOptions::default()
    };
    let r = render_with(&plain, &fonts, &manifest);
    let gs = glyphs(&r);
    assert!(formula(&plain, &gs, "f(x)").iter().all(|g| g.font == "LatinModernMath-Regular"));
    // ... and a document `\setmathfont` outranks it (a family the manifest
    // could not have chosen: the roman face, which has no MATH table, so the
    // warning names the document's command, not the manifest).
    let over = doc("\\setmathfont{Latin Modern Roman}", "$f(x)=1$");
    let r = render_with(&over, &fonts, &manifest);
    let d = codes(&r);
    assert!(d.iter().any(|(c, m)| c == "math_font_unavailable" && m.contains("\\setmathfont{Latin Modern Roman}") && m.contains("no OpenType MATH table")), "{d:?}");
}

#[test]
fn a_missing_family_or_a_body_setmathfont_keeps_tex_metrics_byte_for_byte() {
    if !common::lm_available() {
        return;
    }
    let s = Staged::new("missing", &LM_MATH_SET);
    let fonts = s.fonts();
    let body = "$f(x) = \\frac{a}{b} + \\sum_{i=1}^n x_i^2$ \\[ \\int_0^1 \\hat{x} \\left(\\frac{a}{b}\\right) \\]";
    let plain = render_with(&doc("", body), &fonts, &RenderOptions::default());
    let missing = render_with(&doc("\\setmathfont{No Such Math}", body), &fonts, &RenderOptions::default());
    let d = codes(&missing);
    assert!(d.iter().any(|(c, m)| c == "math_font_unavailable" && m.contains("\"No Such Math\" not found") && m.contains("--list-math-fonts")), "{d:?}");
    // The same glyphs at the same places (the source offsets differ by the
    // preamble line, so they are left out of the comparison).
    let placed = |r: &Rendered| -> Vec<(String, String, i64, i64, i64)> {
        glyphs(r).iter().map(|g| (g.text.clone(), g.font.clone(), (g.x * 1000.0).round() as i64, (g.y * 1000.0).round() as i64, (g.advance * 1000.0).round() as i64)).collect()
    };
    assert_eq!(placed(&plain), placed(&missing));
    // In the body: noted and ignored (unicode-math's own rule).
    let late = render_with(&doc("", &format!("\\setmathfont{{Latin Modern Math}} {body}")), &fonts, &RenderOptions::default());
    let d = codes(&late);
    assert!(d.iter().any(|(c, m)| c == "math_font_ignored" && m.contains("document body")), "{d:?}");
    let faces = |r: &Rendered| -> Vec<(String, String)> { glyphs(r).iter().filter(|g| g.src.0 > 0).map(|g| (g.text.clone(), g.font.clone())).collect() };
    assert_eq!(faces(&plain), faces(&late));
    // Options are accepted and noted, the family applied.
    let scaled = render_with(&doc("\\setmathfont[Scale=MatchLowercase,range=\\int]{Latin Modern Math}", body), &fonts, &RenderOptions::default());
    let d = codes(&scaled);
    assert!(d.iter().any(|(c, m)| c == "fontspec_feature_ignored" && m.contains("\\setmathfont{Latin Modern Math}") && m.contains("range=")), "{d:?}");
    assert!(glyphs(&scaled).iter().any(|g| g.font == "LatinModernMath-Regular" && g.text == "("));
}

/// `\\setmathfont[Scale=..]` scales the math glyph metrics like a text
/// family's `Scale=`: a factor of 1.2 widens every advance by 1.2, and a
/// `Scale`-only selection draws no ignored-option note.
#[test]
fn setmathfont_scale_factor_scales_math_glyph_metrics() {
    if !common::lm_available() {
        return;
    }
    let s = Staged::new("math-scale", &LM_MATH_SET);
    let fonts = s.fonts();
    let body = "$f(x) = 2$";
    let r0 = render_with(&doc("\\setmathfont{Latin Modern Math}", body), &fonts, &RenderOptions::default());
    let r1 = render_with(&doc("\\setmathfont[Scale=1.2]{Latin Modern Math}", body), &fonts, &RenderOptions::default());
    let d = codes(&r1);
    assert!(!d.iter().any(|(c, m)| c == "fontspec_feature_ignored" && m.contains("Scale")), "{d:?}");
    let (g0, g1) = (glyphs(&r0), glyphs(&r1));
    let adv = |text: &str, gs: &[Placed], glyph: &str| -> f64 {
        let f = formula(text, gs, "f(x)");
        find(&f, glyph).advance
    };
    let (plain, scaled) = (doc("\\setmathfont{Latin Modern Math}", body), doc("\\setmathfont[Scale=1.2]{Latin Modern Math}", body));
    for needle in ["=", "2", "("] {
        let (a0, a1) = (adv(&plain, &g0, needle), adv(&scaled, &g1, needle));
        assert!(close(a1 / a0, 1.2, 0.001), "{needle}: {a0} -> {a1}");
    }
}

/// The LuaTeX oracle numbers for Latin Modern Math at 10 pt (the box walk
/// described in the module comment), against this pipeline's own layout
/// of the same formulas under `\setmathfont{Latin Modern Math}`.
#[test]
fn latin_modern_math_geometry_matches_the_luatex_oracle() {
    if !common::lm_available() {
        return;
    }
    let s = Staged::new("oracle", &LM_MATH_SET);
    let fonts = s.fonts();
    let text = doc(
        "\\setmathfont{Latin Modern Math}",
        "$x^2_i$ and $T^a$ and $\\frac{a}{b}$ and \\[\\sqrt{\\frac{a}{b}}\\] and \\[\\sum_{i=1}^n\\] and $\\hat{x}$ and $\\vec{v}$ and $\\hat{A}$ and \\[\\int_0^1\\] and $\\overline{x}^2$ and \\[\\left(\\frac{\\frac{\\frac{a}{b}}{\\frac{c}{d}}}{\\frac{\\frac{a}{b}}{\\frac{c}{d}}}\\right)\\]",
    );
    let r = render_with(&text, &fonts, &RenderOptions::default());
    let gs = glyphs(&r);
    let rel = |f: &[&Placed], from: &str, to: &str| {
        let (a, b) = (find(f, from), find(f, to));
        (b.x - a.x, b.y - a.y)
    };
    let tol = 0.01;
    // `x^2_i`: the `2` at (5.72, 3.63), the `i` at (5.72, −2.611): Rule 18e
    // with SubSuperscriptGapMin 1.6 pt raising the subscript's drop from
    // SubscriptShiftDown 2.47 by the clearance; script glyphs are the
    // `ssty` forms (𝑖.st is 4.641 pt tall at 7 pt).
    let f = formula(&text, &gs, "x^2_i");
    let (dx, dy) = rel(&f, "𝑥", "2");
    assert!(close(dx, 5.72, tol) && close(dy, 3.63, tol), "{dx} {dy}");
    let (dx, dy) = rel(&f, "𝑥", "𝑖");
    assert!(close(dx, 5.72, tol) && close(dy, -2.611, tol), "{dx} {dy}");
    // `T^a`: the superscript after 𝑇's 1.48 pt italic correction.
    let f = formula(&text, &gs, "T^a");
    let (dx, dy) = rel(&f, "𝑇", "𝑎");
    assert!(close(dx, 7.32, tol) && close(dy, 3.63, tol), "{dx} {dy}");
    // `\frac{a}{b}` in text style: numerator up 3.94, denominator down
    // 3.45, the denominator centred under the wider numerator (the 7 pt
    // `ssty` forms: 𝑎 4.34 wide, 𝑏 3.514) 0.413 pt in.
    let f = formula(&text, &gs, "\\frac{a}{b}$");
    let (dx, dy) = rel(&f, "𝑎", "𝑏");
    assert!(close(dx, 0.413, tol) && close(dy, -7.39, tol), "{dx} {dy}");
    // `\sqrt{\frac{a}{b}}` in display style: the 24 pt sign (14.5 pt above
    // its origin) set with its origin 0.55 pt above the baseline so its top
    // meets the rule's top at 15.05, the numerator at 6.77 (LuaTeX
    // `make_radical` with RadicalRuleThickness 0.4 and the display gap
    // 1.48, the sign re-boxed to the rule thickness).
    let f = formula(&text, &gs, "\\sqrt{\\frac{a}{b}}");
    let (dx, dy) = rel(&f, "√", "𝑎");
    assert!(close(dx, 11.2, tol) && close(dy, 6.22, tol), "{dx} {dy}");
    assert!(close(find(&f, "√").advance, 10.0, tol));
    // `\sum_{i=1}^n` in display style: Rule 13a with UpperLimitGapMin 2.0
    // and LowerLimitBaselineDropMin 6.0 on the 14 pt operator; the limits
    // centred on its 14.44 pt advance.
    let f = formula(&text, &gs, "\\sum_{i=1}^n");
    let sum = find(&f, "∑");
    let (dx, dy) = rel(&f, "∑", "𝑛");
    assert!(close(dx, 4.749, tol) && close(dy, 11.57, tol), "{dx} {dy}");
    let (dx, dy) = rel(&f, "∑", "𝑖");
    assert!(close(dx, 1.0915, tol) && close(dy, -10.818, tol), "{dx} {dy}");
    assert!(close(sum.advance, 14.44, tol));
    // Accents by top-accent anchors: `\hat{x}` puts the combining hat's
    // origin 5.93 pt right of 𝑥's (3.29 − (−2.64)) on the baseline (𝑥 is
    // lower than AccentBaseHeight 4.5); `\vec{v}` 5.58 (2.94 + 2.64, the
    // glyph's anchor, not the box's centre); `\hat{A}` 8.14 right and 2.66
    // up (𝐴 is 7.16 tall), with the fixed accent, never the 6.4 pt variant.
    let f = formula(&text, &gs, "\\hat{x}");
    let (dx, dy) = rel(&f, "𝑥", "\u{0302}");
    assert!(close(dx, 5.93, tol) && close(dy, 0.0, tol), "{dx} {dy}");
    let f = formula(&text, &gs, "\\vec{v}");
    let (dx, dy) = rel(&f, "𝑣", "\u{20D7}");
    assert!(close(dx, 5.58, tol) && close(dy, 0.0, tol), "{dx} {dy}");
    let f = formula(&text, &gs, "\\hat{A}");
    let (dx, dy) = rel(&f, "𝐴", "\u{0302}");
    assert!(close(dx, 8.14, tol) && close(dy, 2.66, tol), "{dx} {dy}");
    assert!(close(find(&f, "\u{0302}").advance, 0.0, tol), "the fixed accent");
    // `\int_0^1` in display style, no limits: the 9.99 pt operator keeps
    // its advance, the `1` starts at it and the `0` its 5.91 pt italic
    // correction to the left (LuaTeX's `\mathnolimitsmode` 0), 11.11 up
    // and 10.61 down.
    let f = formula(&text, &gs, "\\int_0^1");
    let (dx, dy) = rel(&f, "∫", "1");
    assert!(close(dx, 9.99, tol) && close(dy, 11.11, tol), "{dx} {dy}");
    let (dx, dy) = rel(&f, "∫", "0");
    assert!(close(dx, 4.08, tol) && close(dy, -10.61, tol), "{dx} {dy}");
    // `\overline{x}^2`: the superscript on the 6.42 pt bar box rises
    // 6.42 − 2.5 (SuperscriptBaselineDropMax at the current size, not the
    // script font's 1.75).
    let f = formula(&text, &gs, "\\overline{x}^2");
    let (dx, dy) = rel(&f, "𝑥", "2");
    assert!(close(dx, 5.72, tol) && close(dy, 3.92, tol), "{dx} {dy}");
    // The tall `\left(`: an assembly of the bottom hook, one extender and
    // the top hook, the joints overlapping 1.3546 pt each (LuaTeX's glue
    // between parts, packed to 32.171 pt), the whole centred on the axis.
    let f = formula(&text, &gs, "\\left(\\frac");
    let parens: Vec<&&Placed> = f.iter().filter(|g| g.text == "(").collect();
    assert_eq!(parens.len(), 3, "{f:?}");
    let ys: Vec<f64> = parens.iter().map(|g| g.y).collect();
    let mut sorted = ys.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!(close(sorted[1] - sorted[0], 13.5954, tol), "{sorted:?}");
    assert!(close(sorted[2] - sorted[1], 3.6254, tol), "{sorted:?}");
    let letters: Vec<&&Placed> = f.iter().filter(|g| g.text == "𝑎").collect();
    assert_eq!(letters.len(), 2);
    let a_top = letters.iter().map(|g| g.y).fold(f64::NEG_INFINITY, f64::max);
    assert!(close(a_top - sorted[0], 18.1479 + 13.5854, tol), "{a_top} {sorted:?}");
}

/// STIX Two Math, where the OpenType-only rules bite: cut-in kerns, a
/// radical sign whose ink sits above the baseline, the face's own script
/// percentages (70/55) with its `ssty` forms, and a five-part assembly with
/// unequal joints. The oracle numbers are LuaLaTeX's at 10 pt.
#[test]
fn stix_two_math_kerns_radicals_and_assemblies_match_the_luatex_oracle() {
    if !common::lm_available() {
        return;
    }
    if !Path::new(STIX).is_file() {
        eprintln!("SKIP: {STIX} not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]).with_index_dirs(vec![PathBuf::from("/System/Library/Fonts/Supplemental")]);
    let text = doc(
        "\\setmathfont{STIX Two Math}",
        "$V_a$ and $A^2$ and $F^2_a$ and $\\sqrt{x}$ and $\\sqrt[3]{x}$ and \\[\\sum_{i=1}^n\\] and $\\hat{A}$ and \\[\\left(\\frac{\\frac{\\frac{a}{b}}{\\frac{c}{d}}}{\\frac{\\frac{a}{b}}{\\frac{c}{d}}}\\right)\\] and $f^{2^3}$",
    );
    let r = render_with(&text, &fonts, &RenderOptions::default());
    let d = codes(&r);
    assert!(!d.iter().any(|(c, _)| c == "math_font_unavailable"), "{d:?}");
    let gs = glyphs(&r);
    assert!(gs.iter().any(|g| g.font == "STIXTwoMath-Regular"), "{:?}", gs.iter().map(|g| &g.font).collect::<Vec<_>>());
    let rel = |f: &[&Placed], from: &str, to: &str| {
        let (a, b) = (find(f, from), find(f, to));
        (b.x - a.x, b.y - a.y)
    };
    let tol = 0.01;
    // `V_a`: 𝑉 is 6.4 pt wide; the subscript's top-left and 𝑉's
    // bottom-right staircases move it 2.22 pt in (LuaTeX `find_math_kern`,
    // the smaller of the sums at the script's top and the base's bottom);
    // SubscriptShiftDown 2.1.
    let f = formula(&text, &gs, "V_a");
    let (dx, dy) = rel(&f, "𝑉", "𝑎");
    assert!(close(dx, 4.18, tol) && close(dy, -2.1, tol), "{dx} {dy}");
    // `A^2`: 6.81 − 0.7 (𝐴 has no italic correction; the kern is the
    // top-right staircase's).
    let f = formula(&text, &gs, "A^2");
    let (dx, dy) = rel(&f, "𝐴", "2");
    assert!(close(dx, 6.11, tol) && close(dy, 3.6, tol), "{dx} {dy}");
    // `F^2_a`: the superscript after 𝐹's 0.65 pt italic correction, the
    // subscript 1.6 pt in.
    let f = formula(&text, &gs, "F^2_a");
    let (dx, _) = rel(&f, "𝐹", "2");
    assert!(close(dx, 6.48, tol), "{dx}");
    let (dx, _) = rel(&f, "𝐹", "𝑎");
    assert!(close(dx, 4.23, tol), "{dx}");
    // `\sqrt{x}`: STIX's sign is 9.22 pt tall above its origin and 2.65
    // deep; re-boxed to the 0.68 pt rule it sits 0.175 pt below the
    // baseline with the rule's bottom at 8.365.
    let f = formula(&text, &gs, "\\sqrt{x}$");
    let (dx, dy) = rel(&f, "𝑥", "√");
    assert!(close(dx, -7.94, tol) && close(dy, -0.175, tol), "{dx} {dy}");
    // `\sum_{i=1}^n` in display style: the operator centred under the
    // wider lower limit (its `1` keeps a 0.14 pt correction kern), the
    // limits at 12.205 and −10.745.
    let f = formula(&text, &gs, "\\sum_{i=1}^n");
    let (dx, dy) = rel(&f, "𝑖", "∑");
    assert!(close(dx, 0.1345, tol) && close(dy, 10.745 + 0.015, tol), "{dx} {dy}");
    let (dx, dy) = rel(&f, "𝑖", "𝑛");
    assert!(close(dx, 3.4055, tol) && close(dy, 10.745 + 12.205, tol), "{dx} {dy}");
    // `\hat{A}`: 𝐴 is 6.61 tall, AccentBaseHeight 4.8: the hat 1.81 up and
    // 7.1 right (anchors 4.8 and −2.3).
    let f = formula(&text, &gs, "\\hat{A}");
    let (dx, dy) = rel(&f, "𝐴", "\u{0302}");
    assert!(close(dx, 7.1, tol) && close(dy, 1.81, tol), "{dx} {dy}");
    // The assembly: bottom hook, three extenders, top hook; the hook
    // joints overlap 1.75 pt (their 2.5 pt connector stretched half way to
    // the 1 pt minimum), the extender joints 5.5 (10 → 1): rises of
    // 12.73 − 1.75 and 12.52 − 5.5.
    let f = formula(&text, &gs, "\\left(\\frac");
    let mut ys: Vec<f64> = f.iter().filter(|g| g.text == "(").map(|g| g.y).collect();
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(ys.len(), 5, "{ys:?}");
    let steps: Vec<f64> = ys.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(close(steps[0], 12.73 - 1.75, 0.02) && close(steps[3], 12.52 - 1.75, 0.02), "{steps:?}");
    assert!(close(steps[1], 12.52 - 5.5, 0.02) && close(steps[2], 12.52 - 5.5, 0.02), "{steps:?}");
    // Script sizes from the face: 70% and 55% (the `3` is `three.ssty2`,
    // 561 units, at 5.5 pt).
    let f = formula(&text, &gs, "f^{2^3}");
    let three = find(&f, "3");
    assert!(close(three.size, 5.5, tol), "{}", three.size);
    assert!(close(three.advance, 0.561 * 5.5, tol), "{}", three.advance);
    assert!(close(find(&f, "2").size, 7.0, tol));
}

/// The acceptance from the proposal: hw1 under `\setmathfont{Latin Modern
/// Math}` against its TeX-metrics layout, glyph by glyph. Latin Modern
/// Math's constants are Computer Modern's, so what moves is (a) the font's
/// own designs where the TeX route substitutes another face -- `\mathbb`
/// from New Computer Modern Math (msbm's design, 7.22 pt for ℝ against the
/// open-face 6.39) -- (b) glyphs the TeX route sets from `cmex` sizes the
/// OpenType variants do not have (`\bigl(` is 12 pt in cmex, 10.95 in the
/// face), and (c) the three rules LuaTeX reads from the table where TeX has
/// a different fixed value: a subscript alone drops SubscriptShiftDown
/// (2.47 pt) rather than σ₁₆ (1.5), a display-style superscript rises
/// SuperscriptShiftUp (3.63) rather than σ₁₃ (4.12), and `\scriptspace` is
/// SpaceAfterScript (0.56 against 0.5). The test pins that envelope: every
/// formula without a double-struck letter within 1.5 bp, the plain ones
/// within 0.25, and reports the worst.
#[test]
fn hw1_under_latin_modern_math_stays_within_the_measured_envelope() {
    if !common::lm_available() {
        return;
    }
    let s = Staged::new("hw1", &LM_MATH_SET);
    let fonts = s.fonts();
    let hw1 = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/real-world/hw1/HW1.tex")).expect("hw1 fixture");
    // The selection replaces a same-length comment line so byte offsets
    // (the cluster provenance both sides are keyed on) stay put.
    let marker = "\\usepackage{microtype}\n";
    assert!(hw1.contains(marker));
    let selection = "\\setmathfont{Latin Modern Math}%\n";
    let padded = format!("{marker}{}{}\n", "%", " ".repeat(selection.len() - 2));
    assert_eq!(padded.len(), marker.len() + selection.len());
    let plain_text = hw1.replacen(marker, &padded, 1);
    let otf_text = hw1.replacen(marker, &format!("{marker}{selection}"), 1);
    assert_eq!(plain_text.len(), otf_text.len());
    let plain = render_with(&plain_text, &fonts, &RenderOptions::default());
    let otf = render_with(&otf_text, &fonts, &RenderOptions::default());
    assert!(!codes(&otf).iter().any(|(c, _)| c == "math_font_unavailable"), "{:?}", codes(&otf));
    // Glyphs per source range, in x order; the ranges are the formulas'.
    let group = |r: &Rendered| -> BTreeMap<(usize, usize), Vec<Placed>> {
        let mut m: BTreeMap<(usize, usize), Vec<Placed>> = BTreeMap::new();
        for g in glyphs(r) {
            m.entry(g.src).or_default().push(g);
        }
        for v in m.values_mut() {
            v.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
        }
        m
    };
    let (a, b) = (group(&plain), group(&otf));
    let mut worst: (f64, String) = (0.0, String::new());
    let mut plain_worst: f64 = 0.0;
    let mut compared = 0;
    for (key, gb) in &b {
        let Some(ga) = a.get(key) else { continue };
        let src = &otf_text[key.0..key.1.min(otf_text.len())];
        if key.1 - key.0 < 4 || !src.contains('$') && !src.contains("\\[") {
            continue;
        }
        if src.contains("\\R") || src.contains("\\Q") || src.contains("\\N") || src.contains("\\Z") || src.contains("mathbb") {
            continue;
        }
        assert_eq!(ga.len(), gb.len(), "{src:?}: {} glyphs against {}", ga.len(), gb.len());
        // Nearest same-text glyph, so the two sides' run grouping does not
        // matter; the math italic letters map back to their ASCII.
        let plain_text_of = |t: &str| -> String {
            t.chars()
                .map(|c| match c as u32 {
                    0x1D44E..=0x1D467 => char::from_u32('a' as u32 + (c as u32 - 0x1D44E)).unwrap(),
                    0x1D434..=0x1D44D => char::from_u32('A' as u32 + (c as u32 - 0x1D434)).unwrap(),
                    0x210E => 'h',
                    _ => c,
                })
                .collect()
        };
        // Letters, digits, relations and `\mid` only: nothing whose glyph
        // the two routes take from different designs.
        let simple = !src.contains('^') && !src.contains('_') && !src.replace("\\mid", "").replace("\\[", "").replace("\\]", "").contains('\\');
        for g in gb {
            let best = ga
                .iter()
                .filter(|h| plain_text_of(&h.text) == plain_text_of(&g.text))
                .map(|h| (h.x - g.x).abs().max((h.y - g.y).abs()))
                .fold(f64::INFINITY, f64::min);
            if best.is_finite() {
                compared += 1;
                let bp = best / BP_TO_PT;
                if bp > worst.0 {
                    worst = (bp, format!("{:?} in {src:?}", g.text));
                }
                if simple {
                    plain_worst = plain_worst.max(bp);
                }
            }
        }
    }
    eprintln!("hw1 under Latin Modern Math: {compared} glyphs compared; worst {:.3} bp at {}", worst.0, worst.1);
    assert!(compared > 150, "{compared}");
    assert!(worst.0 <= 1.5, "worst {:.3} bp at {}", worst.0, worst.1);
    // The plain formulas differ by the italic corrections the two designs
    // give a letter before a parenthesis (`$D(m,n)$`: 0.21 bp).
    assert!(plain_worst <= 0.25, "plain formulas: {plain_worst:.3} bp");
}
