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
const GATED: [&str; 15] = [
    "01-verbatim-basic",
    "02-verbatim-ligatures",
    "03-verbatim-tabs",
    "04-verbatim-blank-lines",
    "05-verbatim-star",
    "06-verb-delimiters",
    "07-verbatim-t1",
    "08-verbatim-lmodern",
    "09-verbatim-vmode",
    "10-verbatim-itemize",
    "11-verbatim-11pt",
    "12-verbatim-12pt",
    "13-verbatim-long-line",
    "14-verbatim-pagebreak",
    "32-verbatim-microtype",
];

/// Committed with their references but not gated yet, each for a reason
/// that names what is still missing. Listed here so the material is in the
/// tree and the follow-up is visible rather than forgotten.
const NOT_YET: [(&str, &str); 2] = [
    ("15-verbatim-small",
     "The `\\small` in force. Verified by swapping PR #261's compiler into `vendor/compiler` \
      (uncommitted, restored after): with it the horizontal geometry is exact -- the body sets in \
      CMTT9 and the line's last glyph lands at 194.964 against pdfTeX's 194.964 -- and the whole \
      block is left exactly 1.0 pt low. `\\showoutput` says why: pdfTeX's skip above the block is \
      `\\glue 10.0 plus 4.0 minus 5.0` (`\\topsep` 8+2-4, `\\partopsep` 2+1-1, `\\parskip` 0+1) and \
      its `\\glue(\\baselineskip) 3.55557` is `\\small`'s 11 pt baselineskip minus the previous \
      depth 1.94444 and the line's height 5.49998. The pipeline sets the block at the body's 12 pt \
      baselineskip, which is the entire remaining pound. Needs the re-pin, the pipeline's \
      conversion arms for the new `style` field, and the block's baselineskip to follow its size."),
    ("28-lstinline",
     "The compiler typesets `\\lstset`'s argument as prose: 126 glyphs against pdfTeX's 115, the \
      extra 11 being the characters `basicstyle=` at the head of the first line. `\\lstinline` \
      itself is lexed correctly (PR #188, already in the vendored mirror). Fixed compiler-side in \
      PR #261: verified by swapping that compiler into `vendor/compiler` (uncommitted, restored \
      after), where this fixture matches at 115 glyphs, worst 0.005 bp. Needs the re-pin plus the \
      four pipeline arms for the new `style` field."),
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
    let mut known: Vec<String> = GATED.iter().map(|s| s.to_string()).chain(NOT_YET.iter().map(|(n, _)| n.to_string())).collect();
    known.sort();
    assert_eq!(on_disk, known, "every fixture must be gated or listed in NOT_YET with a reason");
    for name in &on_disk {
        assert!(
            dir.join("reference").join(format!("{name}.json")).is_file(),
            "{name} has no committed pdfLaTeX reference"
        );
    }
    for (_, why) in NOT_YET {
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
