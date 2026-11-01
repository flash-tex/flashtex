//! What each project `.sty`/`.cls` file defined, on the wire.
//!
//! The expansion engine records, for every package or class file it reads,
//! the definitions made at the file's outermost level with the byte span of
//! each defining statement, the file's `\ProvidesPackage` declaration, its
//! `\DeclareOption`s and the command that loaded it
//! (`flashtex_tex_expansion::OpenedFile`, `package_defs.rs` there). This
//! module maps those records into project documents ([`PackageRecord`],
//! built by `crate::expansion` as files are opened), hands them to the
//! parser's result ([`crate::parser::Parsed::package_definitions`]) and the
//! compile output, serialises them as the runtime-v1 `metadata.packages`
//! section ([`to_json`]; the schema is in `crate::protocol`), and labels a
//! diagnostic raised inside a package with the whole load chain
//! ([`label_load_chain`]).
//!
//! An editor uses the section for "declared in mystyle.sty" completion
//! rows, go-to-definition into a package file, and the hint at a
//! `\usepackage` line naming what the package brought in
//! (`apps/mac/docs/package-editing.md`).

use flashtex_tex_expansion::{self as tex, OpenedFile};

use crate::diagnostics::Diagnostic;
use crate::json::{str_, Value};
use crate::{DocumentId, Span};

/// One definition a package file made (see
/// `flashtex_tex_expansion::PackageDefinition` for the field semantics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDefinition {
    pub name: String,
    /// `macro`, `environment`, `conditional`, `counter`, `length`,
    /// `register`, `theorem` or `math_operator`.
    pub kind: &'static str,
    /// The defining command without its backslash (`newcommand`, `def`, ...).
    pub definer: String,
    pub arity: u8,
    pub optional_default: Option<String>,
    pub signature: String,
    /// The whole defining statement, in the package document.
    pub span: Span,
    pub overrides: bool,
    /// `\newtheorem` only.
    pub title: Option<String>,
    pub within: Option<String>,
}

/// `\ProvidesPackage{name}[date version description]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provides {
    pub name: String,
    pub date: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub span: Span,
}

/// One project package or class file the expansion pass read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRecord {
    /// The file's document.
    pub document: DocumentId,
    /// `package` for a `.sty`, `class` for a `.cls`.
    pub kind: &'static str,
    /// The `\usepackage`/`\RequirePackage`/`\documentclass`/`\LoadClass`
    /// that loaded it: in the entry document, or in another package file
    /// for a nested load.
    pub loaded_by: Span,
    pub provides: Option<Provides>,
    /// `\DeclareOption` names in declaration order (`*` for the default).
    pub options_declared: Vec<(String, Span)>,
    pub definitions: Vec<PackageDefinition>,
}

impl PackageRecord {
    /// The engine's record of `file`, whose tokens map to `document`, with
    /// every span mapped through `span` (a span the mapping rejects is
    /// dropped with its record; the loading command's falls back to
    /// `loaded_by`).
    pub fn from_opened(file: &OpenedFile, document: DocumentId, loaded_by: Span, path: &str, span: &dyn Fn(tex::Span) -> Option<Span>) -> Self {
        let kind = if path.ends_with(".cls") { "class" } else { "package" };
        let provides = file.provides.as_ref().and_then(|p| {
            Some(Provides {
                name: p.name.clone(),
                date: p.date.clone(),
                version: p.version.clone(),
                description: p.description.clone(),
                span: span(p.span)?,
            })
        });
        let options_declared = file.options.iter().filter_map(|o| Some((o.name.clone(), span(o.span)?))).collect();
        let definitions = file
            .definitions
            .iter()
            .filter_map(|d| {
                Some(PackageDefinition {
                    name: d.name.clone(),
                    kind: d.kind.as_str(),
                    definer: d.definer.clone(),
                    arity: d.arity,
                    optional_default: d.optional_default.clone(),
                    signature: d.signature.clone(),
                    span: span(d.span)?,
                    overrides: d.overrides,
                    title: d.title.clone(),
                    within: d.within.clone(),
                })
            })
            .collect();
        PackageRecord { document, kind, loaded_by, provides, options_declared, definitions }
    }
}

/// The runtime-v1 `metadata.packages` array (schema in `crate::protocol`).
/// `paths` are the project documents' paths, indexed by [`DocumentId`].
pub fn to_json(records: &[PackageRecord], paths: &[&str]) -> Value {
    let path_of = |document: DocumentId| str_(paths.get(document.0).copied().unwrap_or(""));
    let source = |span: Span| {
        let mut v = Value::obj();
        v.set("path", path_of(span.document));
        v.set("start", Value::Num(span.start as f64));
        v.set("end", Value::Num(span.end as f64));
        v
    };
    let optional = |text: &Option<String>| text.as_ref().map_or(Value::Null, |t| str_(t.clone()));
    Value::Arr(
        records
            .iter()
            .map(|record| {
                let mut v = Value::obj();
                v.set("path", path_of(record.document));
                v.set("kind", str_(record.kind));
                v.set(
                    "provides",
                    record.provides.as_ref().map_or(Value::Null, |p| {
                        let mut o = Value::obj();
                        o.set("name", str_(p.name.clone()));
                        o.set("date", optional(&p.date));
                        o.set("version", optional(&p.version));
                        o.set("description", optional(&p.description));
                        o.set("span", source(p.span));
                        o
                    }),
                );
                v.set("loaded_by", source(record.loaded_by));
                v.set(
                    "options_declared",
                    Value::Arr(
                        record
                            .options_declared
                            .iter()
                            .map(|(name, span)| {
                                let mut o = Value::obj();
                                o.set("name", str_(name.clone()));
                                o.set("span", source(*span));
                                o
                            })
                            .collect(),
                    ),
                );
                v.set(
                    "definitions",
                    Value::Arr(
                        record
                            .definitions
                            .iter()
                            .map(|d| {
                                let mut o = Value::obj();
                                o.set("name", str_(d.name.clone()));
                                o.set("kind", str_(d.kind));
                                o.set("definer", str_(d.definer.clone()));
                                o.set("arity", Value::Num(d.arity as f64));
                                o.set("optional_default", optional(&d.optional_default));
                                o.set("signature", str_(d.signature.clone()));
                                o.set("span", source(d.span));
                                o.set("overrides", Value::Bool(d.overrides));
                                if d.kind == "theorem" {
                                    o.set("title", optional(&d.title));
                                    o.set("within", optional(&d.within));
                                }
                                o
                            })
                            .collect(),
                    ),
                );
                v
            })
            .collect(),
    )
}

/// Label every diagnostic raised inside a project package or class file
/// with the command that loaded the file, and, when that command is itself
/// in a package file, with that file's loader too, up to the document --
/// `b.sty is loaded here` (in `a.sty`), `a.sty is loaded here` (in
/// `main.tex`) -- as LaTeX's log prints the stack of files being read.
/// `package_files` is `(document, loading command)` per file in loading
/// order; a label a diagnostic already carries is not repeated.
pub fn label_load_chain(diagnostics: &mut [Diagnostic], package_files: &[(DocumentId, Span)], paths: &[&str]) {
    if package_files.is_empty() {
        return;
    }
    let file_name = |document: DocumentId| {
        let path = paths.get(document.0).copied().unwrap_or("");
        path.rsplit('/').next().unwrap_or(path).to_string()
    };
    for diagnostic in diagnostics.iter_mut() {
        let Some(span) = diagnostic.span else { continue };
        let mut document = span.document;
        // Follow the chain outward; the limit guards a cycle in the records.
        for _ in 0..package_files.len() {
            let Some((_, loaded_at)) = package_files.iter().find(|(file, _)| *file == document) else { break };
            if !diagnostic.labels.iter().any(|l| l.span == *loaded_at) {
                *diagnostic = std::mem::replace(diagnostic, Diagnostic::error("", None, None))
                    .with_label(*loaded_at, format!("{} is loaded here", file_name(document)), false);
            }
            if loaded_at.document == document {
                break;
            }
            document = loaded_at.document;
        }
    }
}
