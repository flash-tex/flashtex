//! runtime-v1 JSON Lines transport.
//!
//! One complete JSON object per line. Replies preserve the request `id`. Unknown
//! protocol versions and unknown types produce an `error` envelope — never a
//! silent success, as the contract requires.

use crate::date::TodayDate;
use crate::diagnostics::{Diagnostic, Severity};
use crate::incremental::Session;
use crate::json::{self, str_, Value};
use crate::layout::Font;
use crate::layout::{LayoutConstraints, Page};
use crate::parser::{ParseOptions, SourceDocument};
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

pub const PROTOCOL_VERSION: i64 = 1;
/// Documented maximum accepted line size. Oversized payloads are rejected.
pub const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;

/// Most document/capability modes kept warm at once. A worker can be asked to
/// compile any number of projects over its lifetime, so the cache is bounded and
/// evicts the least-recently-used mode rather than retaining every one ever seen.
const MAX_WARM_SESSIONS: usize = 8;

/// Keyed by project, entry path, negotiated capabilities **and the request's
/// date**: the date is a compile input, so a warm session must not hand
/// yesterday's pages back to a request made today.
type WarmSessions =
    HashMap<(String, String, AcceptedCapabilities, TodayDate), (u64, Session)>;
static SESSIONS: OnceLock<Mutex<WarmSessions>> = OnceLock::new();
static SESSION_TICK: AtomicU64 = AtomicU64::new(0);

/// One request read without allowing an untrusted line to grow memory without
/// bound. `TooLarge` is returned only after the complete offending line has
/// been consumed, so the caller can safely continue with the next request.
#[derive(Debug, PartialEq, Eq)]
pub enum RequestLine {
    Data(Vec<u8>),
    TooLarge,
}

pub fn read_request_line<R: BufRead>(reader: &mut R) -> io::Result<Option<RequestLine>> {
    let mut line = Vec::new();
    let mut oversized = false;
    let mut saw_input = false;

    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            if !saw_input {
                return Ok(None);
            }
            return Ok(Some(if oversized {
                RequestLine::TooLarge
            } else {
                RequestLine::Data(line)
            }));
        }

        saw_input = true;
        let newline = available.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(available.len(), |index| index + 1);
        let content = newline.map_or(available, |index| &available[..index]);

        if !oversized {
            let remaining = MAX_LINE_BYTES.saturating_sub(line.len());
            if content.len() > remaining {
                oversized = true;
                line.clear();
            } else {
                line.extend_from_slice(content);
            }
        }

        reader.consume(consumed);
        if newline.is_some() {
            return Ok(Some(if oversized {
                RequestLine::TooLarge
            } else {
                RequestLine::Data(line)
            }));
        }
    }
}

pub fn error_envelope(id: &str, code: &str, message: &str) -> Value {
    let mut payload = Value::obj();
    payload.set("code", str_(code));
    payload.set("message", str_(message));
    payload.set(
        "diagnostics",
        Value::Arr(vec![Diagnostic::error(message, None, None).to_json("")]),
    );
    let mut v = Value::obj();
    v.set("protocol_version", Value::Num(PROTOCOL_VERSION as f64));
    v.set("id", str_(id));
    v.set("type", str_("error"));
    v.set("payload", payload);
    v
}

/// Handles one already-delimited request line, including transport bytes that
/// cannot be represented as UTF-8. The worker and tests share this reply path.
pub fn handle_request_bytes(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(line) => handle_line(line),
        Err(_) => json::write(&error_envelope(
            "",
            "invalid_utf8",
            "request line is not valid UTF-8",
        )),
    }
}

/// Project-relative paths only: no absolute paths, no parent traversal.
fn path_is_safe(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') || path.starts_with('\\') {
        return false;
    }
    // Reject Windows-style drive prefixes too.
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        return false;
    }
    !path.split(['/', '\\']).any(|c| c == "..")
}

/// Handles one input line and returns the reply line to write.
pub fn handle_line(line: &str) -> String {
    if line.len() > MAX_LINE_BYTES {
        return json::write(&error_envelope(
            "",
            "payload_too_large",
            &format!("line exceeds the {}-byte limit", MAX_LINE_BYTES),
        ));
    }

    let value = match json::parse(line) {
        Ok(v) => v,
        Err(e) => {
            return json::write(&error_envelope("", "malformed_json", &e.0));
        }
    };

    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    match value.get("protocol_version").and_then(|v| v.as_i64()) {
        Some(PROTOCOL_VERSION) => {}
        Some(other) => {
            return json::write(&error_envelope(
                &id,
                "unsupported_protocol_version",
                &format!(
                    "protocol version {} is not supported; this build speaks version {}",
                    other, PROTOCOL_VERSION
                ),
            ));
        }
        None => {
            return json::write(&error_envelope(
                &id,
                "missing_protocol_version",
                "protocol_version is required",
            ));
        }
    }

    match value.get("type").and_then(|v| v.as_str()) {
        Some("compile") => match value.get("payload") {
            Some(p) => json::write(&compile(&id, p)),
            None => json::write(&error_envelope(
                &id,
                "missing_payload",
                "compile requires a payload",
            )),
        },
        Some(other) => json::write(&error_envelope(
            &id,
            "unsupported_type",
            &format!("message type '{}' is not supported", other),
        )),
        None => json::write(&error_envelope(&id, "missing_type", "type is required")),
    }
}

fn result_envelope(id: &str, payload: Value) -> Value {
    let mut v = Value::obj();
    v.set("protocol_version", Value::Num(PROTOCOL_VERSION as f64));
    v.set("id", str_(id));
    v.set("type", str_("compile_result"));
    v.set("payload", payload);
    v
}

/// `paths` is indexed by `DocumentId`. Each item reports the file its bytes
/// actually live in, so click-to-source navigation opens the right file in a
/// multi-file project instead of always pointing at the entry document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
struct AcceptedCapabilities {
    rules_v1: bool,
    font_hints_v1: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NegotiatedCapabilities {
    request_field_present: bool,
    accepted: Vec<String>,
    enabled: AcceptedCapabilities,
}

/// `payload.date` — the civil date `\today` renders
/// (`protocol/proposals/runtime-v1-request-date.md`).
///
/// This compiler never reads the clock: runtime-v1 requires byte-identical
/// output for byte-identical input, so the caller reads the clock and sends the
/// answer. An **absent** field is the Unix epoch, exactly what this worker has
/// always printed, which is what keeps every existing client and every
/// committed fixture byte-identical. A **malformed** value is an error: a
/// caller that sent a date meant it, and quietly typesetting a different one is
/// the failure this field exists to end.
fn request_date(payload: &Value) -> Result<TodayDate, Diagnostic> {
    let Some(value) = payload.get("date") else {
        return Ok(TodayDate::EPOCH);
    };
    let Some(text) = value.as_str() else {
        return Err(Diagnostic::error(
            "compile payload 'date' must be a string in YYYY-MM-DD form",
            None,
            None,
        ));
    };
    TodayDate::parse_iso(text).map_err(|error| {
        Diagnostic::error(
            format!("compile payload 'date' is invalid ({text:?}): {error}"),
            None,
            None,
        )
    })
}

fn negotiate_layout_capabilities(payload: &Value) -> Result<NegotiatedCapabilities, Diagnostic> {
    let Some(value) = payload.get("layout_capabilities") else {
        return Ok(NegotiatedCapabilities::default());
    };
    let Some(items) = value.as_arr() else {
        return Err(Diagnostic::error(
            "layout_capabilities must be a list",
            None,
            None,
        ));
    };
    if items.len() > 16 {
        return Err(Diagnostic::error(
            "layout_capabilities must contain at most 16 entries",
            None,
            None,
        ));
    }

    let mut seen = HashSet::new();
    let mut negotiated = NegotiatedCapabilities {
        request_field_present: true,
        ..NegotiatedCapabilities::default()
    };
    for item in items {
        let Some(capability) = item.as_str() else {
            return Err(Diagnostic::error(
                "every layout_capabilities entry must be a string",
                None,
                None,
            ));
        };
        if capability.is_empty() {
            return Err(Diagnostic::error(
                "layout_capabilities entries must not be empty",
                None,
                None,
            ));
        }
        if capability.len() > 64 {
            return Err(Diagnostic::error(
                "layout_capabilities entries must be at most 64 UTF-8 bytes",
                None,
                None,
            ));
        }
        if !seen.insert(capability) {
            return Err(Diagnostic::error(
                format!("duplicate layout capability {capability:?}"),
                None,
                None,
            ));
        }

        match capability {
            "rules-v1" => {
                negotiated.enabled.rules_v1 = true;
                negotiated.accepted.push(capability.to_string());
            }
            "font-hints-v1" => {
                negotiated.enabled.font_hints_v1 = true;
                negotiated.accepted.push(capability.to_string());
            }
            _ => {}
        }
    }
    Ok(negotiated)
}

fn add_accepted_capabilities(payload: &mut Value, capabilities: &NegotiatedCapabilities) {
    if capabilities.request_field_present {
        payload.set(
            "layout_capabilities",
            Value::Arr(
                capabilities
                    .accepted
                    .iter()
                    .map(|capability| str_(capability.clone()))
                    .collect(),
            ),
        );
    }
}

#[cfg(test)]
fn font_json(font: Font) -> Value {
    let (family, weight, style) = match font {
        Font::TimesRoman => ("Times-Roman", "normal", "normal"),
        Font::TimesBold => ("Times-Bold", "bold", "normal"),
        Font::TimesItalic => ("Times-Italic", "normal", "italic"),
        Font::TimesBoldItalic => ("Times-BoldItalic", "bold", "italic"),
        Font::Helvetica => ("Helvetica", "normal", "normal"),
        Font::Courier => ("Courier", "normal", "normal"),
        Font::Symbol => ("Symbol", "normal", "normal"),
    };
    debug_assert!(!family.is_empty() && family.len() <= 128);
    debug_assert!(!family.chars().any(char::is_control));
    let mut value = Value::obj();
    value.set("family", str_(family));
    value.set("weight", str_(weight));
    value.set("style", str_(style));
    value
}

fn valid_page_unit(value: f64) -> bool {
    value.is_finite() && value.abs() <= 1_000_000.0
}

fn valid_rule_geometry(x_pt: f64, rule: crate::layout::RuleGeometry) -> bool {
    valid_page_unit(x_pt)
        && valid_page_unit(rule.y_pt)
        && valid_page_unit(rule.width_pt)
        && rule.width_pt > 0.0
        && valid_page_unit(rule.height_pt)
        && rule.height_pt > 0.0
}

/// Renders pages straight to JSON text.
///
/// Byte-for-byte identical to building a `Value` tree and serialising it: keys
/// are emitted in the same sorted order a BTreeMap would produce, and numbers
/// and strings go through the same formatting helpers. The pinned byte-exact
/// fixtures are what prove that, and they fail loudly if this drifts.
///
/// Why bypass the tree: at 500 KB the reply holds tens of thousands of items,
/// each of which was a map with owned String keys that was allocated, filled,
/// serialised and dropped. Serialisation was 61 ms of a 143 ms cold
/// time-to-first-byte.
fn pages_json(pages: &[Page], paths: &[&str], capabilities: &AcceptedCapabilities) -> Value {
    // Rough capacity guess: replies are large and reallocation is the cost.
    let item_count: usize = pages.iter().map(|p| p.items.len()).sum();
    let mut out = String::with_capacity(item_count * 160 + 256);
    out.push('[');
    for (page_index, pg) in pages.iter().enumerate() {
        if page_index > 0 {
            out.push(',');
        }
        // Sorted: height_pt, items, number, width_pt
        out.push_str("{\"height_pt\":");
        json::write_number_into(pg.height_pt, &mut out);
        out.push_str(",\"items\":[");
        for (item_index, it) in pg.items.iter().enumerate() {
            if item_index > 0 {
                out.push(',');
            }
            let negotiated_rule = if capabilities.rules_v1 { it.rule } else { None };
            let path = paths.get(it.span.document.0).copied().unwrap_or("");
            if let Some(rule) = negotiated_rule {
                debug_assert!(valid_rule_geometry(it.x_pt, rule));
                // Sorted: height_pt, kind, source, width_pt, x_pt, y_pt
                out.push_str("{\"height_pt\":");
                json::write_number_into(rule.height_pt, &mut out);
                out.push_str(",\"kind\":\"rule\",\"source\":");
                write_source_into(&mut out, path, it.span.start, it.span.end);
                out.push_str(",\"width_pt\":");
                json::write_number_into(rule.width_pt, &mut out);
                out.push_str(",\"x_pt\":");
                json::write_number_into(it.x_pt, &mut out);
                out.push_str(",\"y_pt\":");
                json::write_number_into(rule.y_pt, &mut out);
                out.push('}');
            } else {
                // Sorted: baseline_y_pt, font?, font_size_pt, kind, source, text, x_pt
                out.push_str("{\"baseline_y_pt\":");
                json::write_number_into(it.baseline_y_pt, &mut out);
                if capabilities.font_hints_v1 {
                    out.push_str(",\"font\":");
                    out.push_str(if crate::lm_math::covers(&it.text) {
                        LM_MATH_FONT_JSON
                    } else if crate::newcm_math::covers(&it.text) {
                        NEWCM_MATH_FONT_JSON
                    } else {
                        font_json_literal(it.font)
                    });
                }
                out.push_str(",\"font_size_pt\":");
                json::write_number_into(it.font_size_pt, &mut out);
                out.push_str(",\"kind\":\"text\",\"source\":");
                write_source_into(&mut out, path, it.span.start, it.span.end);
                out.push_str(",\"text\":");
                json::write_string_into(&it.text, &mut out);
                out.push_str(",\"x_pt\":");
                json::write_number_into(it.x_pt, &mut out);
                out.push('}');
            }
        }
        out.push_str("],\"number\":");
        json::write_number_into(pg.number as f64, &mut out);
        out.push_str(",\"width_pt\":");
        json::write_number_into(pg.width_pt, &mut out);
        out.push('}');
    }
    out.push(']');
    Value::Raw(out)
}

/// `font-hints-v1` family for glyphs bound to `crate::lm_math`.
const LM_MATH_FONT_JSON: &str =
    r#"{"family":"Latin Modern Math","style":"normal","weight":"normal"}"#;

/// `font-hints-v1` family for glyphs bound to `crate::newcm_math`.
const NEWCM_MATH_FONT_JSON: &str =
    r#"{"family":"New Computer Modern Math","style":"normal","weight":"normal"}"#;

fn font_json_literal(font: Font) -> &'static str {
    match font {
        Font::TimesRoman => r#"{"family":"Times-Roman","style":"normal","weight":"normal"}"#,
        Font::TimesBold => r#"{"family":"Times-Bold","style":"normal","weight":"bold"}"#,
        Font::TimesItalic => r#"{"family":"Times-Italic","style":"italic","weight":"normal"}"#,
        Font::TimesBoldItalic => {
            r#"{"family":"Times-BoldItalic","style":"italic","weight":"bold"}"#
        }
        Font::Helvetica => r#"{"family":"Helvetica","style":"normal","weight":"normal"}"#,
        Font::Courier => r#"{"family":"Courier","style":"normal","weight":"normal"}"#,
        Font::Symbol => r#"{"family":"Symbol","style":"normal","weight":"normal"}"#,
    }
}

/// Sorted: end_byte, path, start_byte
fn write_source_into(out: &mut String, path: &str, start: usize, end: usize) {
    out.push_str("{\"end_byte\":");
    json::write_number_into(end as f64, out);
    out.push_str(",\"path\":");
    json::write_string_into(path, out);
    out.push_str(",\"start_byte\":");
    json::write_number_into(start as f64, out);
    out.push('}');
}

fn failed(
    id: &str,
    project_id: &str,
    revision: i64,
    diags: Vec<Diagnostic>,
    path: &str,
    capabilities: Option<&NegotiatedCapabilities>,
) -> Value {
    let mut payload = Value::obj();
    payload.set("project_id", str_(project_id));
    payload.set("revision", Value::Num(revision as f64));
    payload.set("status", str_("failed"));
    payload.set("pages", Value::Arr(Vec::new()));
    payload.set(
        "diagnostics",
        Value::Arr(diags.iter().map(|d| d.to_json(path)).collect()),
    );
    payload.set("pdf_path", Value::Null);
    if let Some(capabilities) = capabilities {
        add_accepted_capabilities(&mut payload, capabilities);
    }
    result_envelope(id, payload)
}

fn compile(id: &str, payload: &Value) -> Value {
    let project_id = payload
        .get("project_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let revision = payload
        .get("revision")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let entry = payload
        .get("entry_path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let capabilities = match negotiate_layout_capabilities(payload) {
        Ok(capabilities) => capabilities,
        Err(diag) => {
            return failed(id, &project_id, revision, vec![diag], &entry, None);
        }
    };
    let parse_options = match request_date(payload) {
        Ok(today) => ParseOptions { today },
        Err(diag) => {
            return failed(
                id,
                &project_id,
                revision,
                vec![diag],
                &entry,
                Some(&capabilities),
            );
        }
    };

    let empty = Vec::new();
    let docs = payload
        .get("documents")
        .and_then(|v| v.as_arr())
        .unwrap_or(&empty);

    // Validate every supplied path before compiling anything.
    for d in docs {
        let p = d.get("path").and_then(|v| v.as_str()).unwrap_or("");
        if !path_is_safe(p) {
            let diag = Diagnostic {
                severity: Severity::Error,
                message: format!("rejected document path '{}': paths must be project-relative with no parent traversal", p),
                span: None,
                recovery: None,
                code: None,
                suggestion: None,
                labels: Vec::new(),
                notes: Vec::new(),
                help: None,
            };
            return failed(
                id,
                &project_id,
                revision,
                vec![diag],
                p,
                Some(&capabilities),
            );
        }
    }
    if !entry.is_empty() && !path_is_safe(&entry) {
            let diag = Diagnostic {
                severity: Severity::Error,
                message: format!(
                    "rejected entry_path '{}': paths must be project-relative with no parent traversal",
                    entry
                ),
                span: None,
                recovery: None,
                code: None,
                suggestion: None,
                labels: Vec::new(),
                notes: Vec::new(),
                help: None,
            };
        return failed(
            id,
            &project_id,
            revision,
            vec![diag],
            &entry,
            Some(&capabilities),
        );
    }

    // Every supplied document participates: \input resolves against this set, and
    // DocumentId indexes it, so span order here defines the identity of a span.
    let project: Vec<(String, String)> = docs
        .iter()
        .map(|d| {
            (
                d.get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                d.get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            )
        })
        .collect();

    let entry_doc = docs
        .iter()
        .find(|d| d.get("path").and_then(|v| v.as_str()) == Some(entry.as_str()))
        .or_else(|| docs.first());

    let path = match entry_doc {
        Some(d) => d
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        None => {
            let diag = Diagnostic {
                severity: Severity::Error,
                message: "no documents supplied to compile".into(),
                span: None,
                recovery: None,
                code: None,
                suggestion: None,
                labels: Vec::new(),
                notes: Vec::new(),
                help: None,
            };
            return failed(
                id,
                &project_id,
                revision,
                vec![diag],
                &entry,
                Some(&capabilities),
            );
        }
    };

    // Session state is internal to runtime-v1: request and response shapes stay
    // unchanged. Project plus entry path identifies a document across revisions.
    let sessions = SESSIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut sessions = sessions
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let tick = SESSION_TICK.fetch_add(1, Ordering::Relaxed);
    let key = (
        project_id.clone(),
        path.clone(),
        capabilities.enabled,
        parse_options.today,
    );
    if !sessions.contains_key(&key) && sessions.len() >= MAX_WARM_SESSIONS {
        // Evict the least recently used document. Dropping a session only costs
        // the next compile of that document its reuse; it never changes output,
        // because every result is equivalent to a clean build by construction.
        if let Some(oldest) = sessions
            .iter()
            .min_by_key(|(_, (used, _))| *used)
            .map(|(k, _)| k.clone())
        {
            sessions.remove(&oldest);
        }
    }
    let slot = sessions
        .entry(key)
        .or_insert_with(|| (tick, Session::new()));
    slot.0 = tick;
    let sources: Vec<SourceDocument<'_>> = project
        .iter()
        .map(|(p, t)| SourceDocument {
            path: p.as_str(),
            text: t.as_str(),
        })
        .collect();
    let incremental = slot.1.compile_project_with(
        &sources,
        &path,
        LayoutConstraints::default(),
        &parse_options,
    );
    let pages = incremental.output.pages;
    let mut diags = incremental.output.diagnostics;

    if capabilities.enabled.rules_v1 {
        for page in &pages {
            for item in &page.items {
                if let Some(rule) = item.rule {
                    if !valid_rule_geometry(item.x_pt, rule) {
                        let diag = Diagnostic::error(
                            "fraction rule geometry exceeds the runtime-v1 page-unit bounds",
                            Some(item.span),
                            None,
                        );
                        let source_path = project
                            .get(item.span.document.0)
                            .map(|(path, _)| path.as_str())
                            .unwrap_or(path.as_str());
                        return failed(
                            id,
                            &project_id,
                            revision,
                            vec![diag],
                            source_path,
                            Some(&capabilities),
                        );
                    }
                }
            }
        }
    }

    // Status describes this compilation. Export warnings are added afterwards so
    // a perfectly valid document containing a fraction is not downgraded to
    // "recovered" for a limitation of a different pipeline stage.
    let has_content = pages.iter().any(|p| !p.items.is_empty());
    let status = if diags.is_empty() {
        "ok"
    } else if has_content {
        "recovered"
    } else {
        "failed"
    };

    // Tell the author before they export, not after. Issue #9: the PDF path uses
    // the base-14 fonts, so a glyph outside that repertoire becomes a question
    // mark in the exported file. Silently substituting is what rev 4 forbids.
    let mut offenders: Vec<char> = Vec::new();
    let mut first_span = None;
    let mut first_lm_math_span = None;
    for page in &pages {
        for item in &page.items {
            if capabilities.enabled.rules_v1 && item.rule.is_some() {
                continue;
            }
            if first_lm_math_span.is_none()
                && item
                    .text
                    .chars()
                    // lm.math has no ASCII glyphs; skip the table scan for them.
                    .any(|c| !c.is_ascii() && crate::lm_math::advance(c).is_some())
            {
                first_lm_math_span = Some(item.span);
            }
            for c in crate::export::unrepresentable(&item.text) {
                if !offenders.contains(&c) {
                    offenders.push(c);
                    if first_span.is_none() {
                        first_span = Some(item.span);
                    }
                }
            }
        }
    }
    for c in &offenders {
        let reason = crate::export::reason(*c).unwrap_or("not representable in the export fonts");
        diags.push(Diagnostic::warning(
            format!(
                "{c:?} (U+{:04X}) will not survive PDF export: {reason}",
                *c as u32
            ),
            first_span,
            Some("the preview shows it correctly; the exported PDF will not".into()),
        ));
    }
    if let Some(span) = first_lm_math_span {
        diags.push(Diagnostic::warning(
            "blackboard bold, \\setminus, \\Longrightarrow and other amssymb/latexsym symbols \
             with no base-14 glyph use Latin Modern Math glyphs (unicode-math design); their \
             widths differ from pdfLaTeX's msbm10/cmsy10",
            Some(span),
            Some("drew the real glyphs; this is not pixel parity with pdfLaTeX".into()),
        ));
    }
    let paths: Vec<&str> = project.iter().map(|(p, _)| p.as_str()).collect();

    // Bound the reply so it cannot exceed the consumer's transport frame.
    //
    // Issue #21: a valid 500 KB project produced a 13.4 MB reply, and the
    // runtime rejected it as "malformed/truncated/oversized". The author saw
    // corruption when the real cause was size, and the compiler had said nothing.
    //
    // Pages are dropped only from the end, and only with an explicit diagnostic
    // naming how many and why, because the issue is clear that silently skipped
    // pages are not acceptable. Partial output plus an explicit diagnostic is the
    // same contract the compiler already honours for malformed input.
    let (kept_pages, dropped) = bound_pages(&pages, &paths, &capabilities.enabled);
    if dropped > 0 {
        diags.push(Diagnostic::error(
            format!(
                "document produces more positioned output than the {} MiB transport frame allows; \
                 {dropped} of {} pages were not delivered",
                MAX_RESULT_BYTES / (1024 * 1024),
                pages.len()
            ),
            None,
            Some(format!(
                "delivered the first {} pages; the rest are compiled but undeliverable until \
                 chunked or compact transport exists",
                pages.len() - dropped
            )),
        ));
    }

    let mut p = Value::obj();
    p.set("project_id", str_(project_id));
    p.set("revision", Value::Num(revision as f64));
    p.set(
        "status",
        str_(if dropped > 0 { "recovered" } else { status }),
    );
    p.set(
        "pages",
        pages_json(&kept_pages, &paths, &capabilities.enabled),
    );
    p.set(
        "diagnostics",
        Value::Arr(diags.iter().map(|d| d.to_json_with_paths(&paths)).collect()),
    );
    p.set("pdf_path", Value::Null);
    add_accepted_capabilities(&mut p, &capabilities);
    result_envelope(id, p)
}

/// Largest reply this compiler will emit, matching the documented runtime frame.
pub const MAX_RESULT_BYTES: usize = 8 * 1024 * 1024;

/// Keeps the leading pages that fit within [`MAX_RESULT_BYTES`], returning them
/// and how many were dropped.
///
/// Measures the serialised size of each page rather than guessing from item
/// counts, because item cost varies by an order of magnitude between a heading
/// and a dense math page.
fn bound_pages(
    pages: &[Page],
    paths: &[&str],
    capabilities: &AcceptedCapabilities,
) -> (Vec<Page>, usize) {
    // Reserve room for the envelope, diagnostics and the capability echo.
    let budget = MAX_RESULT_BYTES.saturating_sub(64 * 1024);
    let mut used = 0usize;
    let mut kept: Vec<Page> = Vec::new();
    for page in pages {
        let cost = json::write(&pages_json(std::slice::from_ref(page), paths, capabilities)).len();
        if used + cost > budget && !kept.is_empty() {
            let dropped = pages.len() - kept.len();
            return (kept, dropped);
        }
        used += cost;
        kept.push(page.clone());
    }
    (kept, 0)
}

#[cfg(test)]
mod font_literal_tests {
    use super::*;

    /// The literal and the structured form must stay byte-identical. If someone
    /// changes font_json, this fails rather than silently changing wire output.
    #[test]
    fn every_font_literal_matches_its_serialised_value() {
        for font in [
            Font::TimesRoman,
            Font::TimesBold,
            Font::TimesItalic,
            Font::TimesBoldItalic,
            Font::Helvetica,
            Font::Courier,
            Font::Symbol,
        ] {
            assert_eq!(
                font_json_literal(font),
                json::write(&font_json(font)),
                "{font:?} literal drifted from its serialised form"
            );
        }
    }

    #[test]
    fn latin_modern_math_literal_matches_its_serialised_value() {
        let mut value = Value::obj();
        value.set("family", str_(crate::lm_math::FAMILY));
        value.set("weight", str_("normal"));
        value.set("style", str_("normal"));
        assert_eq!(LM_MATH_FONT_JSON, json::write(&value));
        value.set("family", str_(crate::newcm_math::FAMILY));
        assert_eq!(NEWCM_MATH_FONT_JSON, json::write(&value));
    }
}
