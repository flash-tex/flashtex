//! Compile a hypothetical reviewed edit without changing the editor or journal.
//! The caller configures the original FlashTeX compiler executable explicitly.
use crate::{
    digest, identifier, range, relative_path, BridgeError, Document, PreparedEdit, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const MAX_WIRE: usize = 8 * 1024 * 1024;
const MAX_STDERR: u64 = 64 * 1024;
const MAX_SAFE_INTEGER: u64 = (1u64 << 53) - 1;

// --- LaTeX content safety --------------------------------------------------
//
// `proposal.latex` is model output: untrusted text that is (a) sent to a real
// compiler process by `CompilerValidator::validate` below, before any human
// has approved anything, and (b) spliced verbatim into the user's own source
// file once a human does approve it. TeX is a full programming language with
// file I/O and a shell escape, so this text is code flowing into a
// code-execution context, not just displayed content.
//
// This module does not attempt to decide whether a snippet "means" something
// safe — sanitizing an arbitrary, Turing-complete language by pattern
// matching can never be complete or sound. Instead it recognizes a small,
// explicit denylist of named control sequences that are independently known
// to grant filesystem access, shell execution, or the power to change how
// the rest of the document is parsed (catcodes, primitive/macro
// redefinition), plus a few purely structural invariants — balanced
// grouping, no premature `\end{document}`, no bidi/invisible characters that
// make the reviewed text display differently than it compiles — that have no
// legitimate use in a small inserted body snippet. Anything not on this list
// passes through unchanged; a human still reviews the literal `latex` text
// (and any `advisories` this scan adds) before `prepare_insert` is approved.
//
// Scanning walks `latex` once as a flat `Vec<char>` with plain counters, not
// recursion, so a maliciously deep `{{{{...` cannot overflow this scanner's
// own stack regardless of nesting depth.

/// Control words with no legitimate use in an inserted body snippet: shell
/// escape (`write`/`immediate`), file I/O (`input`/`include`/`openin`/
/// `openout`/`closein`/`closeout`/`read`), anything that changes how
/// subsequent characters or names are read or expand (`catcode`, the
/// `\def`-family, `\let`, `\csname`), the `@`-namespace unlock that reaches
/// internal kernel commands (`\makeatletter`/`\makeatother`), and the
/// top-level document declarations (`\documentclass`/`\usepackage`) that a
/// body-only edit has no business emitting (packages belong in
/// `required_dependencies` for a human to add).
const DENIED_CONTROL_WORDS: &[&str] = &[
    "write",
    "immediate",
    "input",
    "include",
    "openin",
    "openout",
    "closein",
    "closeout",
    "read",
    "catcode",
    "def",
    "edef",
    "gdef",
    "xdef",
    "let",
    "futurelet",
    "chardef",
    "mathchardef",
    "countdef",
    "dimendef",
    "skipdef",
    "muskipdef",
    "toksdef",
    "csname",
    "endcsname",
    "makeatletter",
    "makeatother",
    "documentclass",
    "usepackage",
];
/// Control words that are only ever a hang risk (not file/shell access) and
/// are occasionally legitimate in hand-written macros, so they are surfaced
/// to the human reviewer via `ambiguities` instead of being hard-rejected.
const SOFT_CONTROL_WORDS: &[&str] = &["loop", "repeat"];
/// Above this depth, flag for human review: implausible for ordinary math
/// but not yet clearly abusive.
const SOFT_GROUP_DEPTH: usize = 20;
/// Above this depth, reject outright: no legitimate inserted snippet nests
/// this deep, and TeX engines themselves have finite save-stack/input-stack
/// capacity that this size is meant to stay well clear of.
const HARD_GROUP_DEPTH: usize = 2000;

/// A single explicit-denylist/structural safety pass over a proposed LaTeX
/// snippet. See the module-level comment above for the threat model and
/// scope of what this is (and is not) meant to catch.
pub struct LatexScan {
    pub advisories: Vec<String>,
}
pub fn scan_latex(latex: &str) -> Result<LatexScan> {
    fn unsafe_char(c: char) -> bool {
        // Raw control characters (NUL is already rejected earlier, but this
        // is defense in depth) have no place in LaTeX source. Bidi
        // direction-override/isolate controls and the explicit
        // left-to-right/right-to-left marks let inserted text *display* in
        // a different order than it compiles in (the "Trojan Source" class
        // of attack) and are never needed to transcribe a figure or
        // formula.
        (c.is_control() && !matches!(c, '\t' | '\n' | '\r'))
            || matches!(
                c,
                '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}'
            )
    }
    fn invisible_char(c: char) -> bool {
        // Zero-width/joiner characters and a stray BOM are not direction
        // spoofing, but a body snippet from an image-to-LaTeX conversion has
        // no legitimate reason to contain invisible formatting characters
        // either; flag rather than silently drop them.
        matches!(
            c,
            '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
        )
    }
    fn invalid(message: impl Into<String>) -> BridgeError {
        BridgeError::new("invalid_proposal", message)
    }

    let chars: Vec<char> = latex.chars().collect();
    let mut advisories = Vec::new();
    let mut seen_soft_words = std::collections::BTreeSet::new();
    let mut saw_invisible = false;
    let mut depth: usize = 0;
    let mut max_depth: usize = 0;
    let mut envs: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if unsafe_char(c) {
            return Err(invalid(format!(
                "LaTeX contains a disallowed control or bidi-override character (U+{:04X})",
                c as u32
            )));
        }
        if invisible_char(c) {
            saw_invisible = true;
            i += 1;
            continue;
        }
        match c {
            '\\' => {
                i += 1;
                let Some(&next) = chars.get(i) else {
                    return Err(invalid("LaTeX ends with a dangling backslash"));
                };
                if next.is_ascii_alphabetic() {
                    let start = i;
                    while chars.get(i).is_some_and(char::is_ascii_alphabetic) {
                        i += 1;
                    }
                    let word: String = chars[start..i].iter().collect();
                    if DENIED_CONTROL_WORDS.contains(&word.as_str()) {
                        return Err(invalid(format!(
                            "LaTeX uses \\{word}, which is not permitted in an inserted snippet"
                        )));
                    }
                    if SOFT_CONTROL_WORDS.contains(&word.as_str()) {
                        seen_soft_words.insert(word.clone());
                    }
                    if word == "begin" || word == "end" {
                        // Resolve `\begin{name}` / `\end{name}` by looking
                        // past optional spaces/tabs for a brace-delimited
                        // name, without introducing state carried across
                        // loop iterations.
                        let mut j = i;
                        while matches!(chars.get(j), Some(' ') | Some('\t')) {
                            j += 1;
                        }
                        if chars.get(j) == Some(&'{') {
                            let name_start = j + 1;
                            let mut k = name_start;
                            while chars.get(k).is_some_and(|c| !matches!(c, '}' | '{' | '\\')) {
                                k += 1;
                            }
                            if chars.get(k) == Some(&'}') {
                                let name: String = chars[name_start..k].iter().collect();
                                i = k + 1;
                                if name == "document" {
                                    return Err(invalid(format!(
                                        "LaTeX contains \\{word}{{document}}, which would truncate or corrupt the surrounding document"
                                    )));
                                }
                                if word == "begin" {
                                    envs.push(name);
                                } else {
                                    match envs.pop() {
                                        Some(top) if top == name => {}
                                        _ => {
                                            return Err(invalid(format!(
                                            "LaTeX has \\end{{{name}}} with no matching \\begin{{{name}}} in this snippet"
                                        )))
                                        }
                                    }
                                }
                                continue;
                            }
                        }
                    }
                    continue;
                }
                // Control symbol: exactly one non-letter character, which is
                // a literal (e.g. `\{`, `\}`, `\\`, `\%`), not grouping or a
                // comment marker.
                i += 1;
            }
            '{' => {
                depth += 1;
                max_depth = max_depth.max(depth);
                if depth > HARD_GROUP_DEPTH {
                    return Err(invalid(format!(
                        "LaTeX nests groups {depth} deep, far beyond any legitimate snippet (possible resource-exhaustion attempt)"
                    )));
                }
                i += 1;
            }
            '}' => {
                if depth == 0 {
                    return Err(invalid("LaTeX has an unmatched closing brace `}`"));
                }
                depth -= 1;
                i += 1;
            }
            '%' => {
                // Default-catcode line comment. Sound only because `\catcode`
                // itself is denied above, so `%` cannot have been redefined
                // by this same snippet before this point.
                while chars.get(i).is_some_and(|&c| c != '\n') {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    if depth != 0 {
        return Err(invalid(format!(
            "LaTeX has {depth} unclosed group(s) (`{{` without a matching `}}`)"
        )));
    }
    if let Some(name) = envs.last() {
        return Err(invalid(format!(
            "LaTeX has an unclosed \\begin{{{name}}} with no matching \\end{{{name}}}"
        )));
    }
    if max_depth > SOFT_GROUP_DEPTH {
        advisories.push(format!(
            "LaTeX nests grouping {max_depth} levels deep; verify this is intentional before approving"
        ));
    }
    for word in seen_soft_words {
        advisories.push(format!(
            "LaTeX uses \\{word}; verify it has a genuine terminating condition before approving (an infinite loop will hang compilation)"
        ));
    }
    if saw_invisible {
        advisories.push(
            "LaTeX contains invisible/zero-width Unicode characters; verify they are intentional before approving".into(),
        );
    }
    Ok(LatexScan { advisories })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotIdentity {
    pub path: String,
    pub revision: u64,
    pub source_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewEvidence {
    pub request_id: String,
    pub project_id: String,
    pub revision: u64,
    pub snapshots: Vec<SnapshotIdentity>,
    pub original_snapshots: Vec<SnapshotIdentity>,
    pub status: String,
    pub diagnostics: Vec<Value>,
    pub pages: Vec<Value>,
    pub source_is_hypothetical: bool,
    pub compiler_compatibility: String,
}

/// Contains immutable source copies; constructing or validating it does not
/// prepare, approve, persist, apply or confirm any native editor transaction.
pub struct ProposedCompilation {
    id: String,
    project: String,
    revision: u64,
    entry: String,
    documents: BTreeMap<String, Document>,
    originals: Vec<SnapshotIdentity>,
}
impl ProposedCompilation {
    pub fn new(id: &str, entry: &str, documents: &[Document], edit: &PreparedEdit) -> Result<Self> {
        identifier(id)?;
        relative_path(entry)?;
        if edit.expected_revision > MAX_SAFE_INTEGER {
            return Err(BridgeError::new(
                "validation_revision",
                "Compiler protocol requires exactly representable revisions",
            ));
        }
        let mut snapshots = BTreeMap::new();
        let mut source_bytes = 0usize;
        let mut originals = Vec::new();
        for doc in documents {
            source_bytes = source_bytes.saturating_add(doc.text.len());
            if source_bytes > MAX_WIRE || snapshots.len() >= 1024 {
                return Err(BridgeError::new(
                    "validation_too_large",
                    "Project snapshot exceeds validation limits",
                ));
            }
            relative_path(&doc.path)?;
            originals.push(SnapshotIdentity {
                path: doc.path.clone(),
                revision: doc.revision,
                source_sha256: digest(doc.text.as_bytes()),
            });
            if doc.project_id != edit.project_id
                || snapshots.insert(doc.path.clone(), doc.clone()).is_some()
            {
                return Err(BridgeError::new(
                    "validation_snapshot",
                    "Supply unique snapshots from exactly one project",
                ));
            }
        }
        if !snapshots.contains_key(entry) {
            return Err(BridgeError::new(
                "validation_entry_missing",
                "The project entry snapshot is required",
            ));
        }
        let doc = snapshots.get_mut(&edit.path).ok_or_else(|| {
            BridgeError::new("validation_target_missing", "Target snapshot is required")
        })?;
        range(&doc.text, edit.start_byte, edit.end_byte)?;
        if doc.revision != edit.expected_revision
            || digest(doc.text.as_bytes()) != edit.document_before_sha256
            || doc.text[edit.start_byte..edit.end_byte] != edit.removed_text
        {
            return Err(BridgeError::new(
                "validation_stale_edit",
                "Source no longer matches the reviewed edit",
            ));
        }
        doc.text
            .replace_range(edit.start_byte..edit.end_byte, &edit.replacement);
        let result = Self {
            originals,
            id: id.into(),
            project: edit.project_id.clone(),
            revision: edit.expected_revision,
            entry: entry.into(),
            documents: snapshots,
        };
        result.request_bytes()?;
        Ok(result)
    }
    pub fn request_bytes(&self) -> Result<Vec<u8>> {
        let docs: Vec<_> = self
            .documents
            .values()
            .map(|d| json!({"path":d.path,"text":d.text}))
            .collect();
        let mut bytes = serde_json::to_vec(
            &json!({"protocol_version":1,"id":self.id,"type":"compile",
            "payload":{"project_id":self.project,"revision":self.revision,"entry_path":self.entry,"documents":docs}}),
        )?;
        bytes.push(b'\n');
        if bytes.len() > MAX_WIRE {
            return Err(BridgeError::new(
                "validation_too_large",
                "Hypothetical compile request exceeds 8 MiB",
            ));
        }
        Ok(bytes)
    }
    fn source(&self, value: &Value) -> Result<()> {
        if value.is_null() {
            return Ok(());
        }
        let path = value
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("Missing source path"))?;
        let doc = self
            .documents
            .get(path)
            .ok_or_else(|| invalid("Source refers to an unknown snapshot"))?;
        let start = value
            .get("start_byte")
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| invalid("Invalid source start"))?;
        let end = value
            .get("end_byte")
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| invalid("Invalid source end"))?;
        range(&doc.text, start, end)
            .map_err(|_| invalid("Compiler source range is not a valid UTF-8 span"))
    }
    pub fn response(&self, bytes: &[u8]) -> Result<ReviewEvidence> {
        if bytes.len() > MAX_WIRE {
            return Err(invalid("Compiler response exceeds limit"));
        }
        let response: Value = serde_json::from_slice(bytes)
            .map_err(|_| invalid("Compiler response is not one JSON object"))?;
        if response["protocol_version"] != 1
            || response["id"] != self.id
            || response["type"] != "compile_result"
        {
            return Err(invalid(
                "Compiler response ID/version/type does not match the request",
            ));
        }
        let p = &response["payload"];
        if p["project_id"] != self.project || p["revision"].as_u64() != Some(self.revision) {
            return Err(invalid("Compiler returned another project or revision"));
        }
        let status = p["status"]
            .as_str()
            .filter(|s| matches!(*s, "ok" | "recovered" | "failed"))
            .ok_or_else(|| invalid("Invalid compiler status"))?;
        let diagnostics = p["diagnostics"]
            .as_array()
            .ok_or_else(|| invalid("Missing compiler diagnostics"))?;
        for d in diagnostics {
            if !matches!(d["severity"].as_str(), Some("error" | "warning"))
                || !d["message"].is_string()
                || d.get("source").is_none()
                || !(d["recovery"].is_null() || d["recovery"].is_string())
            {
                return Err(invalid("Invalid compiler diagnostic"));
            }
            self.source(&d["source"])?;
        }
        let pages = p["pages"]
            .as_array()
            .ok_or_else(|| invalid("Missing compiler pages"))?;
        for (index, page) in pages.iter().enumerate() {
            if page["number"].as_u64() != Some(index as u64 + 1) {
                return Err(invalid("Invalid page sequence"));
            }
            for key in ["width_pt", "height_pt"] {
                positive(&page[key])?;
            }
            for item in page["items"]
                .as_array()
                .ok_or_else(|| invalid("Missing page items"))?
            {
                if item["kind"] != "text" || !item["text"].is_string() {
                    return Err(invalid("Unsupported compiler display primitive"));
                }
                positive(&item["font_size_pt"])?;
                for key in ["x_pt", "baseline_y_pt"] {
                    finite(&item[key])?;
                }
                if item.get("source").is_none() {
                    return Err(invalid("Missing item source"));
                }
                self.source(&item["source"])?;
            }
        }
        Ok(ReviewEvidence { request_id:self.id.clone(),project_id:self.project.clone(),revision:self.revision,
            snapshots:self.documents.values().map(|d|SnapshotIdentity { path:d.path.clone(),revision:d.revision,source_sha256:digest(d.text.as_bytes()) }).collect(),
            original_snapshots:self.originals.clone(),
            status:status.into(),diagnostics:diagnostics.clone(),pages:pages.clone(),source_is_hypothetical:true,
            compiler_compatibility:"Evidence reflects this compiler's reported subset; success does not prove full LaTeX or multi-file compatibility".into() })
    }
}
fn invalid(message: &str) -> BridgeError {
    BridgeError::new("validation_invalid_response", message)
}
fn finite(v: &Value) -> Result<f64> {
    v.as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| invalid("Non-finite or missing geometry"))
}
fn positive(v: &Value) -> Result<()> {
    if finite(v)? > 0.0 {
        Ok(())
    } else {
        Err(invalid("Nonpositive geometry"))
    }
}

pub struct CompilerValidator {
    pub executable: PathBuf,
    pub timeout: Duration,
}
impl CompilerValidator {
    /// File-backed IO prevents pipe deadlocks. Output files are polled for limits;
    /// an overproducing child is killed and never parsed beyond the read cap.
    pub fn validate(&self, request: &ProposedCompilation) -> Result<ReviewEvidence> {
        if self.timeout.is_zero() || self.timeout > Duration::from_secs(60) {
            return Err(BridgeError::new(
                "validation_timeout_config",
                "Set a timeout in (0,60] seconds",
            ));
        }
        let mut input = tempfile::tempfile()?;
        input.write_all(&request.request_bytes()?)?;
        input.seek(SeekFrom::Start(0))?;
        let mut output = tempfile::tempfile()?;
        let error = tempfile::tempfile()?;
        let mut child = {
            let mut attempts = 0;
            loop {
                let spawned = Command::new(&self.executable)
                    .stdin(Stdio::from(input.try_clone()?))
                    .stdout(Stdio::from(output.try_clone()?))
                    .stderr(Stdio::from(error.try_clone()?))
                    .spawn();
                match spawned {
                    Ok(child) => break child,
                    // Transient fork/exec race: when another thread of this
                    // process forks while a freshly written executable is still
                    // open for writing, the forked child inherits that write fd
                    // until its own exec, so exec'ing the file here can fail
                    // with ETXTBSY even though the writer already closed it.
                    // The condition clears as soon as that child execs; retry
                    // briefly instead of failing the validation.
                    Err(e)
                        if e.kind() == std::io::ErrorKind::ExecutableFileBusy
                            && attempts < 20 =>
                    {
                        attempts += 1;
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => {
                        return Err(BridgeError::new(
                            "validation_launch",
                            format!(
                                "Could not launch the configured FlashTeX compiler {}: {e}",
                                self.executable.display()
                            ),
                        ));
                    }
                }
            }
        };
        let deadline = Instant::now() + self.timeout;
        let execution = (|| -> Result<()> {
            loop {
                if output.metadata()?.len() > MAX_WIRE as u64
                    || error.metadata()?.len() > MAX_STDERR
                {
                    return Err(BridgeError::new(
                        "validation_output_limit",
                        "Compiler exceeded output limits",
                    ));
                }
                if let Some(status) = child.try_wait()? {
                    return if status.success() {
                        Ok(())
                    } else {
                        Err(BridgeError::new(
                            "validation_compiler_failed",
                            "Compiler process failed; source is unchanged",
                        ))
                    };
                }
                if Instant::now() >= deadline {
                    return Err(BridgeError::new(
                        "validation_timeout",
                        "Compiler validation timed out; source is unchanged",
                    ));
                }
                thread::sleep(Duration::from_millis(5));
            }
        })();
        if execution.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        execution?;
        output.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        Read::take(&mut output, MAX_WIRE as u64 + 1).read_to_end(&mut bytes)?;
        request.response(&bytes)
    }
}

impl crate::Bridge {
    /// Validate an unapproved proposal for display in the review UI. This never
    /// calls prepare_insert, saves a journal record or advances source revisions.
    pub fn validate_capture(
        &self,
        request_id: &str,
        capture_id: &str,
        expected_revision: u64,
        entry_path: &str,
        compiler: &CompilerValidator,
    ) -> Result<ReviewEvidence> {
        let record = self.store.require(capture_id)?;
        if record.rejected || record.applied.is_some() {
            return Err(BridgeError::new(
                "validation_capture_closed",
                "Rejected or applied captures cannot be validated for insertion",
            ));
        }
        let proposal = record.proposal.as_ref().ok_or_else(|| {
            BridgeError::new(
                "proposal_missing",
                "Convert the capture before validating it",
            )
        })?;
        proposal.validate()?;
        self.verify_proposal_context(&record)?;
        let anchor = self.capture_anchor(&record.capture)?;
        let doc = self.document(&anchor.project_id, &anchor.path)?;
        if doc.revision != expected_revision {
            return Err(BridgeError::new(
                "revision_conflict",
                "Validate against the current source revision",
            ));
        }
        let edit = PreparedEdit {
            capture_id: capture_id.into(),
            edit_id: format!("validation-{capture_id}"),
            project_id: anchor.project_id.clone(),
            path: anchor.path.clone(),
            expected_revision,
            start_byte: anchor.start_byte,
            end_byte: anchor.end_byte,
            removed_text: doc.text[anchor.start_byte..anchor.end_byte].into(),
            replacement: proposal.latex.clone(),
            document_before_sha256: digest(doc.text.as_bytes()),
        };
        let documents: Vec<_> = self
            .documents
            .values()
            .filter(|d| d.project_id == anchor.project_id)
            .cloned()
            .collect();
        let correlation = format!("validation-{}", digest(request_id.as_bytes()));
        let request = ProposedCompilation::new(&correlation, entry_path, &documents, &edit)?;
        compiler.validate(&request)
    }
}
