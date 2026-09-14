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
    let sources: Vec<SourceDocument<'_>> = project
        .iter()
        .map(|(p, t)| SourceDocument {
            path: p.as_str(),
            text: t.as_str(),
        })
        .collect();
    // PROPOSAL (FT-063): an optional absolute `project_root` directory the
    // request's `\includegraphics` files are read from (rooted, no symlinks,
    // through project-files). A relative or empty value is ignored.
    let request_root = payload
        .get("project_root")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute());
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
    let mut v1 = crate::v1::fallback(&rendered.v2, caps, accepted.clone());
    // display-list-v2: the envelope is serialised first because declining it
    // (over the line limit) changes the echoed capabilities and diagnostics
    // of the compile_result that precedes it.
    let mut extra_lines = Vec::new();
    let drop_cap = |v1: &mut crate::v1::V1Payload, cap: &str| {
        v1.accepted = v1.accepted.take().map(|a| a.into_iter().filter(|c| c != cap).collect());
    };
    if caps.display_list && v1.status != "failed" {
        let wire = crate::display::Wire { images: caps.images, device_color: caps.device_color };
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
            let estimate = rendered.v2.estimated_json_bytes();
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
                    drop_cap(&mut v1, crate::v1::CAP_DISPLAY_LIST);
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
    let line = v1.write_envelope(&id);
    if line.len() > limit {
        let pages = rendered.v2.pages.len();
        return Reply {
            line: json::write(&failed(
                &id,
                &project_id,
                revision,
                &format!(
                    "compile_result would be {} bytes for {pages} pages, over the {limit}-byte reply limit; split the project or compile fewer pages",
                    line.len(),
                ),
                accepted.map(|a| a.into_iter().filter(|c| c != crate::v1::CAP_DISPLAY_LIST).collect()),
            )),
            extra_lines: Vec::new(),
            rendered: Some(rendered),
            id,
        };
    }
    Reply {
        line,
        extra_lines,
        rendered: Some(rendered),
        id,
    }
}
