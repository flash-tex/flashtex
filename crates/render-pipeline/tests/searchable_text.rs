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

mod common;

use common::lm_available;
use flashtex_render_pipeline::{FontSet, RenderOptions, Rendered};
use flashtex_compiler::parser::SourceDocument;

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

/// The first non-blank line `pdftotext MODE` reads out of `pdf`, or `None` when
/// Poppler is not installed. `mode` is the extra flag (`""` for the default).
fn pdftotext(pdf: &[u8], tag: &str, mode: &str) -> Option<String> {
    let dir = std::env::temp_dir().join(format!("flashtex-gh48-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{tag}.pdf"));
    std::fs::write(&path, pdf).unwrap();

    let mut cmd = std::process::Command::new("pdftotext");
    if !mode.is_empty() {
        cmd.arg(mode);
    }
    let out = match cmd.arg(&path).arg("-").output() {
        Ok(o) => o,
        // No Poppler on this machine: the caller reports the skip loudly.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => panic!("pdftotext: {e}"),
    };
    let _ = std::fs::remove_file(&path);
    assert!(out.status.success(), "pdftotext {mode}: {}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    Some(text.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string())
}

/// All three modes, or `None` when Poppler is missing.
fn three_modes(body: &str, tag: &str) -> Option<[String; 3]> {
    let pdf = exact_pdf(body);
    let default = pdftotext(&pdf, tag, "")?;
    let layout = pdftotext(&pdf, tag, "-layout")?;
    let raw = pdftotext(&pdf, tag, "-raw")?;
    Some([default, layout, raw])
}

fn poppler_missing() -> bool {
    eprintln!("SKIPPED: pdftotext (poppler) is not on PATH");
    true
}

/// The case a reader actually copies out of an exported document: multi-letter
/// words must keep their spaces in *every* mode. This is the assertion that
/// protects users, and it is green today.
#[test]
fn ordinary_words_keep_their_spaces_in_every_poppler_mode() {
    if !lm_available() {
        return;
    }
    let Some([default, layout, raw]) = three_modes("hello world foo", "words") else {
        assert!(poppler_missing());
        return;
    };
    assert_eq!(default, "hello world foo", "pdftotext default mode");
    assert_eq!(layout, "hello world foo", "pdftotext -layout");
    assert_eq!(raw, "hello world foo", "pdftotext -raw");
}

/// GH-48's own reproduction. `-raw` is the load-bearing assertion: it proves the
/// exporter really did put a word break between the two runs, so a future change
/// that silently welds `a` and `b` into one run fails here.
///
/// The default and `-layout` expectations are the *pdflatex oracle* for the same
/// source (poppler 26.09.0, measured 2026-09-16): pdfTeX's own PDF extracts as
/// `ab` too. They are pinned so that a divergence from the reference
/// implementation -- in either direction -- is caught rather than assumed.
#[test]
fn single_letter_words_extract_exactly_as_pdflatex_does() {
    if !lm_available() {
        return;
    }
    let Some([default, layout, raw]) = three_modes("$\\text{a b}$", "ab") else {
        assert!(poppler_missing());
        return;
    };
    assert_eq!(raw, "a b", "the word break must reach the content stream");
    assert_eq!(default, "ab", "pdflatex oracle for the same source is 'ab'");
    assert_eq!(layout, "ab", "pdflatex oracle for the same source is 'ab'");
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
    assert!(
        spaced[1].1 > welded[1].1,
        "the gap must be carried by the second glyph's origin ({} vs {})",
        spaced[1].1,
        welded[1].1
    );
}
