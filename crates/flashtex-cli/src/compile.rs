//! One compile of a project: the render pipeline in-process, diagnostics
//! unified with the closure's, the exact-route PDF and the v2 display list
//! as side outputs, and the `check --json` report.

use std::path::{Path, PathBuf};
use std::time::Instant;

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Severity;
use flashtex_render_pipeline::{pdf, FontSet, RenderOptions};

use crate::project::Project;

/// A diagnostic as the CLI prints it: `path:line:col: severity[code] message`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub path: String,
    /// 1-based line and column (in characters) of `start_byte`, when the
    /// diagnostic points into a document of the project.
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub start_byte: Option<usize>,
    pub end_byte: Option<usize>,
    pub error: bool,
    pub code: String,
    pub message: String,
    pub recovery: Option<String>,
}

impl Diagnostic {
    pub fn severity(&self) -> &'static str {
        if self.error {
            "error"
        } else {
            "warning"
        }
    }

    /// The one-line form on stderr. A diagnostic without a source position
    /// names the entry file (the compile as a whole).
    pub fn line_text(&self) -> String {
        let at = match (self.line, self.column) {
            (Some(l), Some(c)) => format!("{}:{l}:{c}: ", self.path),
            _ => format!("{}: ", self.path),
        };
        let tail = self.recovery.as_deref().map_or(String::new(), |r| format!(" (recovery: {r})"));
        format!("{at}{}[{}] {}{tail}", self.severity(), self.code, self.message)
    }
}

/// What a compile produced, before any file is written.
pub struct Outcome {
    /// `ok` (no diagnostics), `recovered` (diagnostics, output produced) or
    /// `failed` (errors and no content): the runtime-v1 status.
    pub status: &'static str,
    pub pages: usize,
    pub diagnostics: Vec<Diagnostic>,
    pub rendered: flashtex_render_pipeline::Rendered,
    /// Wall time of the render (parse + layout + display list).
    pub render_ms: f64,
    pub passes: u32,
}

impl Outcome {
    pub fn errors(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.error).count()
    }
    pub fn warnings(&self) -> usize {
        self.diagnostics.len() - self.errors()
    }
}

/// Renders the project. `revision` is echoed into the display list (`watch`
/// counts rebuilds).
pub fn compile(project: &Project, fonts: &FontSet, options: &RenderOptions, revision: u64) -> Outcome {
    let sources: Vec<SourceDocument<'_>> = project
        .documents
        .iter()
        .map(|d| SourceDocument { path: d.path.as_str(), text: d.text.as_str() })
        .collect();
    let project_id = Path::new(&project.entry).file_stem().and_then(|s| s.to_str()).unwrap_or("main");
    let opts = RenderOptions { project_root: Some(project.root.clone()), ..options.clone() };
    let started = Instant::now();
    let rendered = flashtex_render_pipeline::render(&sources, &project.entry, revision, project_id, fonts, &opts);
    let render_ms = started.elapsed().as_secs_f64() * 1000.0;
    let text_of = |path: &str| project.documents.iter().find(|d| d.path == path).map(|d| d.text.as_str());
    let mut diagnostics: Vec<Diagnostic> = project
        .diagnostics
        .iter()
        .map(|d| {
            let at = d.start_byte.and_then(|b| text_of(&d.path).map(|t| line_col(t, b)));
            Diagnostic {
                path: d.path.clone(),
                line: at.map(|a| a.0),
                column: at.map(|a| a.1),
                start_byte: d.start_byte,
                end_byte: d.end_byte,
                error: d.error,
                code: d.code.to_string(),
                message: d.message.clone(),
                recovery: None,
            }
        })
        .collect();
    for d in &rendered.v2.diagnostics {
        let source = d.sources.first();
        let path = source.map_or(project.entry.as_str(), |s| rendered.v2.document_paths.path(s.document));
        let at = source.and_then(|s| text_of(path).map(|t| line_col(t, s.start())));
        diagnostics.push(Diagnostic {
            path: path.to_string(),
            line: at.map(|a| a.0),
            column: at.map(|a| a.1),
            start_byte: source.map(|s| s.start()),
            end_byte: source.map(|s| s.end()),
            error: d.severity == Severity::Error,
            code: d.code.clone(),
            message: d.message.clone(),
            recovery: d.recovery.clone(),
        });
    }
    // The runtime-v1 status rule (render-pipeline `v1::fallback`): a
    // project-closure error is an error diagnostic like the compiler's.
    let has_content = rendered.v2.pages.iter().any(|p| !p.items.is_empty());
    let has_error = diagnostics.iter().any(|d| d.error);
    let status = if diagnostics.is_empty() {
        "ok"
    } else if has_content || !has_error {
        "recovered"
    } else {
        "failed"
    };
    Outcome {
        status,
        pages: rendered.v2.pages.len(),
        diagnostics,
        passes: rendered.passes,
        rendered,
        render_ms,
    }
}

/// 1-based line and column (in characters) of a byte offset; an offset past
/// the end or inside a multi-byte sequence is clamped to the nearest boundary.
pub fn line_col(text: &str, byte: usize) -> (usize, usize) {
    let mut b = byte.min(text.len());
    while b > 0 && !text.is_char_boundary(b) {
        b -= 1;
    }
    let before = &text[..b];
    let line = before.matches('\n').count() + 1;
    let col = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
    (line, col)
}

/// Writes `bytes` to `path` through a sibling temporary file and a rename,
/// so a reader (a PDF viewer, `watch`'s Ctrl-C) never sees a torn file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("out");
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    std::fs::write(&tmp, bytes).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("cannot write {}: {e}", path.display())
    })
}

/// The exact-route PDF (`pdf::write_pdf_exact`): real embedded font
/// subsets, images, links. Returns the bytes and the route's notes.
pub fn exact_pdf(outcome: &Outcome, fonts: &FontSet, project_root: &Path) -> Result<pdf::ExactPdfOut, String> {
    pdf::write_pdf_exact(&outcome.rendered.v2, fonts.dirs(), Some(project_root))
}

/// The `display_list` envelope (`display-list-v2`, images included) for
/// `--v2`, byte-identical to what the worker sends the IDE.
pub fn v2_json(outcome: &Outcome, id: &str) -> String {
    outcome.rendered.v2.write_json_with(id, true)
}

/// The `check --json` / `build --json` report. Schema `flashtex-check/1`:
/// the keys below are stable; new keys may be added.
pub fn report_json(project: &Project, outcome: &Outcome, outputs: &[(&str, &Path)], total_ms: f64) -> String {
    let mut o = Value::obj();
    o.set("schema", json::str_("flashtex-check/1"));
    o.set("entry", json::str_(project.entry.clone()));
    o.set("project_root", json::str_(project.root.display().to_string()));
    o.set("status", json::str_(outcome.status));
    o.set("pages", json::num(outcome.pages as f64));
    o.set("documents", Value::Arr(project.documents.iter().map(|d| json::str_(d.path.clone())).collect()));
    o.set(
        "diagnostics",
        Value::Arr(
            outcome
                .diagnostics
                .iter()
                .map(|d| {
                    let mut v = Value::obj();
                    v.set("path", json::str_(d.path.clone()));
                    v.set("line", opt_num(d.line));
                    v.set("column", opt_num(d.column));
                    v.set("start_byte", opt_num(d.start_byte));
                    v.set("end_byte", opt_num(d.end_byte));
                    v.set("severity", json::str_(d.severity()));
                    v.set("code", json::str_(d.code.clone()));
                    v.set("message", json::str_(d.message.clone()));
                    v.set("recovery", d.recovery.clone().map_or(Value::Null, json::str_));
                    v
                })
                .collect(),
        ),
    );
    let mut summary = Value::obj();
    summary.set("errors", json::num(outcome.errors() as f64));
    summary.set("warnings", json::num(outcome.warnings() as f64));
    o.set("summary", summary);
    let mut timing = Value::obj();
    timing.set("render_ms", json::num(round2(outcome.render_ms)));
    timing.set("total_ms", json::num(round2(total_ms)));
    timing.set("passes", json::num(outcome.passes as f64));
    o.set("timing", timing);
    let mut outs = Value::obj();
    for (k, p) in outputs {
        outs.set(k, json::str_(p.display().to_string()));
    }
    o.set("outputs", outs);
    o.set("version", json::str_(crate::version_string()));
    json::write(&o)
}

fn opt_num(n: Option<usize>) -> Value {
    n.map_or(Value::Null, |n| json::num(n as f64))
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// `-o` default: the entry's stem with `.pdf`, next to the entry file.
pub fn default_pdf_path(main: &Path) -> PathBuf {
    main.with_extension("pdf")
}
