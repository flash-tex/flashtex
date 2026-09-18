//! GH-48: the searchable text of an exported PDF, read back through Poppler's
//! three extraction modes (`pdftotext`, `pdftotext -layout`, `pdftotext -raw`).
//!
//! Issue #48 reported that `$\text{a b}$` extracts as `ab` in Poppler's default
//! and `-layout` modes while `-raw` gives `a b`, and asked whether the exporter
//! drops the word break. It does not. Measured on poppler 26.09.0 against
//! pdfTeX 3.141592653 (TeX Live), the *reference implementation produces the
//! same three answers from its own PDF*:
//!
//! ```text
//! source                     producer   default   -layout   -raw
//! $\text{a b}$               pdflatex   'ab'      'ab'      'a b'
//! $\text{a b}$               flashtex   'ab'      'ab'      'a b'
//! hello world foo            pdflatex   ok        ok        ok
//! hello world foo            flashtex   ok        ok        ok
//! ```
//!
//! pdfTeX writes the gap as a `TJ` kern inside one text object
//! (`[(a)-333(b)]TJ`); this exporter writes the two runs at absolute `Tm`
//! origins. Both carry the same 0.333 em gap and Poppler answers both the same
//! way, because its default and `-layout` modes rebuild words from *geometry*
//! and ignore whatever the string encodes: a PDF whose string literally
//! contains U+0020 (`[<612062>]TJ`, "a b") also extracts as `ab`. The merge is
//! a per-line Poppler heuristic whose gap threshold is degenerate when every
//! word on the line is a single character -- measured with `\hspace`:
//!
//! ```text
//! word length   0.25em  0.30em  0.333em  0.36em  0.40em  0.45em  0.50em
//! 1 char        merge   merge   merge    merge   merge   SPACE   SPACE
//! 2+ chars      SPACE   SPACE   SPACE    SPACE   SPACE   SPACE   SPACE
//! ```
//!
//! So the only producer-side lever is glyph geometry (forbidden: it would break
//! pdflatex fidelity) or `/ActualText` spans around every interword gap (a
//! format-policy decision that diverges from pdfTeX's bytes, and is deliberately
//! left to the PDF/Text owners in #48).
//!
//! What these tests pin is therefore the thing that *is* ours: the word break
//! must survive into the content stream (`-raw` proves it), ordinary prose must
//! extract correctly in all three modes, and the single-letter case must keep
//! answering exactly what pdflatex answers rather than drifting away from it.
//! That last check runs against a live pdfTeX oracle when `pdflatex` is on
//! PATH (the executed comparison, not a hand-copied literal); without pdfTeX
//! it falls back to the documented literal, guarded to the Poppler release
//! the literal was measured on.

mod common;

use common::lm_available;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{BP_PER_TEX_PT, TICKS_PER_BP};
use flashtex_render_pipeline::{FontSet, RenderOptions, Rendered};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{amsmath}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn render_with(fonts: &FontSet, body: &str) -> Rendered {
    let text = doc(body);
    let sources = [SourceDocument { path: "main.tex", text: &text }];
    flashtex_render_pipeline::render(&sources, "main.tex", 7, "gh48", fonts, &RenderOptions::default())
}

/// The exact-route PDF bytes for `body`, the same route `flashtex build` uses.
fn exact_pdf(body: &str) -> Vec<u8> {
    let fonts = FontSet::with_default_dirs(&[]);
    let rendered = render_with(&fonts, body);
    flashtex_render_pipeline::pdf::write_pdf_exact(&rendered.v2, fonts.dirs(), None)
        .expect("exact PDF")
        .bytes
}

/// Mirror of `common::lm_available`: Poppler is a load-bearing dependency of
/// these tests, so a missing `pdftotext` is a loud failure by default.
///
/// The old helper unconditionally reported "skip" (and its `eprintln!` is
/// captured by libtest, so a Poppler-less run printed nothing and reported
/// **3 passed** having executed zero extraction assertions). A genuinely
/// popplerless environment can still skip, but only by asking for it:
/// `FLASHTEX_ALLOW_POPPLERLESS_TESTS=1`, which restores the old `false`.
fn poppler_available() -> bool {
    if std::process::Command::new("pdftotext").arg("-v").output().is_ok() {
        return true;
    }
    if std::env::var_os("FLASHTEX_ALLOW_POPPLERLESS_TESTS").is_some() {
        eprintln!("SKIPPED: pdftotext (poppler) is not on PATH");
        return false;
    }
    panic!(
        "pdftotext (poppler) is not on PATH, so this test would have skipped silently \
         and the run would have been green without measuring anything. Install \
         poppler-utils (apt) or poppler (brew), or set \
         FLASHTEX_ALLOW_POPPLERLESS_TESTS=1 to skip deliberately."
    );
}

/// The Poppler release the fallback literals below were measured on. Poppler's
/// single-character-word merge is a heuristic, not a contract: if a later
/// release changes it, the `ab` literals fail on a commit where our output is
/// unchanged, so the fallback only runs against this release (see
/// [`poppler_is_measured`]).
const MEASURED_POPPLER: &str = "26.09.0";

/// `pdftotext -v` reports `pdftotext version <v>` (on stderr) plus library
/// copyright lines. Returns the `<v>` token, or an empty string when the
/// output does not have the expected shape.
fn pdftotext_version() -> String {
    let out = std::process::Command::new("pdftotext")
        .arg("-v")
        .output()
        .expect("poppler_available() passed, so pdftotext runs");
    let text = format!(
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut words = text.split_whitespace();
    while let Some(w) = words.next() {
        if w == "version" {
            return words.next().unwrap_or("").trim_end_matches(',').to_string();
        }
    }
    String::new()
}

/// Whether the installed Poppler is the release the fallback literals were
/// measured on. A different release may answer the single-letter case
/// differently (see [`MEASURED_POPPLER`]), so the literal fallback must not
/// run there; the caller skips loudly instead.
fn poppler_is_measured() -> bool {
    pdftotext_version() == MEASURED_POPPLER
}

/// A per-extraction scratch directory, removed when the extraction ends,
/// panic or not. (This suite used to share one `flashtex-gh48-{pid}` dir and
/// remove only the `.pdf`, leaking one empty directory per test run.)
struct ScratchDir {
    path: PathBuf,
}

static SCRATCH_SEQ: AtomicU64 = AtomicU64::new(0);

impl ScratchDir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "flashtex-gh48-{tag}-{}-{}",
            std::process::id(),
            SCRATCH_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        ScratchDir { path: dir }
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// The first non-blank line `pdftotext MODE` reads out of the PDF at `pdf`.
/// Poppler's presence was already probed by [`poppler_available`], so a spawn
/// failure here is a real error, not a skip.
fn extract_line(pdf: &Path, mode: &str) -> String {
    let mut cmd = std::process::Command::new("pdftotext");
    if !mode.is_empty() {
        cmd.arg(mode);
    }
    let out = cmd.arg(pdf).arg("-").output().expect("pdftotext runs (see poppler_available)");
    assert!(out.status.success(), "pdftotext {mode}: {}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string()
}

/// All three Poppler modes over `pdf`, read back from a scratch file that is
/// removed before this returns.
fn three_modes_of_pdf(pdf: &[u8], tag: &str) -> [String; 3] {
    let dir = ScratchDir::new(tag);
    let path = dir.path.join(format!("{tag}.pdf"));
    std::fs::write(&path, pdf).unwrap();
    ["", "-layout", "-raw"].map(|mode| extract_line(&path, mode))
}

/// All three modes over the exact-route PDF for `body`.
fn three_modes(body: &str, tag: &str) -> [String; 3] {
    three_modes_of_pdf(&exact_pdf(body), tag)
}

/// The same `body` rendered through the real pdfTeX and read back in all three
/// Poppler modes: the executed oracle. `None` when `pdflatex` is not on PATH
/// (CI has no TeX installation); the caller then falls back to the documented
/// literal. Any other spawn failure, or a failing pdfTeX run, panics.
fn pdflatex_three_modes(body: &str, tag: &str) -> Option<[String; 3]> {
    let dir = ScratchDir::new(tag);
    let tex = dir.path.join(format!("{tag}-oracle.tex"));
    std::fs::write(&tex, doc(body)).unwrap();
    let out = match std::process::Command::new("pdflatex")
        .args(["-interaction=nonstopmode", "-halt-on-error", "-output-directory"])
        .arg(&dir.path)
        .arg(&tex)
        .output()
    {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => panic!("pdflatex: {e}"),
    };
    assert!(
        out.status.success(),
        "pdflatex {tag}: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let pdf = dir.path.join(format!("{tag}-oracle.pdf"));
    Some(["", "-layout", "-raw"].map(|mode| extract_line(&pdf, mode)))
}

/// The case a reader actually copies out of an exported document: multi-letter
/// words must keep their spaces in *every* mode. This is the assertion that
/// protects users, and it is green today.
#[test]
fn ordinary_words_keep_their_spaces_in_every_poppler_mode() {
    if !lm_available() {
        return;
    }
    if !poppler_available() {
        return;
    }
    let [default, layout, raw] = three_modes("hello world foo", "words");
    assert_eq!(default, "hello world foo", "pdftotext default mode");
    assert_eq!(layout, "hello world foo", "pdftotext -layout");
    assert_eq!(raw, "hello world foo", "pdftotext -raw");
}

/// GH-48's own reproduction. `-raw` is the load-bearing assertion: it proves the
/// exporter really did put a word break between the two runs, so a future change
/// that silently welds `a` and `b` into one run fails here.
///
/// The default and `-layout` expectations are the *pdflatex oracle* for the same
/// source. With pdfTeX on PATH the comparison is executed: whatever pdfTeX's
/// own PDF extracts as, ours must answer identically. Without pdfTeX the test
/// falls back to the documented literal (`ab`, measured on poppler 26.09.0),
/// and refuses to run that literal against any other Poppler release.
#[test]
fn single_letter_words_extract_exactly_as_pdflatex_does() {
    if !lm_available() {
        return;
    }
    if !poppler_available() {
        return;
    }
    let [default, layout, raw] = three_modes("$\\text{a b}$", "ab");
    assert_eq!(raw, "a b", "the word break must reach the content stream");
    match pdflatex_three_modes("$\\text{a b}$", "ab") {
        Some(oracle) => {
            assert_eq!(default, oracle[0], "default extraction must equal pdflatex's");
            assert_eq!(layout, oracle[1], "-layout extraction must equal pdflatex's");
            assert_eq!(raw, oracle[2], "-raw extraction must equal pdflatex's");
            eprintln!("pdflatex oracle (executed): {oracle:?}");
        }
        None => {
            eprintln!(
                "pdflatex is not on PATH: checking against the documented literal oracle \
                 (poppler {MEASURED_POPPLER}); rerun with pdflatex on PATH for the executed comparison"
            );
            if !poppler_is_measured() {
                eprintln!(
                    "SKIPPED: installed poppler is {} but the literal oracle was measured on \
                     poppler {MEASURED_POPPLER}, which may merge single-character words differently",
                    pdftotext_version()
                );
                return;
            }
            assert_eq!(default, "ab", "pdflatex oracle for the same source is 'ab'");
            assert_eq!(layout, "ab", "pdflatex oracle for the same source is 'ab'");
        }
    }
}

/// The gap is carried as geometry, not as a glyph. Nobody may "fix" the
/// extraction by painting a space character: pdfTeX paints none (it writes a
/// bare `-333` inside `[(a)-333(b)]TJ`), the page raster has to stay identical,
/// and a measured control shows it would not even work — a PDF whose shown
/// string literally contains U+0020 still extracts as `ab` in Poppler's default
/// and `-layout` modes.
#[test]
fn the_interword_gap_is_geometry_and_paints_no_glyph() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let glyphs = |body: &str| -> Vec<(u16, i64)> {
        use flashtex_render_pipeline::display::Item;
        render_with(&fonts, body).v2.pages[0]
            .resident_items()
            .iter()
            .filter_map(|i| match i {
                Item::GlyphRun(r) => Some(r),
                _ => None,
            })
            .flat_map(|r| r.glyphs.iter().map(|g| (g.gid, g.origin_x.0)))
            .collect()
    };
    let welded = glyphs("$\\text{ab}$");
    let spaced = glyphs("$\\text{a b}$");
    assert_eq!(
        welded.len(),
        spaced.len(),
        "`a b` painted {} glyphs and `ab` painted {}; the interword gap must add no glyph",
        spaced.len(),
        welded.len()
    );
    assert_eq!(welded[0], spaced[0], "the first glyph must not move");
    // The gap must be the interword space itself, not just any positive nudge:
    // cmr10's fontdimen2 is 1/3 em, i.e. 10/3 TeX pt at 10pt (measured
    // 3482191 ticks = 3.32089bp). The 0.01bp tolerance is ~100x the observed
    // f64-layout rounding dust (~75 ticks) and ~300x smaller than the value,
    // so welding the gap (0) or halving it both still fail.
    let expected_bp = 10.0 / 3.0 * BP_PER_TEX_PT;
    let gap_bp = (spaced[1].1 - welded[1].1) as f64 / TICKS_PER_BP;
    assert!(
        (gap_bp - expected_bp).abs() < 0.01,
        "the gap must be the interword space ({expected_bp:.5}bp), got {gap_bp:.5}bp"
    );
}
