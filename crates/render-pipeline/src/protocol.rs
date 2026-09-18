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
/// GH-735: these are enforced **at discovery** (`discover_closure`), not only
/// on the way out, because this runs on the compile path a keystroke drives.
/// Bounding only what is forwarded would leave the read itself unbounded: a
/// 62 MiB include was read and then dropped, at 179 ms against a 3 ms
/// baseline, and 2000 includes were all read to forward 256.
pub const MAX_CLOSURE_DOCUMENTS: usize = 256;
pub const MAX_CLOSURE_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;

/// Total bytes one request's closure may read off disk, across every `.tex`
/// and `.bib` file discovery opens. A `\includegraphics` target is charged
/// nothing here at all -- GH-INCLUDEGRAPHICS-READ: nothing this walk returns
/// needs its bytes (see `Walk::load`'s doc comment), so its `stat` size is
/// not weighed and its existence is not counted. One `stat` per figure is
/// microseconds against the millisecond reads this budget exists to bound,
/// so a fan-out of huge images needs no bound of its own here.
/// `MAX_CLOSURE_DOCUMENTS * MAX_CLOSURE_DOCUMENT_BYTES` is 2 GiB, which is not
/// a bound worth having on a per-keystroke path; 32 MiB is four times the
/// largest single document and far above any real LaTeX source closure, so
/// the projects that reach it are the ones whose author wants to be told
/// rather than to typeset at 200 ms a keystroke.
pub const MAX_CLOSURE_READ_BYTES: u64 = 32 * 1024 * 1024;

/// What one request's closure would cost to read, and why it was refused.
struct ReadBudget {
    /// Files charged so far -- only the ones actually opened off disk; an
    /// overlaid buffer costs nothing, which is why the client's normal case
    /// (every document sent) never approaches the limit.
    documents: usize,
    bytes: u64,
    /// Set once, by the first charge that did not fit. The walk stops
    /// descending further once this is set, and the whole closure is then
    /// discarded -- never half of it.
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

/// What [`Walk::load`] (inside [`discover_closure`]) found for one candidate.
enum Load {
    /// Text served from the request's own overlay -- no disk touched, no
    /// charge. Never returned for a `Graphic` target (matches
    /// `Discovery::load`: an overlay holds edited text buffers, and nothing
    /// here ever needs a graphic's bytes anyway).
    Overlay(String),
    /// A `Tex` or `Bibliography` file's text, read off disk and charged.
    Disk(String),
    /// A `Graphic` target: [`Walk::exists`] plus the symlink-refusal check
    /// already proved it is there and no symlink leads to it, and its `stat`
    /// size is charged nothing -- its bytes are never opened. See
    /// [`discover_closure`]'s doc comment for what actually still needs them.
    DiskGraphic,
    /// The candidate itself or an ancestor component below the root is a
    /// symlink: refused without following it, carrying the diagnostic detail
    /// (the part after `\command{argument}: `). Never read, never charged.
    SymlinkRefused(String),
    /// `stat`/`read` raced with an external deletion between `exists()` and
    /// here. A genuine miss is the compiler's own "not found" to raise, not
    /// this walk's (matches `MissingFile`, dropped in [`closure_from_disk`]).
    Missing,
    /// Read, but not valid UTF-8.
    NotUtf8,
    /// `stat`/`read` failed for another reason (permissions, and so on).
    Error(String),
    /// The read budget is spent; `Walk::budget.exceeded` is already set.
    BudgetExceeded,
}

/// Discovers `entry`'s `\input`/`\include`/`\bibliography`/`\addbibresource`/
/// `\includegraphics` closure from `root` in one pass that both costs and
/// performs the walk -- there is no separate probe. `overlay` is served ahead
/// of disk for every path it holds (an unsaved buffer always wins), and is
/// what gets scanned for further references. `Ok` carries the documents the
/// request did not already supply, plus the diagnostics raised about
/// references refused along the way; `Err` is the reason the whole closure
/// was refused (GH-735's budget), in which case nothing here is kept.
///
/// This mirrors `vendor/project-files`'s `Discovery` (`ProjectGraph::
/// discover_with`) exactly for what render-pipeline needs -- same candidate
/// order (`candidates`), same `exists` test, same cycle/diamond/depth
/// rules -- built from that crate's own public path, scan and graph-kind
/// primitives, rather than calling `discover_with` itself, because
/// `discover_with` cannot be asked to stop at a `Graphic` target's `stat`:
/// its `Discovery::load` always does `fs::read` of the whole file before
/// noticing the kind it just loaded has no use for the bytes
/// (GH-INCLUDEGRAPHICS-READ). Containment differs from the vendored copy on
/// purpose: the vendored `escapes_via_symlink` only refuses a symlink whose
/// canonical target leaves the root, while the live `project-files` crate
/// refuses every symlink, wherever it points -- so this walk implements
/// that live refuse-all-symlinks rule locally (`symlink_metadata` on the
/// target and each ancestor component, with the live diagnostic text),
/// rather than inheriting the weaker vendored check. `vendor/project-files`
/// is pinned and read-only to this lane, so the walk moves here instead of
/// the fix moving there --
/// and, as a consequence, a `.tex` file is now read only once (discovery used
/// to cost it here, then `discover_with` reread it from warm page cache to
/// forward it; there is only one read now, by the same walk that costs it).
///
/// A `.tex`/`.bib` candidate is still read in full: its content decides what
/// to visit next (`.tex`) or whether it is valid UTF-8 (`.bib`), and `.tex`
/// text is what gets forwarded as a document. A `\includegraphics` target
/// only ever needs to answer "does this exist, and is it inside the root"
/// here -- nothing this function returns carries a graphic's bytes, size or
/// dimensions. Those are the compiler's own job, read once through the same
/// rooted primitive (`ImageCache::load` in `floats.rs`, capped at
/// `MAX_IMAGE_BYTES`) when a page that actually contains the image is laid
/// out, and cached per path across the render. Discovery visiting
/// `\includegraphics` targets at all does not change what that step reads;
/// discovery was just also, redundantly, reading the same bytes and
/// throwing them away.
///
/// Containment is enforced here directly (the refuse-all-symlinks check in
/// `Walk::load`, backstopped by `exists`/`escapes`), not re-verified
/// afterward by a second, vendor-owned walk: this **is** the walk now.
/// GH-735's adversarial tests (an escaping `..`, an escaping symlink, an
/// oversized or over-fanned-out closure) plus the in-root-symlink refusal
/// exercise this end to end, through [`handle_line`], and must keep passing.
fn discover_closure(
    root: &std::path::Path,
    entry: &flashtex_project_files::ProjectPath,
    overlay: &flashtex_project_files::graph::Overlay,
) -> Result<(Vec<(String, String)>, Vec<crate::display::Diagnostic>), String> {
    use flashtex_project_files::graph::{candidates, FileKind, MAX_DEPTH};
    use flashtex_project_files::{scan_references, Overlay, ProjectPath, Reference};

    struct Walk<'a> {
        root: &'a std::path::Path,
        canonical_root: std::path::PathBuf,
        overlay: &'a Overlay,
        seen: std::collections::BTreeSet<ProjectPath>,
        ancestors: Vec<ProjectPath>,
        budget: ReadBudget,
        documents: Vec<(String, String)>,
        diagnostics: Vec<crate::display::Diagnostic>,
    }

    impl Walk<'_> {
        /// `Discovery::exists`, except a symlink counts as existing (wherever
        /// it points, even nowhere): it must reach [`Walk::load`]'s refusal
        /// with the matching diagnostic, not be reported as a missing file.
        /// Directories never count, exactly as before.
        fn exists(&self, path: &ProjectPath) -> bool {
            if self.overlay.get(path).is_some() {
                return true;
            }
            match std::fs::symlink_metadata(path.to_os_path(self.root)) {
                Ok(m) => {
                    let file_type = m.file_type();
                    file_type.is_file() || file_type.is_symlink()
                }
                Err(_) => false,
            }
        }

        /// The live `project-files` refuse-all-symlinks rule, checked
        /// locally: the candidate itself or any ancestor component below the
        /// root must not be a symlink, wherever it points -- even inside the
        /// root. `symlink_metadata` never follows the final component, so a
        /// symlink is refused without reading through it. Returns the
        /// diagnostic detail (the part after `\command{argument}: `) with the
        /// live crate's wording. Overlaid paths never touch disk and are
        /// exempt, matching `escapes`.
        fn symlink_refusal(&self, path: &ProjectPath) -> Option<String> {
            if self.overlay.get(path).is_some() {
                return None;
            }
            let target = path.as_str();
            let mut prefix = String::new();
            for component in target.split('/') {
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(component);
                match std::fs::symlink_metadata(self.root.join(&prefix)) {
                    Ok(m) if m.file_type().is_symlink() => {
                        return Some(if prefix == target {
                            format!("{path} is a symbolic link; project files are read without following symlinks")
                        } else {
                            format!("{path}: `{prefix}` is a symbolic link; project files are read without following symlinks")
                        });
                    }
                    _ => {}
                }
            }
            None
        }

        /// Backstop for a non-symlink path whose canonical target still
        /// leaves the root. Unreachable for normalized project-relative paths
        /// (normalization rejects `..` and absolute paths, and every symlink
        /// component is refused above), but kept so a path that escapes by
        /// any other means is still refused rather than read.
        fn escapes(&self, path: &ProjectPath) -> bool {
            if self.overlay.get(path).is_some() {
                return false;
            }
            match std::fs::canonicalize(path.to_os_path(self.root)) {
                Ok(canon) => !canon.starts_with(&self.canonical_root),
                Err(_) => false,
            }
        }

        /// Refuses, then charges and loads `path` of `kind`. The symlink
        /// check runs here -- immediately before any `metadata`/`read` of the
        /// same path, rather than in a separate earlier pass -- so a symlink
        /// swapped in after `exists()` is still refused instead of followed;
        /// only the microseconds between this check and the read below remain
        /// (a fully atomic check-and-read would need the fd-rooted primitive
        /// live `project-files` reads through). A `Graphic` target stops at
        /// `metadata()` -- a `stat`, never an `open` -- and is charged
        /// nothing; every other kind is read in full and charged, exactly as
        /// discovery has always done for a `.tex`/`.bib` candidate, because
        /// their content decides what happens next.
        fn load(&mut self, path: &ProjectPath, kind: FileKind) -> Load {
            if kind != FileKind::Graphic {
                if let Some(text) = self.overlay.get(path) {
                    return Load::Overlay(text.to_string());
                }
            }
            if let Some(detail) = self.symlink_refusal(path) {
                return Load::SymlinkRefused(detail);
            }
            let os = path.to_os_path(self.root);
            let len = match std::fs::metadata(&os) {
                Ok(m) => m.len(),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Load::Missing,
                Err(e) => return Load::Error(e.to_string()),
            };
            if kind == FileKind::Graphic {
                return Load::DiskGraphic; // stat only; nothing read, nothing to charge
            }
            if !self.budget.charge(path.as_str(), len) {
                return Load::BudgetExceeded;
            }
            match std::fs::read_to_string(&os) {
                Ok(text) => Load::Disk(text),
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => Load::NotUtf8,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Load::Missing,
                Err(e) => Load::Error(e.to_string()),
            }
        }

        fn diag(&mut self, from: &ProjectPath, r: &Reference, is_error: bool, message: String, code: &str) {
            let sources = vec![crate::display::SourceRange {
                path: std::rc::Rc::from(from.as_str()),
                start_byte: r.span.start,
                end_byte: r.span.end,
            }];
            self.diagnostics.push(if is_error {
                crate::display::Diagnostic::error(code, message, sources)
            } else {
                crate::display::Diagnostic::warning(code, message, sources)
            });
        }

        /// A file whose content is in hand: a disk-read `.tex` file is
        /// forwarded as a document (an overlaid one is not -- the request
        /// already has it); `.tex` text of either source is scanned for
        /// further references; a `.bib` file is neither.
        fn visit_loaded(&mut self, path: &ProjectPath, kind: FileKind, text: String, from_disk: bool) {
            if !self.seen.insert(path.clone()) {
                return;
            }
            if from_disk && kind == FileKind::Tex {
                self.documents.push((path.as_str().to_string(), text.clone()));
            }
            if kind != FileKind::Tex {
                return;
            }
            self.ancestors.push(path.clone());
            for r in scan_references(&text) {
                if self.budget.exceeded.is_some() {
                    break;
                }
                self.follow(path, &r);
            }
            self.ancestors.pop();
        }

        /// Resolves and visits one reference found in `from`'s text.
        fn follow(&mut self, from: &ProjectPath, r: &Reference) {
            if !r.literal {
                return; // needs macro expansion; the compiler's to raise.
            }
            let base = match ProjectPath::normalize(&r.argument) {
                Ok(p) => p,
                Err(e) => {
                    self.diag(from, r, true, format!("\\{}{{{}}}: {e}", r.kind.command(), r.argument), "invalid_path");
                    return;
                }
            };
            let (kind, cands) = candidates(r.kind, &base);
            let Some(target) = cands.iter().find(|c| self.exists(c)).cloned() else {
                return; // no candidate exists; the compiler's "not found" to raise.
            };
            // Refuse-all-symlinks first, so an escaping symlink is reported
            // as a symlink (the live wording) rather than as an escape.
            if let Some(detail) = self.symlink_refusal(&target) {
                self.diag(
                    from,
                    r,
                    true,
                    format!("\\{}{{{}}}: {detail}", r.kind.command(), r.argument),
                    "path_escapes_root",
                );
                return;
            }
            if self.escapes(&target) {
                self.diag(
                    from,
                    r,
                    true,
                    format!("\\{}{{{}}}: {target} is a symlink outside the project root", r.kind.command(), r.argument),
                    "path_escapes_root",
                );
                return;
            }
            if self.ancestors.contains(&target) {
                return; // include cycle; the compiler owns this diagnostic.
            }
            if self.seen.contains(&target) {
                return; // diamond: already discovered.
            }
            if kind == FileKind::Tex && self.ancestors.len() >= MAX_DEPTH {
                self.diag(
                    from,
                    r,
                    true,
                    format!("\\{}{{{}}}: nesting deeper than {MAX_DEPTH} files", r.kind.command(), r.argument),
                    "include_depth",
                );
                return;
            }
            match self.load(&target, kind) {
                Load::Overlay(text) => self.visit_loaded(&target, kind, text, false),
                Load::Disk(text) => self.visit_loaded(&target, kind, text, true),
                Load::DiskGraphic => {
                    self.seen.insert(target);
                }
                // A symlink swapped in between the pre-check above and this
                // read: refused here with the same per-site diagnostic (and,
                // like the pre-check, without marking it seen, so a second
                // reference site still gets its own diagnostic).
                Load::SymlinkRefused(detail) => {
                    self.diag(
                        from,
                        r,
                        true,
                        format!("\\{}{{{}}}: {detail}", r.kind.command(), r.argument),
                        "path_escapes_root",
                    );
                }
                Load::Missing | Load::BudgetExceeded => {}
                Load::NotUtf8 => {
                    self.seen.insert(target.clone());
                    let is_error = kind != FileKind::Bibliography;
                    self.diag(from, r, is_error, format!("{target} is not valid UTF-8"), "not_utf8");
                }
                Load::Error(message) => {
                    self.seen.insert(target.clone());
                    self.diag(from, r, true, format!("{target} could not be read: {message}"), "read_error");
                }
            }
        }
    }

    if !root.is_dir() {
        return Ok((Vec::new(), Vec::new()));
    }
    let Ok(canonical_root) = std::fs::canonicalize(root) else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mut walk = Walk {
        root,
        canonical_root,
        overlay,
        seen: std::collections::BTreeSet::new(),
        ancestors: Vec::new(),
        budget: ReadBudget { documents: 0, bytes: 0, exceeded: None },
        documents: Vec::new(),
        diagnostics: Vec::new(),
    };
    match walk.load(entry, FileKind::Tex) {
        Load::Overlay(text) => walk.visit_loaded(entry, FileKind::Tex, text, false),
        Load::Disk(text) => walk.visit_loaded(entry, FileKind::Tex, text, true),
        // `handle_line_inner` guarantees the entry is always one of the
        // request's own documents, so this always loads from the overlay in
        // practice; a disk fallback that fails simply finds nothing further.
        Load::DiskGraphic | Load::SymlinkRefused(_) | Load::Missing | Load::NotUtf8 | Load::Error(_) | Load::BudgetExceeded => {}
    }
    match walk.budget.exceeded {
        Some(reason) => Err(reason),
        None => Ok((walk.documents, walk.diagnostics)),
    }
}

/// GH-75: completes the `\input`/`\include` closure of `entry` from
/// `root`, returning the documents the request did **not** carry plus the
/// diagnostics discovery raised about references it refused.
///
/// The request's own documents are overlaid on the walk ([`discover_closure`]),
/// so an unsaved buffer is what gets scanned for further includes and is
/// never replaced by the stale bytes on disk; and the walk enforces the
/// project's rooted, symlink-refusing containment rule directly, rather than
/// a second copy of it.
///
/// Nothing here is fatal: with no root, an unusable entry path, or a root that
/// cannot be opened, the compile proceeds on exactly the documents the request
/// sent, which is what it did before this existed.
fn closure_from_disk(
    root: Option<&std::path::Path>,
    entry: &str,
    supplied: &[(String, String)],
) -> (Vec<(String, String)>, Vec<crate::display::Diagnostic>) {
    use flashtex_project_files::graph::Overlay;
    use flashtex_project_files::ProjectPath;

    let none = (Vec::new(), Vec::new());
    let Some(root) = root else { return none };
    let Ok(entry_path) = ProjectPath::normalize(entry) else { return none };
    let mut overlay = Overlay::new();
    for (path, text) in supplied {
        let Ok(normalized) = ProjectPath::normalize(path) else { continue };
        overlay.insert(normalized, text.clone());
    }
    // GH-735: cost the closure while walking it. Over budget, nothing from the
    // walk is kept at all -- the compile falls back to exactly the documents
    // the request sent -- and the reason is an error diagnostic. Never half a
    // closure: a document that quietly typesets with some of its includes
    // missing is worse than one that says why they are.
    match discover_closure(root, &entry_path, &overlay) {
        Ok((documents, diagnostics)) => (
            // `path_is_safe` and the byte cap are the same gates the request's
            // own paths and sizes already passed at discovery time; applied
            // again here as a defensive backstop, since these did not come
            // from the request.
            documents
                .into_iter()
                .filter(|(path, text)| path_is_safe(path) && text.len() <= MAX_CLOSURE_DOCUMENT_BYTES)
                .collect(),
            diagnostics,
        ),
        Err(reason) => (
            Vec::new(),
            vec![crate::display::Diagnostic::error(
                "closure_budget_exceeded",
                format!("{reason}; compiling only the documents the request sent"),
                Vec::new(),
            )],
        ),
    }
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

#[cfg(all(test, unix))]
mod includegraphics_read_tests {
    //! GH-INCLUDEGRAPHICS-READ: `discover_closure` must resolve a
    //! `\includegraphics` target -- exists, inside the root -- without ever
    //! opening it for its bytes.
    //!
    //! A regression test cannot time this reliably (a shared machine makes a
    //! timing threshold a flake generator), so it proves "never opened" a
    //! different way: a file with no read permission. `stat` -- what
    //! `Walk::exists`/`Walk::load`'s size check use -- needs no read
    //! permission on the file itself, only search permission on its
    //! directories, but `fs::read`/`read_to_string` -- what a reintroduced
    //! full read would call -- fails on it with `EACCES`. So a clean result
    //! is possible only if the content was never opened; a regression that
    //! reads it again surfaces as a `read_error` diagnostic, the same one
    //! `vendor/project-files`' `Discovery` raises for a real read failure.
    //!
    //! This calls `closure_from_disk` directly (not the full `handle_line`
    //! reply) so the assertion is about discovery alone: the compiler's own,
    //! separate, legitimate image read (`ImageCache::load` in `floats.rs`,
    //! when a page containing the image is actually laid out) is not on this
    //! path and cannot confound the result either way.

    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A project staged in a per-test temp directory, removed on drop.
    struct Project(std::path::PathBuf);

    impl Project {
        fn new(tag: &str) -> Project {
            let dir = std::env::temp_dir().join(format!("flashtex-gh-includegraphics-read-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("stage a project directory");
            Project(std::fs::canonicalize(&dir).expect("canonicalize the project directory"))
        }

        fn write(&self, path: &str, bytes: &[u8]) -> &Project {
            let full = self.0.join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).expect("stage a subdirectory");
            }
            std::fs::write(&full, bytes).expect("write a project file");
            self
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for Project {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn includegraphics_target_is_stat_only_never_opened() {
        let project = Project::new("stat-only");
        let entry = "\\includegraphics{figures/plot}".to_string();
        project.write("main.tex", entry.as_bytes());
        project.write("figures/plot.png", b"not a real png; its bytes must never be opened by discovery");
        let image = project.path().join("figures/plot.png");
        std::fs::set_permissions(&image, std::fs::Permissions::from_mode(0o200)).expect("make the image write-only (unreadable)");
        // A `stat` needs no read permission on the file itself -- confirm the
        // fixture actually tests what it claims to, independent of this fix.
        assert!(std::fs::metadata(&image).is_ok(), "stat must still work on an unreadable file");
        assert!(std::fs::read(&image).is_err(), "and a real read of it must fail, or this test proves nothing");

        let supplied = vec![("main.tex".to_string(), entry.clone())];
        let (documents, diagnostics) = closure_from_disk(Some(project.path()), "main.tex", &supplied);

        assert!(documents.is_empty(), "no further .tex documents to discover here: {documents:?}");
        assert!(
            diagnostics.iter().all(|d| d.code != "read_error"),
            "discovery must never try to open the graphic's content: {diagnostics:?}"
        );
    }

    /// The same fixture, but the image is reachable only through an on-disk
    /// `\input`, so discovery must actually read a `.tex` file from disk and
    /// scan it (not just walk the overlaid entry) before reaching the
    /// `\includegraphics` reference -- the shape described in the bug report
    /// (a `sections/intro.tex` on disk whose figures are large).
    #[test]
    fn includegraphics_target_reached_through_an_on_disk_include_is_stat_only() {
        let project = Project::new("stat-only-nested");
        let main = "\\input{sections/intro}".to_string();
        let intro = "\\includegraphics{figures/plot}Body.".to_string();
        project.write("main.tex", main.as_bytes()).write("sections/intro.tex", intro.as_bytes());
        project.write("figures/plot.png", b"not a real png; its bytes must never be opened by discovery");
        let image = project.path().join("figures/plot.png");
        std::fs::set_permissions(&image, std::fs::Permissions::from_mode(0o200)).expect("make the image write-only (unreadable)");

        let supplied = vec![("main.tex".to_string(), main.clone())];
        let (documents, diagnostics) = closure_from_disk(Some(project.path()), "main.tex", &supplied);

        assert!(
            documents.iter().any(|(p, t)| p == "sections/intro.tex" && t == &intro),
            "the on-disk include is still discovered and forwarded: {documents:?}"
        );
        assert!(
            diagnostics.iter().all(|d| d.code != "read_error"),
            "discovery must never try to open the graphic's content: {diagnostics:?}"
        );
    }
}
