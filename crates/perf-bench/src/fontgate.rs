//! What decides whether a timing number is allowed to exist.
//!
//! A run whose Latin Modern metrics did not load is not a slower run of the
//! same workload — it is a different workload: different faces, different
//! advances, different line breaks, a different page count. Such a run must
//! not produce a number at all, so [`METRIC_CODES`] refuses the case outright.
//!
//! The line is drawn at *the configuration*, not at *the document*. A
//! character with no glyph in Latin Modern, or a class command the compiler
//! does not model yet, changes what is on the page but is identical on every
//! run — the timing is stable and comparable, so the case is measured and the
//! degradation is recorded next to it ([`CONTENT_CODES`], [`degradations`]).
//! Refusing those too would have left six of the ten real-world fixtures
//! unmeasured for saying, in effect, that Latin Modern has no CJK.
//!
//! The codes come from `crates/render-pipeline/src/typeset.rs`; they are
//! emitted through `Context::report_once`, so one occurrence is one distinct
//! problem.

use std::path::PathBuf;

use flashtex_render_pipeline::display::{Diagnostic, Severity};
use flashtex_render_pipeline::FontSet;

/// An exact font configuration: no host probing, no bundle-relative guessing,
/// no fallback to whatever TeX Live the machine happens to have.
///
/// `FontSet::with_default_dirs` appends a dozen host directories and several
/// executable-relative ones. On a machine with a system TeX installation that
/// is a silent invitation to measure someone else's metrics, and a run that
/// substitutes metrics is not a slower run of the same document. The harness
/// therefore resolves one explicit pair of lists and builds the set with
/// `FontSet::with_dirs`, which probes nothing.
pub struct FontConfig {
    pub font_dirs: Vec<PathBuf>,
    pub tfm_dirs: Vec<PathBuf>,
    /// Where the configuration came from, for the report.
    pub source: String,
}

/// The metric directories that live under a font tree's `texmf`, in the order
/// `fonts.rs` wants them: Latin Modern first, then EC, then the AMS symbol and
/// Euler sets.
const TEXMF_METRIC_DIRS: &[&str] = &[
    "texmf/fonts/tfm/public/lm",
    "texmf/fonts/tfm/jknappen/ec",
    "texmf/fonts/tfm/public/amsfonts/symbols",
    "texmf/fonts/tfm/public/amsfonts/euler",
];

/// Resolves the font configuration. Explicit flags win, then the environment,
/// then the repository's own `apps/mac/Fonts` — so the harness is correct by
/// default and never silently falls through to a host TeX tree.
///
/// When only a font directory is known, the metric directories are derived by
/// walking its `texmf`, which is why `--fonts` alone is enough here.
pub fn resolve(repo: &std::path::Path, fonts_flag: Option<&str>, tfm_flag: Option<&str>) -> FontConfig {
    let split = |s: &str| -> Vec<PathBuf> { s.split(':').filter(|p| !p.is_empty()).map(PathBuf::from).collect() };
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());

    let (font_dirs, source) = match (fonts_flag, env("FLASHTEX_FONT_DIRS")) {
        (Some(f), _) => (split(f), "--fonts".to_string()),
        (None, Some(v)) => (split(&v), "FLASHTEX_FONT_DIRS".to_string()),
        (None, None) => (vec![repo.join("apps/mac/Fonts")], "repository default (apps/mac/Fonts)".to_string()),
    };

    let tfm_dirs = match (tfm_flag, env("FLASHTEX_TFM_DIRS")) {
        (Some(t), _) => split(t),
        (None, Some(v)) => split(&v),
        (None, None) => font_dirs.iter().flat_map(|d| TEXMF_METRIC_DIRS.iter().map(move |m| d.join(m))).filter(|p| p.is_dir()).collect(),
    };

    FontConfig { font_dirs, tfm_dirs, source }
}

impl FontConfig {
    pub fn build(&self) -> FontSet {
        FontSet::with_dirs(self.font_dirs.clone(), self.tfm_dirs.clone())
    }

    fn join(dirs: &[PathBuf]) -> String {
        dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(":")
    }

    pub fn font_dirs_arg(&self) -> String {
        FontConfig::join(&self.font_dirs)
    }

    pub fn tfm_dirs_arg(&self) -> String {
        FontConfig::join(&self.tfm_dirs)
    }
}

/// Font diagnostics that mean the glyphs or the metrics were not the ones the
/// document asked for. Any of these makes the run a different workload —
/// different advances, different line breaks, a different page count — so a
/// timing taken next to one describes a document we did not mean to measure.
/// Fatal.
///
/// `lmmono`/`ectt` is the common one on `main` today: the typewriter faces and
/// metrics are not in `apps/mac/Fonts`, so every fixture using `\texttt`
/// refuses to be measured until they are added.
pub const METRIC_CODES: &[&str] = &[
    "required_metrics_unavailable",
    "font_unavailable",
    "math_font_unavailable",
    "math_metrics_opentype",
    "tfm_missing",
    "ec_metrics_unavailable",
    "font_shape_substituted",
    "font_outline_substituted",
    "tfm_run_error",
];

/// Font diagnostics that are a property of the document rather than of the
/// configuration: a character with no glyph in any available face, or a script
/// the shaper does not cover. They are deterministic and identical run to run,
/// so the timing is stable and comparable. Recorded on the case, not fatal —
/// refusing them would throw away the `unicode-accents` fixture for saying, in
/// effect, that Latin Modern has no CJK.
pub const CONTENT_CODES: &[&str] = &["missing_glyph", "unsupported_script"];

/// Checks the font set itself, before any document is rendered. `Err` is a
/// human-readable reason the run must stop.
pub fn preflight(fonts: &FontSet) -> Result<(), String> {
    fonts
        .required_metrics()
        .map_err(|e| format!("pinned Latin Modern 12 pt metrics did not load: {e}\n  FLASHTEX_TFM_DIRS must name a directory ending in fonts/tfm/public/lm (or a flat directory holding the four TFMs and the GUST licence)"))?;
    if !fonts.latin_modern_available() {
        return Err(format!(
            "Latin Modern faces missing; the pipeline would substitute Times.\n  font dirs searched: {:?}",
            fonts.dirs()
        ));
    }
    let failures = fonts.failures();
    if !failures.is_empty() {
        return Err(format!("font files failed to load: {failures:?}"));
    }
    Ok(())
}

/// Checks one render's diagnostics for anything that invalidates a
/// measurement. `Err` lists what made the output a different document.
pub fn check_diagnostics(diagnostics: &[Diagnostic]) -> Result<(), String> {
    let bad: Vec<String> = diagnostics
        .iter()
        .filter(|d| METRIC_CODES.contains(&d.code.as_str()))
        .map(|d| format!("{} [{}]: {}", d.code, severity(d.severity), d.message))
        .collect();
    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!("font diagnostics in a benchmarked render:\n  {}", bad.join("\n  ")))
    }
}

/// What the case should be labelled with: diagnostics that change what is on
/// the page without changing whether the timing is meaningful. Reported so
/// nobody reads a degraded case as a complete one.
pub fn degradations(diagnostics: &[Diagnostic]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for d in diagnostics {
        let kind = if CONTENT_CODES.contains(&d.code.as_str()) {
            "content"
        } else if d.severity == Severity::Error {
            // Almost always "\\foo is not supported": the compiler does not
            // model the command, so some content is missing. Deterministic and
            // stable, so the case is still worth measuring — but it is not the
            // whole document, and the report has to say so.
            "unsupported"
        } else {
            continue;
        };
        let line = format!("{kind}: {}: {}", d.code, d.message);
        if !out.contains(&line) {
            out.push(line);
        }
    }
    out
}

/// Whether this render produced any error diagnostic.
///
/// Used to validate a *keystroke*, not a document: the unedited text and the
/// edited text must produce the same result, or the scenario is measuring
/// error recovery rather than an edit. It is deliberately not a corpus gate —
/// a fixture whose class the compiler does not fully model still has a stable,
/// comparable timing.
pub fn check_no_errors(diagnostics: &[Diagnostic]) -> Result<(), String> {
    let errs: Vec<String> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| format!("{}: {}", d.code, d.message))
        .collect();
    if errs.is_empty() {
        Ok(())
    } else {
        Err(format!("error diagnostics in a benchmarked render:\n  {}", errs.join("\n  ")))
    }
}

fn severity(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

/// What the report records about the font configuration, so two runs can be
/// shown to have measured the same thing.
///
/// The resolved search lists run to dozens of paths, most of them probes that
/// do not exist; printing them buries the report. What decides the outcome is
/// the three override variables plus which directories actually contributed,
/// so those are what is recorded, with a digest over the full resolved list so
/// a silent change in discovery order still shows up.
pub fn describe(cfg: &FontConfig, fonts: &FontSet) -> Vec<(String, String)> {
    let all: Vec<String> = fonts.dirs().iter().chain(fonts.tfm_dirs()).map(|d| d.display().to_string()).collect();
    let digest: String = flashtex_font_engine::sha256::digest(all.join("\n").as_bytes()).iter().take(8).map(|b| format!("{b:02x}")).collect();
    vec![
        ("source".into(), cfg.source.clone()),
        ("font_dirs".into(), cfg.font_dirs_arg()),
        ("tfm_dirs".into(), cfg.tfm_dirs_arg()),
        ("search_digest".into(), digest),
        ("required_metrics".into(), fonts.required_metrics().map_or_else(|e| e, |_| "loaded".into())),
    ]
}
