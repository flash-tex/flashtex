//! `\verb` and `verbatim` against pdfLaTeX, glyph origin by glyph origin.
//!
//! The `.tex` sources and the `reference/*.json` origins come from draft
//! PR #145, which measured them with **MacTeX 2026**
//! (`pdfTeX 3.141592653-2.6-1.40.29`, two passes, `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1`). They are committed oracle data and are never
//! regenerated here. Only the test material was taken from #145 -- the
//! rendering is written against main's NFSS font selection, not #145's
//! rival `Role::Mono`/`MonoMetrics` model.
//!
//! The two distributions were checked against each other rather than
//! assumed to agree: this host's **TeX Live 2025** (`pdfTeX
//! 3.141592653-2.6-1.40.27`) was run over all seventeen fixtures with
//! `fixtures/verbatim/oracle.py` and reproduced every committed reference
//! to **0.001 bp or better**, with identical rule sets and page counts. A
//! number measured here can therefore be compared with these files
//! directly.
//!
//! Units are bp; `y` runs down from the page top. A reference glyph must
//! find a candidate within [`TOL`] in both x and y (Chebyshev), glyph counts
//! per page must be equal, and **no substitution diagnostic may appear** —
//! a run laid out on a fallback face is a failure, not a pass.

mod common;

use std::path::PathBuf;

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// pdfTeX writes glyph origins to 5 decimal places; 0.1 bp is a hair over
/// a thousandth of an em at 10 pt and well under a rounding step.
const TOL: f64 = 0.1;

const SUBSTITUTION_CODES: [&str; 4] = [
    "font_unavailable",
    "required_metrics_unavailable",
    "ec_metrics_unavailable",
    "font_outline_substituted",
];

/// The fixtures this lane sets exactly as pdfLaTeX does.
const GATED: [&str; 13] = [
    "01-verbatim-basic",
    "02-verbatim-ligatures",
    "03-verbatim-tabs",
    "04-verbatim-blank-lines",
    "05-verbatim-star",
    "06-verb-delimiters",
    "07-verbatim-t1",
    "08-verbatim-lmodern",
    "09-verbatim-vmode",
    "11-verbatim-11pt",
    "12-verbatim-12pt",
    "13-verbatim-long-line",
    "14-verbatim-pagebreak",
];

/// The listings fixtures PR #248 committed with TL2025 references. The
/// listings rendering engine is a separate body of work (PR #145's
/// `listings.rs` was 1623 lines against a font model main has replaced); it
/// is being rebuilt on the current model in its own lane. Each entry names
/// the part of listings' geometry it needs, from the measured
/// specification, so nothing here is a placeholder.
const LISTINGS_NOT_YET: [(&str, &str); 15] = [
    ("16-lst-default",
     "`[c]fixed` columns in the surrounding roman face: a token of N characters is set in an hbox \
      N cells wide with N+1 `\\hss`, so the first glyph sits N(W-w)/(N+1) in. Also needs TS1 for \
      `*` and CMSY for the braces."),
    ("17-lst-tt-fixed",
     "`[c]fixed` with `basicstyle=\\ttfamily`: cell width W = 0.6em of the basicstyle font, fixed \
      once at `InitVars` (`fontadjust=false`)."),
    ("18-lst-tt-flexible",
     "`columns=flexible`: natural token boxes, and leading whitespace worth `\\lst@width` = 0.45em \
      each rather than a space glyph."),
    ("19-lst-fullflexible",
     "`columns=fullflexible`. Note this fixture's reference is byte-identical to 18's: this code \
      never produces positive lost space mid-line, so it does not actually discriminate the two \
      modes. A fixture that does is still wanted."),
    ("20-lst-numbers",
     "`numbers=left`: `\\lst@PlaceNumber` is an `\\llap`, so the code x is untouched and the \
      number's *right* edge sits at `leftmargin - numbersep`."),
    ("21-lst-frame-single",
     "`frame=single`: four rules at `framerule` .4pt, `x = leftmargin - framesep - framerule`, \
      `w = \\textwidth + 2(framesep + framerule)`, and `\\lst@frameInit`'s negative correction \
      cancelling the interline glue."),
    ("22-lst-frame-lines-numbers",
     "`frame=lines` + `numbers=left` under `\\small`: a t/b-only frame's rules are *not* widened by \
      `framesep`, and `numberstyle` empty means the number is `\\normalfont` at the current size."),
    ("23-lst-c-keywords",
     "`language=C`: `commentstyle=\\itshape` (CMITT10) changes glyph positions inside a token, and \
      `keywordstyle=\\bfseries` is substituted away in OT1 cmtt (`OT1/cmtt/bx/n` does not exist)."),
    ("24-lst-python-keywords",
     "`language=Python` at `\\small`: code in CMTT9, comments in CMITT10 *scaled to 9pt* -- there \
      is no cmitt9, so the NFSS path must scale."),
    ("25-lst-java-roman",
     "`language=Java` with a roman basicstyle and CMBX10 keywords. Worth noting: this fixture \
      contains no ligature-forming pair at all, so it does not test what #145 thought it did. \
      Measured separately: listings suppresses ligatures *unconditionally*, roman basicstyle \
      included, matching `verbatim` glyph for glyph -- so a listing needs per-character shaping, \
      not `Shaper::shape_with`'s `\\@noligs` split."),
    ("26-lst-breaklines",
     "`breaklines=true`: ragged right, `breakindent` 20pt continuation, and no break mark \
      (`prebreak`/`postbreak` are empty and the `\\llap` is zero-width)."),
    ("27-lst-showstringspaces",
     "`showstringspaces`: `false` suppresses the visible-space glyph but the space keeps its \
      column -- x0 and xN are identical either way. Also pins that two adjacent listings *add* \
      both skips (24.0 pt between them), because `\\vspace` is not `\\addvspace`."),
    ("29-lst-tabs-gobble",
     "`tabsize=4, gobble=2`: unlike `\\@verbatim`, listings implements real absolute tab stops -- \
      `\\lst@ProcessTabulator` moves to the next multiple of `tabsize` in `\\lst@width` units, so a \
      tab costs 3 columns from column 1 and 4 from column 0."),
    ("30-lst-lstset-margin",
     "`xleftmargin=2em` under `\\footnotesize`: the margin is expanded *inside* the listing, after \
      `basicstyle`, so `2em` is 2 x cmtt8's quad = 17.00024 pt -- and cmtt8's em is 8.5 pt, not \
      8.4."),
    ("31-lst-t1-lmodern-bold",
     "T1 + lmodern: keywords resolve `T1/lmtt/bx/n` -> `T1/lmtt/b/n` -> `ec-lmtk10` \
      (LMMonoLt10-Bold), comments to `ec-lmtti10`. Here the keywords really do come out bold, \
      unlike fixture 23."),
];

/// Committed with their references but not gated yet, each for a reason
/// that names what is still missing. Listed here so the material is in the
/// tree and the follow-up is visible rather than forgotten.
const NOT_YET: [(&str, &str); 4] = [
    ("10-verbatim-itemize",
     "`\\@verbatim`'s `\\trivlist` inside a list. Measured with `\\showoutput`: pdfTeX sets \
      `\\leftskip 25.00003` (the enclosing `\\leftmargini`) on every verbatim line and 12 pt of \
      glue above it (`\\topsep` 8 pt + the enclosing list's `\\parsep` 4 pt, which `\\list` made \
      `\\parskip`). The pipeline lowers verbatim to a top-level flush-left paragraph, so it sets \
      the lines at the page margin with only `\\topsep`: 25 pt out and 4 pt up."),
    ("15-verbatim-small",
     "The size declaration in force. pdfTeX sets the body in CMTT9 (per-character advance 4.7073 bp \
      against CMTT10's 5.2303); the pipeline sets CMTT10. The compiler's `Inline::Verbatim` and \
      `Block::Verbatim` carry no `style`, so the declaration never reaches the pipeline, and \
      `declared_size` is explicit that the pipeline must not re-derive sizes from the source \
      (pin `b38e1884`). It is a compiler change plus a vendor re-pin."),
    ("28-lstinline",
     "The compiler typesets `\\lstset`'s argument as prose: 126 glyphs against pdfTeX's 115, the \
      extra 11 being the characters `basicstyle=` at the head of the first line. `\\lstinline` \
      itself is lexed correctly (PR #188, already in the vendored mirror). Needs the compiler to \
      consume listings' setup commands, plus a vendor re-pin."),
    ("32-verbatim-microtype",
     "microtype's protrusion on the surrounding roman text; the verbatim lines themselves are \
      already excluded (the default sets are `rm*`/`sf*`)."),
];

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/verbatim")
}

/// The bundled tree, with its three rooted metric directories — never the
/// process environment, so this behaves the same under CI's `FLASHTEX_*`
/// exports and without them.
fn bundled_fonts() -> Option<FontSet> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts");
    // The typewriter faces arrive with the bundled-font PR; until it lands
    // there is nothing to measure `\texttt` geometry against but a
    // substitution, which this test must never accept.
    if !root.join("lmmono10-regular.otf").is_file() || !root.join("texmf/fonts/tfm/jknappen/ec/ectt1000.tfm").is_file() {
        return None;
    }
    let texmf = root.join("texmf/fonts/tfm");
    Some(FontSet::with_dirs(
        vec![root],
        vec![texmf.join("public/lm"), texmf.join("jknappen/ec"), texmf.join("public/amsfonts/symbols")],
    ))
}

struct Reference {
    pages: usize,
    /// `(page, x, baseline y, text)`, page 1-based in the file.
    glyphs: Vec<(usize, f64, f64, String)>,
}

fn reference(name: &str) -> Reference {
    let path = fixtures_dir().join("reference").join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc = json::parse(&raw).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
    let n = |v: &Value| match v {
        Value::Num(n) => *n,
        _ => panic!("number expected in {}", path.display()),
    };
    Reference {
        pages: doc.get("pages").and_then(Value::as_i64).unwrap() as usize,
        glyphs: doc
            .get("glyphs")
            .and_then(Value::as_arr)
            .unwrap()
            .iter()
            .map(|g| {
                let a = g.as_arr().unwrap();
                // `[page (1-based), x, baseline y, text, font, size]`.
                (a[0].as_i64().unwrap() as usize - 1, n(&a[1]), n(&a[2]), a[3].as_str().unwrap_or("?").to_string())
            })
            .collect(),
    }
}

/// `(pages, glyph origins)` of a fixture, plus the substitution diagnostics
/// it raised.
fn candidate(fonts: &FontSet, name: &str) -> (usize, Vec<(usize, f64, f64)>, Vec<String>) {
    let dir = fixtures_dir();
    let tex = std::fs::read_to_string(dir.join(format!("{name}.tex"))).unwrap();
    let docs = [SourceDocument { path: "main.tex", text: &tex }];
    let options = RenderOptions { project_root: Some(dir), ..RenderOptions::default() };
    let r = render(&docs, "main.tex", 1, "verbatim", fonts, &options);
    let subs = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| SUBSTITUTION_CODES.contains(&d.code.as_str()))
        .map(|d| format!("{}: {}", d.code, d.message))
        .collect();
    let mut glyphs = Vec::new();
    for (pi, page) in r.v2.pages.iter().enumerate() {
        for item in &page.items {
            if let flashtex_render_pipeline::display::Item::GlyphRun(run) = item {
                for g in &run.glyphs {
                    glyphs.push((pi, g.origin_x.to_bp(), g.baseline_y.to_bp()));
                }
            }
        }
    }
    (r.v2.pages.len(), glyphs, subs)
}

#[test]
fn verbatim_fixtures_match_pdflatex() {
    let Some(fonts) = bundled_fonts() else {
        eprintln!(
            "SKIP verbatim_oracle: apps/mac/Fonts carries no lmmono*.otf / ectt*.tfm yet \
             (they arrive with the bundled-typewriter-fonts PR). Without them every \\texttt \
             run is laid out on substituted roman metrics, which this test must not accept."
        );
        return;
    };
    let mut failures: Vec<String> = Vec::new();
    for name in GATED {
        let r = reference(name);
        let (pages, mut cand, subs) = candidate(&fonts, name);
        if !subs.is_empty() {
            failures.push(format!("{name}: laid out on a substituted face: {subs:?}"));
            continue;
        }
        if pages != r.pages {
            failures.push(format!("{name}: {} page(s), reference has {}", pages, r.pages));
            continue;
        }
        if cand.len() != r.glyphs.len() {
            failures.push(format!("{name}: {} glyphs, reference has {}", cand.len(), r.glyphs.len()));
            continue;
        }
        let (mut worst, mut missed) = (0.0f64, Vec::new());
        for (page, x, y, text) in &r.glyphs {
            let best = cand
                .iter()
                .enumerate()
                .filter(|(_, c)| c.0 == *page)
                .map(|(i, c)| (i, (c.1 - x).abs().max((c.2 - y).abs())))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            match best {
                Some((i, d)) if d <= TOL => {
                    worst = worst.max(d);
                    cand.remove(i);
                }
                other => missed.push(format!(
                    "    p{page} x={x:.3} y={y:.3} {text:?} nearest {:.3} bp away",
                    other.map_or(f64::INFINITY, |(_, d)| d)
                )),
            }
        }
        if !missed.is_empty() {
            missed.truncate(6);
            failures.push(format!("{name}: {} glyph(s) off\n{}", missed.len(), missed.join("\n")));
        } else {
            eprintln!("PASS {name:24} {:4} glyphs, worst {worst:.3} bp", r.glyphs.len());
        }
    }
    assert!(failures.is_empty(), "{} fixture(s):\n{}", failures.len(), failures.join("\n"));
}

/// Every fixture in the directory is either gated or listed with a reason,
/// so a new one cannot be added and quietly left unmeasured.
#[test]
fn every_committed_fixture_is_accounted_for() {
    let dir = fixtures_dir();
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.strip_suffix(".tex").map(str::to_string))
        .collect();
    on_disk.sort();
    let mut known: Vec<String> = GATED
        .iter()
        .map(|s| s.to_string())
        .chain(NOT_YET.iter().map(|(n, _)| n.to_string()))
        .chain(LISTINGS_NOT_YET.iter().map(|(n, _)| n.to_string()))
        .collect();
    known.sort();
    assert_eq!(on_disk, known, "every fixture must be gated or listed in NOT_YET with a reason");
    for name in &on_disk {
        assert!(
            dir.join("reference").join(format!("{name}.json")).is_file(),
            "{name} has no committed pdfLaTeX reference"
        );
    }
    for (_, why) in NOT_YET.iter().chain(LISTINGS_NOT_YET.iter()) {
        assert!(why.len() > 40, "a NOT_YET entry needs a real reason");
    }
}


/// The star form moves nothing: Cork slot 32 (`visiblespace`, the open box
/// `\verb*` and `verbatim*` set with `\char32`) is exactly as wide as the
/// typewriter font's interword space, so painting it cannot shift a line.
///
/// Measured from the bundled metrics rather than assumed -- this is the
/// invariant that lets the pipeline keep one width computation for both
/// forms.
#[test]
fn the_visible_space_is_exactly_one_interword_space_wide() {
    let Some(fonts) = bundled_fonts() else { return };
    let mut checked = 0;
    for file in ["ectt1000.tfm", "ectt0900.tfm", "ectt1200.tfm", "ec-lmtt10.tfm", "ec-lmtt9.tfm"] {
        let Ok(tfm) = fonts.tfm(file) else { continue };
        let slot32 = tfm.metrics(32).unwrap_or_else(|| panic!("{file} has no character at slot 32")).width;
        let space = tfm.param(2).unwrap_or_else(|| panic!("{file} has no fontdimen2"));
        assert_eq!(slot32, space, "{file}: slot 32 {slot32} vs fontdimen2 {space}");
        assert_eq!(tfm.param(3), Some(0), "{file}: a typewriter space stretches");
        assert_eq!(tfm.param(4), Some(0), "{file}: a typewriter space shrinks");
        checked += 1;
    }
    assert!(checked >= 3, "only {checked} typewriter TFM(s) found to measure");
}

/// The glyph itself is in every bundled typewriter face, at U+2423 OPEN
/// BOX. The CFF charset names it `uni2423`, not `visiblespace`, which is
/// why an earlier reading of this bundle concluded the face carried no
/// such glyph and `verbatim*` could not be set without adding a font.
#[test]
fn every_bundled_typewriter_face_carries_the_visible_space_glyph() {
    let Some(fonts) = bundled_fonts() else { return };
    for file in [
        "lmmono8-regular.otf",
        "lmmono9-regular.otf",
        "lmmono10-regular.otf",
        "lmmono12-regular.otf",
        "lmmono10-italic.otf",
        "lmmonoslant10-regular.otf",
        "lmmonocaps10-regular.otf",
        "lmmonolt10-bold.otf",
    ] {
        let face = fonts.otf(file).unwrap_or_else(|e| panic!("{file}: {e}"));
        let gid = face.face().glyph_id('\u{2423}').unwrap_or_else(|| panic!("{file} has no U+2423"));
        let bounds = face.bounds(gid, Some('\u{2423}'));
        assert!(!bounds.empty && bounds.x_max > 0, "{file}: U+2423 is an empty outline");
    }
}
