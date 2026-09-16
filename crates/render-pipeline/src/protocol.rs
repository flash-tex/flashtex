//! runtime-v1 JSON Lines transport for `flashtex-render`: reads `compile`
//! envelopes, replies with `compile_result` (the v1 fallback), follows it
//! with the rendering-v2 `display_list` line when `display-list-v2` was
//! negotiated (`docs/contracts/runtime-v1-display-list-v2.md`), and can
//! keep the v2 display list of the last request for `--v2`/`--pdf`.
//! Unknown protocol versions, message types and unsafe paths are rejected
//! with the compiler's error envelopes — never a silent partial success.

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::parser::SourceDocument;
use flashtex_compiler::protocol::{error_envelope, PROTOCOL_VERSION};

use crate::delta::{self, DeltaState};
use crate::display::PageWindow;
use crate::v1::Capabilities;
use crate::{render_windowed, FontSet, RenderCache, RenderOptions, Rendered};

pub const MAX_LINE_BYTES: usize = flashtex_compiler::protocol::MAX_LINE_BYTES;
/// Largest reply line the Mac reader accepts (`JSONLines.maxLineBytes`);
/// a larger result is failed explicitly instead of being cut off.
/// `FLASHTEX_MAX_REPLY_BYTES` lowers it (tests exercise the limit paths).
pub const MAX_REPLY_BYTES: usize = 16 * 1024 * 1024;

pub fn max_reply_bytes() -> usize {
    std::env::var("FLASHTEX_MAX_REPLY_BYTES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .map_or(MAX_REPLY_BYTES, |v| v.min(MAX_REPLY_BYTES))
}
pub const MAX_CAPABILITIES: usize = 16;
pub const MAX_CAPABILITY_BYTES: usize = 64;

/// Project-relative paths only: no absolute paths, no parent traversal.
pub fn path_is_safe(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') || path.starts_with('\\') {
        return false;
    }
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        return false;
    }
    !path.split(['/', '\\']).any(|c| c == "..")
}

/// Most documents the closure adds to one request, and the largest one it
/// reads. The same bounds the Mac client applies to its own scan
/// (`ProjectDocuments.maxClosureDocuments`, `ProjectIncludes.maxDocumentBytes`).
///
/// GH-735: these are enforced **at discovery** (`closure_read_budget`), not
/// only on the way out, because this runs on the compile path a keystroke
/// drives. Bounding only what is forwarded would leave the read itself
/// unbounded: a 62 MiB include was read and then dropped, at 179 ms against a
/// 3 ms baseline, and 2000 includes were all read to forward 256.
pub const MAX_CLOSURE_DOCUMENTS: usize = 256;
pub const MAX_CLOSURE_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;

/// Total bytes one request's closure may read off disk, across every file
/// discovery opens -- `.tex`, `.bib` and the `\includegraphics` targets it
/// reads and discards. `MAX_CLOSURE_DOCUMENTS * MAX_CLOSURE_DOCUMENT_BYTES` is
/// 2 GiB, which is not a bound worth having on a per-keystroke path; 32 MiB is
/// four times the largest single document and far above any real LaTeX source
/// closure, so the projects that reach it are the ones whose author wants to
/// be told rather than to typeset at 200 ms a keystroke.
pub const MAX_CLOSURE_READ_BYTES: u64 = 32 * 1024 * 1024;

/// What one request's closure would cost to read, and why it was refused.
struct ReadBudget {
    /// Files charged so far -- only the ones actually read off disk; an
    /// overlaid buffer costs nothing, which is why the client's normal case
    /// (every document sent) never approaches the limit.
    documents: usize,
    bytes: u64,
    /// Set once, by the first charge that did not fit. Discovery is then not
    /// run at all.
    exceeded: Option<String>,
}

impl ReadBudget {
    /// Charges `len` bytes for `path`. Returns false once the budget is spent,
    /// after which the walk stops.
    fn charge(&mut self, path: &str, len: u64) -> bool {
        if self.exceeded.is_some() {
            return false;
        }
        let limit = MAX_CLOSURE_DOCUMENT_BYTES as u64;
        if len > limit {
            self.exceeded = Some(format!(
                "{path} is {len} bytes, larger than the {limit}-byte limit on one included document"
            ));
            return false;
        }
        self.documents += 1;
        if self.documents > MAX_CLOSURE_DOCUMENTS {
            self.exceeded = Some(format!(
                "the include closure reads more than {MAX_CLOSURE_DOCUMENTS} files off disk (reached at {path})"
            ));
            return false;
        }
        self.bytes += len;
        if self.bytes > MAX_CLOSURE_READ_BYTES {
            self.exceeded = Some(format!(
                "the include closure reads more than {MAX_CLOSURE_READ_BYTES} bytes off disk (reached at {path})"
            ));
            return false;
        }
        true
    }
}

/// GH-735: how much disk `ProjectGraph::discover_with` would read for this
/// request, decided **before** it runs.
///
/// The walk this mirrors -- same candidate order, same `exists` test, same
/// containment check, same cycle/diamond/depth rules -- is
/// `vendor/project-files`'s `Discovery`, which has no budget of its own and
/// which this lane may not edit. So the closure is costed first, out of
/// `metadata()` (a `stat`, not a read) for every file and a read only of the
/// `.tex` files whose references have to be followed, and the walk proper runs
/// only when the whole thing fits.
///
/// The probe never widens what is read: a path it declines to read is a path
/// discovery also declines (it applies `is_file()` and the same
/// `canonicalize`-inside-root test before charging anything), and the bytes it
/// reads are never forwarded -- discovery re-reads, from warm page cache, the
/// files it is allowed to forward. A bug here can therefore cost a false
/// refusal or an unbounded read; it cannot carry out-of-root bytes anywhere.
/// Feeding the probe's own bytes to discovery through the overlay would save
/// that second read, and would also make this the thing that decides what is
/// in root -- which is the one job it is deliberately not given. The second
/// read is what that costs: nothing measurable on a real project
/// (`fixtures/real-world/thesis-chapter`, 11.30 -> 11.16 ms a compile), and
/// 8.8 -> 12.2 ms on a synthetic 200-file on-disk closure.
fn closure_read_budget(
    root: &std::path::Path,
    entry: &flashtex_project_files::ProjectPath,
    overlay: &flashtex_project_files::graph::Overlay,
) -> Option<String> {
    use flashtex_project_files::graph::{candidates, FileKind, Overlay, MAX_DEPTH};
    use flashtex_project_files::{scan_references, ProjectPath};

    let Ok(canonical_root) = std::fs::canonicalize(root) else { return None };

    struct Walk<'a> {
        root: &'a std::path::Path,
        canonical_root: std::path::PathBuf,
        overlay: &'a Overlay,
        seen: std::collections::BTreeSet<ProjectPath>,
        ancestors: Vec<ProjectPath>,
        budget: ReadBudget,
    }

    impl Walk<'_> {
        /// `Discovery::exists`.
        fn exists(&self, path: &ProjectPath) -> bool {
            self.overlay.get(path).is_some() || path.to_os_path(self.root).is_file()
        }

        /// `Discovery::escapes_via_symlink`.
        fn escapes(&self, path: &ProjectPath) -> bool {
            if self.overlay.get(path).is_some() {
                return false;
            }
            match std::fs::canonicalize(path.to_os_path(self.root)) {
                Ok(canon) => !canon.starts_with(&self.canonical_root),
                Err(_) => false,
            }
        }

        /// Charges `path` and, for a `.tex` file, returns the text whose
        /// references still have to be followed. `None` means "walk no
        /// further here" -- either the file costs nothing more to look at
        /// (a `.bib`, a graphic) or the budget is spent.
        fn charge(&mut self, path: &ProjectPath, kind: FileKind) -> Option<String> {
            if kind != FileKind::Graphic {
                if let Some(text) = self.overlay.get(path) {
                    // Served from memory by discovery too: no disk, no charge.
                    return Some(text.to_string());
                }
            }
            let os = path.to_os_path(self.root);
            // Missing or unreadable: discovery diagnoses it and reads nothing.
            let len = std::fs::metadata(&os).ok()?.len();
            if !self.budget.charge(path.as_str(), len) {
                return None;
            }
            if kind != FileKind::Tex {
                // Discovery reads it, and never descends into it.
                return None;
            }
            std::fs::read_to_string(&os).ok()
        }

        fn visit(&mut self, path: &ProjectPath, kind: FileKind) {
            let Some(text) = self.charge(path, kind) else { return };
            self.seen.insert(path.clone());
            self.ancestors.push(path.clone());
            for r in scan_references(&text) {
                if self.budget.exceeded.is_some() {
                    break;
                }
                if !r.literal {
                    continue;
                }
                let Ok(base) = ProjectPath::normalize(&r.argument) else { continue };
                let (kind, cands) = candidates(r.kind, &base);
                let Some(target) = cands.iter().find(|c| self.exists(c)).cloned() else { continue };
                if self.escapes(&target)
                    || self.ancestors.contains(&target)
                    || self.seen.contains(&target)
                    || (kind == FileKind::Tex && self.ancestors.len() >= MAX_DEPTH)
                {
                    continue;
                }
                self.visit(&target, kind);
            }
            self.ancestors.pop();
        }
    }

    let mut walk = Walk {
        root,
        canonical_root,
        overlay,
        seen: std::collections::BTreeSet::new(),
        ancestors: Vec::new(),
        budget: ReadBudget { documents: 0, bytes: 0, exceeded: None },
    };
    walk.visit(entry, FileKind::Tex);
    walk.budget.exceeded
}

/// GH-75: completes the `\input`/`\include` closure of `entry` from
/// `root`, returning the documents the request did **not** carry plus the
/// diagnostics discovery raised about references it refused.
///
/// The request's own documents are overlaid on the walk, so an unsaved buffer
/// is what gets scanned for further includes and is never replaced by the
/// stale bytes on disk; and the walk itself is project-files' rooted,
/// symlink-refusing discovery -- the same one `flashtex build` uses -- so an
/// include resolving outside the root is refused here rather than re-checked
/// with a second containment rule.
///
/// Nothing here is fatal: with no root, an unusable entry path, or a root that
/// cannot be opened, the compile proceeds on exactly the documents the request
/// sent, which is what it did before this existed.
fn closure_from_disk(
    root: Option<&std::path::Path>,
    entry: &str,
    supplied: &[(String, String)],
) -> (Vec<(String, String)>, Vec<crate::display::Diagnostic>) {
    use flashtex_project_files::graph::{DiagnosticKind, Overlay, ProjectGraph, Severity};
    use flashtex_project_files::ProjectPath;

    let none = (Vec::new(), Vec::new());
    let Some(root) = root else { return none };
    let Ok(entry_path) = ProjectPath::normalize(entry) else { return none };
    let mut overlay = Overlay::new();
    let mut have = std::collections::BTreeSet::new();
    for (path, text) in supplied {
        let Ok(normalized) = ProjectPath::normalize(path) else { continue };
        have.insert(normalized.as_str().to_string());
        overlay.insert(normalized, text.clone());
    }
    // GH-735: cost the closure before reading it. Over budget, nothing is
    // discovered at all -- the compile falls back to exactly the documents the
    // request sent, which is what it did before the closure existed -- and the
    // reason is an error diagnostic. Never half a closure: a document that
    // quietly typesets with some of its includes missing is worse than one
    // that says why they are.
    if let Some(reason) = closure_read_budget(root, &entry_path, &overlay) {
        return (
            Vec::new(),
            vec![crate::display::Diagnostic::error(
                "closure_budget_exceeded",
                format!("{reason}; compiling only the documents the request sent"),
                Vec::new(),
            )],
        );
    }
    let Ok(graph) = ProjectGraph::discover_with(root, &entry_path, &overlay) else { return none };

    let mut documents = Vec::new();
    for document in graph.documents() {
        if documents.len() >= MAX_CLOSURE_DOCUMENTS {
            break;
        }
        // The overlaid documents come back out of the graph unchanged; only
        // what the request did not send is new. `path_is_safe` is the same
        // gate the request's own paths passed, applied again because these
        // paths did not come from the request.
        if have.contains(document.path.as_str()) || !path_is_safe(&document.path) {
            continue;
        }
        if document.text.len() > MAX_CLOSURE_DOCUMENT_BYTES {
            continue;
        }
        documents.push((document.path, document.text));
    }

    let diagnostics = graph
        .diagnostics()
        .iter()
        .filter_map(|d| {
            // A missing include and a macro-built path are the compiler's to
            // report: it has the candidate list and the expander, and saying
            // it twice in one compile helps nobody.
            let code = match d.kind {
                DiagnosticKind::MissingFile { .. } | DiagnosticKind::UnresolvableReference { .. } | DiagnosticKind::Cycle { .. } => return None,
                DiagnosticKind::InvalidPath { .. } => "invalid_path",
                DiagnosticKind::EscapesRootViaSymlink { .. } => "path_escapes_root",
                DiagnosticKind::InvalidUtf8 { .. } => "not_utf8",
                DiagnosticKind::ReadError { .. } => "read_error",
                DiagnosticKind::DepthExceeded { .. } => "include_depth",
            };
            let sources = d
                .span
                .map(|s| {
                    vec![crate::display::SourceRange {
                        path: std::rc::Rc::from(d.path.as_str()),
                        start_byte: s.start,
                        end_byte: s.end,
                    }]
                })
                .unwrap_or_default();
            Some(match d.severity {
                Severity::Error => crate::display::Diagnostic::error(code, d.message.clone(), sources),
                Severity::Warning => crate::display::Diagnostic::warning(code, d.message.clone(), sources),
            })
        })
        .collect();
    (documents, diagnostics)
}

/// The request's `display_list_window` (`display-list-v2-window` §4), when it
/// is one a producer can serve. `first_page` is 1-based; the effective window
/// is decided by the render, which clamps against the page count layout
/// produced, and is reported back in the sibling's `window` object.
fn window_of(payload: &Value) -> Option<PageWindow> {
    let w = payload.get("display_list_window")?;
    let field = |k: &str| w.get(k).and_then(|v| v.as_i64()).filter(|v| *v > 0 && *v <= i64::from(u32::MAX));
    Some(PageWindow {
        first_page: field("first_page")? as u32,
        page_count: field("page_count")? as u32,
    })
}

/// The outcome of one request line.
pub struct Reply {
    /// The JSON Lines reply to write.
    pub line: String,
    /// Lines to write right after `line`, before any later reply: the
    /// `display_list` envelope when `display-list-v2` was accepted.
    pub extra_lines: Vec<String>,
    /// The render, when the request compiled (for `--v2`/`--pdf`).
    pub rendered: Option<Rendered>,
    pub id: String,
}

fn failed(id: &str, project_id: &str, revision: i64, message: &str, accepted: Option<Vec<String>>) -> Value {
    let mut payload = Value::obj();
    payload.set("project_id", json::str_(project_id));
    payload.set("revision", json::num(revision as f64));
    payload.set("status", json::str_("failed"));
    payload.set("pages", Value::Arr(Vec::new()));
    let mut d = Value::obj();
    d.set("severity", json::str_("error"));
    d.set("message", json::str_(message));
    d.set("source", Value::Null);
    d.set("recovery", Value::Null);
    payload.set("diagnostics", Value::Arr(vec![d]));
    payload.set("pdf_path", Value::Null);
    if let Some(acc) = accepted {
        payload.set("layout_capabilities", Value::Arr(acc.into_iter().map(json::str_).collect()));
    }
    result_envelope(id, payload)
}

fn result_envelope(id: &str, payload: Value) -> Value {
    let mut v = Value::obj();
    v.set("protocol_version", json::num(PROTOCOL_VERSION as f64));
    v.set("id", json::str_(id));
    v.set("type", json::str_("compile_result"));
    v.set("payload", payload);
    v
}

/// The runtime-v1 worker loop: requests on `input`, one reply per line on
/// `output` (plus the `display_list` line when negotiated), until EOF or a
/// write failure. A block cache lives across requests so a keystroke
/// retypesets only the paragraph it touched. `on_rendered` sees each
/// request's id and render after its reply was flushed (`--v2`/`--pdf`
/// side outputs). Malformed and oversized lines are answered with the
/// compiler's error envelopes, never dropped. Read errors are returned.
pub fn serve<R: std::io::BufRead, W: std::io::Write>(
    input: &mut R,
    output: &mut W,
    fonts: &FontSet,
    options: &RenderOptions,
    mut on_rendered: impl FnMut(&str, &Rendered),
) -> std::io::Result<()> {
    use flashtex_compiler::protocol::{read_request_line, RequestLine};
    let cache = RenderCache::new();
    let delta_state = DeltaState::new();
    let error = |code: &str, msg: &str| Reply {
        line: json::write(&error_envelope("", code, msg)),
        extra_lines: Vec::new(),
        rendered: None,
        id: String::new(),
    };
    loop {
        let reply = match read_request_line(input)? {
            Some(RequestLine::Data(bytes)) => match std::str::from_utf8(&bytes) {
                Ok(line) if line.trim().is_empty() => continue,
                Ok(line) => handle_line_with(line, fonts, options, Some(&cache), Some(&delta_state)),
                Err(_) => error("invalid_utf8", "request line is not valid UTF-8"),
            },
            Some(RequestLine::TooLarge) => error("payload_too_large", &format!("line exceeds the {MAX_LINE_BYTES}-byte limit")),
            None => return Ok(()),
        };
        if writeln!(output, "{}", reply.line).is_err() {
            return Ok(());
        }
        for extra in &reply.extra_lines {
            if writeln!(output, "{extra}").is_err() {
                return Ok(());
            }
        }
        let _ = output.flush();
        if let Some(r) = &reply.rendered {
            on_rendered(&reply.id, r);
        }
    }
}

/// Handles one request line (no `display-list-v2-delta` state: `-delta` is
/// never accepted).
pub fn handle_line(line: &str, fonts: &FontSet, options: &RenderOptions, cache: Option<&RenderCache>) -> Reply {
    handle_line_with(line, fonts, options, cache, None)
}

/// [`handle_line`] with the worker's delta state (`display-list-v2-delta`,
/// `crate::delta`). A reply without a sibling line clears the delta chain
/// (proposal r5 §3): the next request answers full.
pub fn handle_line_with(line: &str, fonts: &FontSet, options: &RenderOptions, cache: Option<&RenderCache>, delta_state: Option<&DeltaState>) -> Reply {
    let reply = handle_line_inner(line, fonts, options, cache, delta_state);
    if reply.extra_lines.is_empty() {
        if let Some(state) = delta_state {
            state.clear();
        }
    }
    reply
}

fn handle_line_inner(line: &str, fonts: &FontSet, options: &RenderOptions, cache: Option<&RenderCache>, delta_state: Option<&DeltaState>) -> Reply {
    let err = |id: &str, code: &str, msg: &str| Reply {
        line: json::write(&error_envelope(id, code, msg)),
        extra_lines: Vec::new(),
        rendered: None,
        id: id.to_string(),
    };
    if line.len() > MAX_LINE_BYTES {
        return err("", "payload_too_large", &format!("line exceeds the {}-byte limit", MAX_LINE_BYTES));
    }
    let value = match json::parse(line) {
        Ok(v) => v,
        Err(e) => return err("", "malformed_json", &e.0),
    };
    let id = value.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    match value.get("protocol_version").and_then(|v| v.as_i64()) {
        Some(PROTOCOL_VERSION) => {}
        Some(other) => {
            return err(
                &id,
                "unsupported_protocol_version",
                &format!("protocol version {other} is not supported; this build speaks version {PROTOCOL_VERSION}"),
            )
        }
        None => return err(&id, "missing_protocol_version", "protocol_version is required"),
    }
    match value.get("type").and_then(|v| v.as_str()) {
        Some("compile") => {}
        Some(other) => return err(&id, "unsupported_type", &format!("message type '{other}' is not supported")),
        None => return err(&id, "missing_type", "type is required"),
    }
    let Some(payload) = value.get("payload") else {
        return err(&id, "missing_payload", "compile requires a payload");
    };
    let project_id = payload.get("project_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let revision = payload.get("revision").and_then(|v| v.as_i64()).unwrap_or(0);
    let entry = payload.get("entry_path").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // Capability negotiation (runtime-v1-layout-capabilities.md).
    let mut requested: Option<Vec<String>> = None;
    if let Some(caps) = payload.get("layout_capabilities") {
        let Some(arr) = caps.as_arr() else {
            return Reply {
                line: json::write(&failed(&id, &project_id, revision, "layout_capabilities must be an array of strings", None)),
                extra_lines: Vec::new(),
                rendered: None,
                id,
            };
        };
        let mut list = Vec::new();
        for c in arr {
            match c.as_str() {
                Some(s) if !s.is_empty() && s.len() <= MAX_CAPABILITY_BYTES && !list.contains(&s.to_string()) => list.push(s.to_string()),
                _ => {
                    return Reply {
                        line: json::write(&failed(
                            &id,
                            &project_id,
                            revision,
                            "layout_capabilities entries must be unique nonempty strings of at most 64 bytes",
                            None,
                        )),
                        extra_lines: Vec::new(),
                        rendered: None,
                        id,
                    };
                }
            }
        }
        if list.len() > MAX_CAPABILITIES {
            return Reply {
                line: json::write(&failed(&id, &project_id, revision, "layout_capabilities lists more than 16 entries", None)),
                extra_lines: Vec::new(),
                rendered: None,
                id,
            };
        }
        requested = Some(list);
    }
    let (caps, accepted) = match &requested {
        Some(list) => {
            let (c, a) = Capabilities::negotiate(list);
            (c, Some(a))
        }
        None => (Capabilities::default(), None),
    };

    let empty = Vec::new();
    let docs = payload.get("documents").and_then(|v| v.as_arr()).unwrap_or(&empty);
    let mut project: Vec<(String, String)> = Vec::with_capacity(docs.len());
    for d in docs {
        let p = d.get("path").and_then(|v| v.as_str()).unwrap_or("");
        if !path_is_safe(p) {
            return Reply {
                line: json::write(&failed(
                    &id,
                    &project_id,
                    revision,
                    &format!("rejected document path '{p}': paths must be project-relative with no parent traversal"),
                    accepted,
                )),
                extra_lines: Vec::new(),
                rendered: None,
                id,
            };
        }
        project.push((p.to_string(), d.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string()));
    }
    if !entry.is_empty() && !path_is_safe(&entry) {
        return Reply {
            line: json::write(&failed(
                &id,
                &project_id,
                revision,
                &format!("rejected entry_path '{entry}': paths must be project-relative with no parent traversal"),
                accepted,
            )),
            extra_lines: Vec::new(),
            rendered: None,
            id,
        };
    }
    if project.is_empty() {
        return Reply {
            line: json::write(&failed(&id, &project_id, revision, "no documents supplied to compile", accepted)),
            extra_lines: Vec::new(),
            rendered: None,
            id,
        };
    }
    let entry_path = if project.iter().any(|(p, _)| *p == entry) {
        entry
    } else {
        project[0].0.clone()
    };
    // PROPOSAL (FT-063): an optional absolute `project_root` directory the
    // request's `\includegraphics` files are read from (rooted, no symlinks,
    // through project-files). A relative or empty value is ignored.
    let request_root = payload
        .get("project_root")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute());
    // GH-75: the request carries the buffers the client has open, which for an
    // IDE is "whatever the user happened to click on". An `\input` of a file
    // that exists in the project but is not open used to compile as `included
    // file not found`, and every `\ref` into it as `??` -- a plain
    // `main.tex` + `sections/intro.tex` document was unusable until each file
    // was opened by hand. When the request says where the project is, the
    // rest of the include closure is read from disk through the same rooted,
    // symlink-refusing discovery `flashtex build` uses, with the request's own
    // documents overlaid so an unsaved buffer always wins over the file.
    let closure_root = request_root.as_deref().or(options.project_root.as_deref());
    let (from_disk, closure_diagnostics) = closure_from_disk(closure_root, &entry_path, &project);
    project.extend(from_disk);
    let sources: Vec<SourceDocument<'_>> = project
        .iter()
        .map(|(p, t)| SourceDocument {
            path: p.as_str(),
            text: t.as_str(),
        })
        .collect();
    // `payload.date` -- the civil date `\today` renders
    // (protocol/proposals/runtime-v1-request-date.md). This worker never reads
    // the clock: runtime-v1 requires byte-identical output for byte-identical
    // input, so the caller reads it and sends the answer.
    //
    // Absent means the Unix epoch, exactly what this worker printed before the
    // field existed, so old clients and committed fixtures are byte-identical.
    // Malformed is an error -- never a silent fallback to some other date.
    let request_date = match payload.get("date") {
        None => None,
        Some(v) => {
            let Some(text) = v.as_str() else {
                return Reply {
                    line: json::write(&failed(&id, &project_id, revision,
                        "compile payload 'date' must be a string in YYYY-MM-DD form", None)),
                    extra_lines: Vec::new(),
                    rendered: None,
                    id,
                };
            };
            match crate::date::TodayDate::parse_iso(text) {
                Ok(date) => Some(date),
                Err(error) => {
                    return Reply {
                        line: json::write(&failed(&id, &project_id, revision,
                            &format!("compile payload 'date' is invalid ({text:?}): {error}"), None)),
                        extra_lines: Vec::new(),
                        rendered: None,
                        id,
                    };
                }
            }
        }
    };
    let with_request_fields;
    let options = if request_root.is_some() || request_date.is_some() {
        with_request_fields = RenderOptions {
            project_root: request_root.or_else(|| options.project_root.clone()),
            today: request_date.unwrap_or(options.today),
            ..options.clone()
        };
        &with_request_fields
    } else {
        options
    };
    // display-list-v2-window (proposal §4): where the viewer is. Meaningful
    // only next to an accepted `display-list-v2-window`; absent with the
    // capability listed means the consumer supports a window but has not said
    // where it wants one, and the reply is unwindowed. Per §8 an unusable
    // window (page 0, count 0, a non-object, a missing field) is not an error:
    // the reply is unwindowed and the name is absent from the echo, which is
    // the consumer's only signal either way.
    let requested_window = caps.window.then(|| window_of(payload)).flatten();
    let limit = max_reply_bytes();
    let revision_u64 = revision.max(0) as u64;
    let mut rendered = render_windowed(&sources, &entry_path, revision_u64, &project_id, fonts, options, cache, requested_window);
    // A window the consumer chose can still be too wide to serialise -- a
    // window is bounded by `MAX_WINDOW_PAGES`, which is a ceiling on what a
    // viewer shows, not on bytes. Rather than decline the sibling (which for a
    // long document is the `status: failed` this capability exists to fix),
    // narrow the window to what the limit actually carries, measured on the
    // pages just built, and serve that. The echoed `window` states what was
    // served, so a narrowing needs no diagnostic and is not an error (§8).
    if let Some(served) = rendered.v2.window.filter(|_| rendered.v2.estimated_json_bytes() > limit) {
        let resident = u64::from(served.page_count).max(1);
        let per_page = (rendered.v2.estimated_json_bytes() as u64).div_ceil(resident);
        let centre = served.first_page + served.page_count / 2;
        let narrowed = PageWindow::fitting(centre, rendered.v2.pages.len() as u32, per_page, limit as u64);
        if narrowed.is_some_and(|n| n.page_count < served.page_count) {
            rendered = render_windowed(&sources, &entry_path, revision_u64, &project_id, fonts, options, cache, narrowed);
        }
    }
    // GH-75: discovery's own refusals (an include that resolves outside the
    // root, an unreadable file). The compiler would otherwise report only
    // "included file not found" for a file that is there but was refused,
    // which says the wrong thing about why.
    if !closure_diagnostics.is_empty() {
        rendered.v2.diagnostics.extend(closure_diagnostics);
    }
    let mut v1 = crate::v1::fallback(&rendered.v2, caps, accepted.clone());
    // display-list-v2: the envelope is serialised first because declining it
    // (over the line limit) changes the echoed capabilities and diagnostics
    // of the compile_result that precedes it.
    let mut extra_lines = Vec::new();
    let drop_cap = |v1: &mut crate::v1::V1Payload, cap: &str| {
        v1.accepted = v1.accepted.take().map(|a| a.into_iter().filter(|c| c != cap).collect());
    };
    let drop_display_list_family = |v1: &mut crate::v1::V1Payload| {
        v1.accepted = v1.accepted.take().map(|a| a.into_iter().filter(|c| !crate::v1::is_display_list_family(c)).collect());
    };
    if caps.display_list && v1.status != "failed" {
        let wire = crate::display::Wire { images: caps.images, device_color: caps.device_color, diagnostics: caps.diagnostics };
        // display-list-v2-delta (proposal r5 §3): against the acknowledged
        // installed base, when it is also this worker's last emitted sibling.
        let base = if caps.delta { payload.get("display_list_base").and_then(delta::Base::from_json) } else { None };
        let mut emitted_delta = false;
        if let (Some(state), Some(base)) = (delta_state, base) {
            if let Some(line) = delta::try_delta(state, &id, &rendered.v2, wire, &base, &project, limit) {
                extra_lines.push(line);
                emitted_delta = true;
            }
        }
        if !emitted_delta {
            // Size first (an upper-bound estimate, then the exact line), so an
            // oversized frame is declined without serialising 16+ MB in vain.
            let estimate = rendered.v2.estimated_json_bytes_for(wire);
            let mut page_bytes = Vec::new();
            let dl = if estimate > limit {
                None
            } else if caps.delta && delta_state.is_some() {
                Some(rendered.v2.write_json_wire_measured(&id, wire, &mut page_bytes))
            } else {
                Some(rendered.v2.write_json_wire(&id, wire))
            };
            let too_big = dl.as_ref().map_or(estimate, String::len);
            match dl {
                Some(dl) if dl.len() <= limit => {
                    if let Some(state) = delta_state.filter(|_| caps.delta) {
                        delta::note_full(state, &id, &rendered.v2, wire, page_bytes, dl.len(), &project);
                    }
                    extra_lines.push(dl);
                }
                _ => {
                    drop_display_list_family(&mut v1);
                    v1.diagnostics.push(crate::display::Diagnostic::warning(
                        "display_list_declined",
                        format!(
                            "display-list-v2 declined: the display_list line would be about {too_big} bytes for {} pages, over the {limit}-byte line limit",
                            rendered.v2.pages.len()
                        ),
                        Vec::new(),
                    ));
                    if v1.status == "ok" {
                        v1.status = "recovered";
                    }
                }
            }
            drop_cap(&mut v1, crate::v1::CAP_DELTA);
        }
        // display-list-v2-only: the v1 pages are elided only when a sibling
        // line actually carries the frame.
        if caps.v2_only && !extra_lines.is_empty() {
            v1.pages.clear();
        } else {
            drop_cap(&mut v1, crate::v1::CAP_V2_ONLY);
        }
        // display-list-v2-window: echoed only on a reply that actually carries
        // a window, the way `-only` is echoed only when the pages were really
        // elided. A consumer that listed the name and got it back knows the
        // reply is an incomplete view and reads the sibling's `window` object
        // for its coverage (§4.1).
        if rendered.v2.window.is_none() || extra_lines.is_empty() {
            drop_cap(&mut v1, crate::v1::CAP_WINDOW);
        }
    } else {
        drop_cap(&mut v1, crate::v1::CAP_DELTA);
        drop_cap(&mut v1, crate::v1::CAP_V2_ONLY);
        drop_cap(&mut v1, crate::v1::CAP_WINDOW);
    }
    let accepted = v1.accepted.clone();
    // Size first, as the display_list branch above: when the envelope cannot
    // fit the reply limit, refuse it without serialising the 16+ MB line the
    // refusal replaces. `envelope_len` is the exact length `write_envelope`
    // would produce, so the refusal message carries the same byte count it
    // always did, and a line that fits is written exactly as before. The
    // pre-check runs only when the item-count heuristic says the limit is in
    // reach; small replies skip both passes.
    let line_len = {
        let approx = 64 + 96 * v1.pages.iter().map(|p| p.items.len()).sum::<usize>();
        if approx > limit / 4 {
            Some(v1.envelope_len(&id))
        } else {
            None
        }
    };
    let line = match line_len {
        Some(len) if len > limit => None,
        _ => {
            let line = v1.write_envelope(&id);
            debug_assert!(line_len.is_none_or(|len| len == line.len()), "envelope_len {line_len:?} != write_envelope {}", line.len());
            Some(line)
        }
    };
    let line = match line.filter(|l| l.len() <= limit) {
        Some(line) => line,
        None => {
            let len = line_len.unwrap_or_else(|| v1.envelope_len(&id));
            let pages = rendered.v2.pages.len();
            return Reply {
                line: json::write(&failed(
                    &id,
                    &project_id,
                    revision,
                    &format!(
                        "compile_result would be {len} bytes for {pages} pages, over the {limit}-byte reply limit; split the project or compile fewer pages",
                    ),
                    accepted.map(|a| a.into_iter().filter(|c| !crate::v1::is_display_list_family(c)).collect()),
                )),
                extra_lines: Vec::new(),
                rendered: Some(rendered),
                id,
            };
        }
    };
    Reply {
        line,
        extra_lines,
        rendered: Some(rendered),
        id,
    }
}
