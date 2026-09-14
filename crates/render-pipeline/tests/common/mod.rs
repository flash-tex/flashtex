#![allow(dead_code)]
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::v1::{self, Capabilities, V1Payload};
use flashtex_render_pipeline::{render, FontSet, RenderOptions, Rendered};

/// Whether Latin Modern resolves, for the ~110 `if !lm_available() { return }`
/// guards across this suite.
///
/// Those guards used to make a fontless run *silently green*: the tests did
/// not fail, they simply never executed, and `eprintln!` is captured by
/// libtest, so nothing was printed either. A whole-suite run with no
/// `FLASHTEX_*` set therefore reported success while measuring almost
/// nothing -- the same trap as an oracle harness scoring an OpenType
/// fallback as a pass.
///
/// So a missing Latin Modern is now a loud failure by default. A genuinely
/// fontless environment can still skip, but only by asking for it:
/// `FLASHTEX_ALLOW_FONTLESS_TESTS=1`, which restores the old `false`.
pub fn lm_available() -> bool {
    if FontSet::with_default_dirs(&[]).latin_modern_available() {
        return true;
    }
    if std::env::var_os("FLASHTEX_ALLOW_FONTLESS_TESTS").is_some() {
        return false;
    }
    panic!(
        "Latin Modern is not resolvable, so this test would have skipped silently \
         and the run would have been green without measuring anything. Point \
         FLASHTEX_FONT_DIRS at a directory of Latin Modern .otf files (the repo \
         bundles apps/mac/Fonts) and FLASHTEX_TFM_DIRS at its metrics, or set \
         FLASHTEX_ALLOW_FONTLESS_TESTS=1 to skip deliberately."
    );
}

pub fn render_docs(docs: &[(&str, &str)], entry: &str) -> Rendered {
    let fonts = FontSet::with_default_dirs(&[]);
    let sources: Vec<SourceDocument<'_>> = docs.iter().map(|(p, t)| SourceDocument { path: p, text: t }).collect();
    render(&sources, entry, 7, "test-project", &fonts, &RenderOptions::default())
}

pub fn render_one(text: &str) -> Rendered {
    render_docs(&[("main.tex", text)], "main.tex")
}

/// [`render_one`] against a caller-supplied font set, for the controls that
/// must not read `FLASHTEX_FONT_DIRS`.
pub fn render_one_with(text: &str, fonts: &FontSet) -> Rendered {
    let sources = [SourceDocument { path: "main.tex", text }];
    render(&sources, "main.tex", 7, "test-project", fonts, &RenderOptions::default())
}

pub fn v1_of(r: &Rendered, caps: Capabilities) -> V1Payload {
    let accepted = {
        let mut a = Vec::new();
        if caps.rules {
            a.push(v1::CAP_RULES.to_string());
        }
        if caps.font_hints {
            a.push(v1::CAP_FONT_HINTS.to_string());
        }
        Some(a)
    };
    v1::fallback(&r.v2, caps, accepted)
}

/// One v2 glyph run as a word box: page, text, left edge, baseline and
/// width in bp (a math formula appears as one run per glyph).
#[derive(Debug, Clone)]
pub struct Word {
    pub page: u32,
    pub text: String,
    pub x: f64,
    pub baseline: f64,
    pub width: f64,
}

pub fn words_of(r: &Rendered) -> Vec<Word> {
    let mut words = Vec::new();
    for page in &r.v2.pages {
        for it in page.resident_items() {
            if let flashtex_render_pipeline::display::Item::GlyphRun(run) = it {
                let Some(first) = run.glyphs.first() else { continue };
                let last = run.glyphs.last().expect("non-empty");
                words.push(Word {
                    page: page.number,
                    text: run.text.clone(),
                    x: first.origin_x.to_bp(),
                    baseline: first.baseline_y.to_bp(),
                    width: (last.origin_x.0 + last.advance_x.0 - first.origin_x.0) as f64 / flashtex_render_pipeline::display::TICKS_PER_BP,
                });
            }
        }
    }
    words
}

/// A font set built from an explicit staged directory rather than from the
/// ambient `FLASHTEX_FONT_DIRS`, with every face named in `without` left out.
///
/// Several tests in this suite are *controls*: they assert that a face is
/// **absent** (no `NewCMMath-Regular`, no resolvable `.tfm`) and that the
/// pipeline then degrades the documented way. `FontSet::with_default_dirs`
/// and `FontSet::new` both read the environment, so the moment CI exports a
/// real font and metric environment the control stops being a control: the
/// very face it asserts is missing resolves, and the test fails for a reason
/// that has nothing to do with the engine. `NewCMMath-Regular.otf` ships in
/// `apps/mac/Fonts` beside the Latin Modern faces, so the contaminating face
/// cannot be excluded by dropping a whole directory.
///
/// This stages links to the real faces in a per-test temp directory, omitting
/// `without`, and pairs them with explicit `tfm_dirs`. The resulting set is
/// hermetic: identical with or without `FLASHTEX_*` exported.
pub fn stage_faces_without(tag: &str, without: &[&str]) -> ControlDir {
    let real = FontSet::with_default_dirs(&[]);
    let dir = std::env::temp_dir().join(format!("flashtex-control-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("stage a control font directory");
    for src in real.dirs() {
        let Ok(entries) = std::fs::read_dir(src) else { continue };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.ends_with(".otf") || without.contains(&name.as_str()) {
                continue;
            }
            let dst = dir.join(&name);
            if dst.exists() {
                continue;
            }
            #[cfg(unix)]
            let _ = std::os::unix::fs::symlink(e.path(), &dst);
            #[cfg(not(unix))]
            let _ = std::fs::copy(e.path(), &dst);
        }
    }
    ControlDir(dir)
}

/// The metric directories the ambient environment resolves, for a control set
/// that wants real metrics but a curated set of faces.
pub fn ambient_tfm_dirs() -> Vec<std::path::PathBuf> {
    FontSet::with_default_dirs(&[]).tfm_dirs().to_vec()
}

/// Removes a staged control directory when the test ends, panic or not.
pub struct ControlDir(std::path::PathBuf);

impl ControlDir {
    pub fn path(&self) -> &std::path::Path {
        &self.0
    }

    /// A hermetic font set over the staged faces, then `extra` (so a test can
    /// add back the one face it is measuring), with explicit metric dirs.
    pub fn font_set(&self, extra: &[std::path::PathBuf], tfm_dirs: Vec<std::path::PathBuf>) -> FontSet {
        let mut dirs = vec![self.0.clone()];
        dirs.extend_from_slice(extra);
        FontSet::with_dirs(dirs, tfm_dirs)
    }
}

impl Drop for ControlDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
