//! Local stdio adapter. Native callers must put pipe IO on a dedicated worker.
use flashtex_document_runtime::{Event, Limits};
use flashtex_edit_ledger::{AppliedReceipt, PreparedEdit, Store};
use flashtex_preview_controller::completed_protocol::{SubmissionBindings, CAPABILITY};
use flashtex_preview_controller::file_project::{DiskState, FileProject};
use flashtex_preview_controller::{ApprovedEdit, Controller, HistoryAction, Update};
use flashtex_project_index::{Category, SearchRequest, SearchTermination, SourceSpan};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, BufRead, BufReader, Read, Write},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
mod optional_output;
mod output_buffer;
mod output_delivery;
mod raw_wire;
mod source_plans;
mod wire;
const MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

const MAX_FRAME: usize = 1024 * 1024;
// Reserve one MiB for typical wrapping metadata; this is not a proof that every
// compiler frame fits after reserialization. OutputBuffer checks the complete JSONL.
const COMPILER_ENVELOPE_RESERVE: usize = 1024 * 1024;
const MAX_COMPILER_FRAME: usize = MAX_OUTPUT_BYTES - COMPILER_ENVELOPE_RESERVE;
fn compiler_limits(config: &Value) -> Result<Limits, String> {
    let mut limits = Limits::default();
    if let Some(value) = config.get("compiler_max_frame_bytes") {
        limits.max_frame = value
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| (128..=MAX_COMPILER_FRAME).contains(n))
            .ok_or("compiler_max_frame_bytes must be 128..15728640")?;
    }
    Ok(limits)
}
// The producer counts JSON bytes; runtime framing also counts the newline.
// Invalid inherited settings have the producer's default semantics. A stricter
// positive setting remains authoritative even if too small for a useful reply.
fn producer_command(path: &str, limits: &Limits, project_root: Option<&str>) -> Command {
    let mut command = producer_command_with_limit(
        path,
        limits,
        std::env::var_os("FLASHTEX_MAX_REPLY_BYTES").as_deref(),
    );
    append_project_root(&mut command, project_root);
    command
}
// FT-063: the producer's default image root (`flashtex-render --project-root`).
// Every compile request also carries the same `payload.project_root`, which is
// authoritative per request; producers that do not know the flag ignore it.
fn append_project_root(command: &mut Command, project_root: Option<&str>) {
    if let Some(root) = project_root {
        command.arg("--project-root").arg(root);
    }
}
fn producer_command_with_limit(
    path: &str,
    limits: &Limits,
    inherited: Option<&std::ffi::OsStr>,
) -> Command {
    let ceiling = limits.max_frame.saturating_sub(1);
    let cap = inherited
        .and_then(|value| value.to_str())
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .map_or(ceiling, |value| value.min(ceiling));
    let mut command = Command::new(path);
    command.env("FLASHTEX_MAX_REPLY_BYTES", cap.to_string());
    command
}
fn string<'a>(v: &'a Value, name: &str) -> Result<&'a str, String> {
    v[name].as_str().ok_or(format!("missing string {name}"))
}
fn number(v: &Value, name: &str) -> Result<u64, String> {
    v[name].as_u64().ok_or(format!("missing integer {name}"))
}
fn metadata_response_mode(payload: &Value) -> Result<bool, String> {
    match payload.get("response_mode") {
        None => Ok(false),
        Some(Value::String(mode)) if mode == "full" => Ok(false),
        Some(Value::String(mode)) if mode == "metadata" => Ok(true),
        _ => Err("response_mode must be full or metadata".into()),
    }
}
fn emit(tx: &output_delivery::Sender, stopped: &AtomicBool, value: Value) {
    emit_with_limit(tx, stopped, value, MAX_OUTPUT_BYTES);
}
fn emit_with_limit(tx: &output_delivery::Sender, stopped: &AtomicBool, value: Value, limit: usize) {
    let bytes = output_buffer::serialize(&value, limit).or_else(|_| {
        let error = failure(
            value["session_id"].as_str().unwrap_or(""),
            value["id"].clone(),
            "response exceeds output limit; source may already be durable",
        );
        output_buffer::serialize(&error, limit)
    });
    let Ok(bytes) = bytes else {
        stopped.store(true, Ordering::SeqCst);
        return;
    };
    if tx.try_send(bytes).is_err() {
        stopped.store(true, Ordering::SeqCst);
    }
}
fn failure(session: &str, id: Value, reason: impl AsRef<str>) -> Value {
    json!({"protocol_version":1,"session_id":session,"id":id,"type":"error","payload":{"message":reason.as_ref()}})
}
fn run(config: Value) -> Result<(), String> {
    let session = string(&config, "session_id")?.to_owned();
    if session.is_empty() || session.len() > 128 {
        return Err("invalid session identity".into());
    }
    let limits = compiler_limits(&config)?;
    let raw_display = match config.get("display_transport") {
        None => false,
        Some(Value::String(mode)) if mode == "value" => false,
        Some(Value::String(mode)) if mode == "raw-prototype" => true,
        _ => return Err("display_transport must be value or raw-prototype".into()),
    };
    let diagnostic_timings = config["diagnostic_timings"].as_bool().unwrap_or(false);
    let project = string(&config, "project_id")?.to_owned();
    let entry = string(&config, "entry_path")?.to_owned();
    let bibliography_paths: Vec<String> = serde_json::from_value(
        config
            .get("bibliography_paths")
            .cloned()
            .unwrap_or(json!([])),
    )
    .map_err(|e| e.to_string())?;
    let (mut controller, file_project) = if config.get("project_root").is_some() {
        if config.get("store_paths").is_some() {
            return Err("choose project_root or store_paths, not both".into());
        }
        let (files, controller) = FileProject::open_with_bibliography(
            std::path::Path::new(string(&config, "project_root")?),
            std::path::Path::new(string(&config, "private_ledger_root")?),
            &project,
            &entry,
            &bibliography_paths,
        )?;
        (controller, Some(files))
    } else {
        let paths = config["store_paths"]
            .as_array()
            .ok_or("store_paths array required")?;
        if paths.is_empty() || paths.len() > 256 {
            return Err("expected 1..256 stores".into());
        }
        let stores = paths
            .iter()
            .map(|p| {
                Store::open(p.as_str().ok_or("store path must be string")?)
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, String>>()?;
        (
            Controller::open_with_bibliography(project, entry, stores, &bibliography_paths)?,
            None,
        )
    };
    let compiler = config
        .get("compiler_path")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if raw_display {
        controller.select_raw_display_prototype()?;
    }
    // File-backed projects forward their canonical root so `\includegraphics`
    // resolves in the producer; store-backed projects send nothing new.
    if let Some(files) = file_project.as_ref() {
        controller.set_project_root(Some(files.root()))?;
    }
    let compiler_error = compiler.as_ref().and_then(|path| {
        let command = producer_command(path, &limits, controller.project_root());
        controller.restart(command, limits.clone()).err()
    });
    let (input_tx, input_rx) = mpsc::sync_channel::<Value>(16);
    let (output_tx, output_rx) = output_delivery::channel_with_diagnostics(8, diagnostic_timings);
    let stopped = Arc::new(AtomicBool::new(false));
    let output_stopped = stopped.clone();
    let output_done = Arc::new(AtomicBool::new(false));
    let writer_done = output_done.clone();
    let writing_since = Arc::new(Mutex::new(None::<(std::time::Instant, Option<u64>)>));
    let writer_clock = writing_since.clone();
    thread::spawn(move || {
        let mut stdout = io::stdout().lock();
        loop {
            let frame = match output_rx.next(Duration::from_millis(2)) {
                Ok(frame) => frame,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
            *writer_clock.lock().unwrap() = Some((std::time::Instant::now(), frame.sequence()));
            frame.trace("write_started");
            if stdout
                .write_all(&frame.bytes)
                .and_then(|_| stdout.flush())
                .is_err()
            {
                frame.trace("write_failed");
                output_stopped.store(true, Ordering::SeqCst);
                break;
            }
            frame.trace("write_finished");
            output_rx.written(&frame);
            *writer_clock.lock().unwrap() = None;
        }
        *writer_clock.lock().unwrap() = None;
        writer_done.store(true, Ordering::SeqCst);
    });
    let reader_output = output_tx.clone();
    let reader_stopped = stopped.clone();
    let reader_session = session.clone();
    thread::spawn(move || {
        let mut stdin = io::stdin().lock();
        loop {
            if reader_stopped.load(Ordering::SeqCst) {
                break;
            }
            let mut frame = Vec::new();
            match stdin
                .by_ref()
                .take(MAX_FRAME as u64 + 1)
                .read_until(b'\n', &mut frame)
            {
                Ok(0) => break,
                Ok(_) if frame.len() <= MAX_FRAME && frame.last() == Some(&b'\n') => {}
                _ => {
                    emit(
                        &reader_output,
                        &reader_stopped,
                        failure(&reader_session, Value::Null, "truncated or oversized input"),
                    );
                    break;
                }
            }
            let request: Value = match serde_json::from_slice(&frame) {
                Ok(value) => value,
                Err(_) => {
                    emit(
                        &reader_output,
                        &reader_stopped,
                        failure(&reader_session, Value::Null, "malformed request JSON"),
                    );
                    continue;
                }
            };
            if let Err(error) = input_tx.try_send(request) {
                match error {
                    mpsc::TrySendError::Full(request) => emit(&reader_output,&reader_stopped,failure(&reader_session,request["id"].clone(),"busy: request not admitted; retry same operation after draining replies")),
                    mpsc::TrySendError::Disconnected(_) => break,
                }
            }
        }
    });
    emit(
        &output_tx,
        &stopped,
        json!({"protocol_version":1,"session_id":session,"id":null,"type":"ready","payload":{"compiler_error":compiler_error,"compiler_max_frame_bytes":limits.max_frame,"helper_max_output_bytes":16*1024*1024}}),
    );
    let mut reviews: BTreeMap<String, PreparedEdit> = BTreeMap::new();
    let mut bindings = SubmissionBindings::default();
    let mut last_display_profile_key = None;
    let mut output_epoch = output_tx.reset_optional();
    let mut request_sequence = Some(0u64);
    while !stopped.load(Ordering::SeqCst) {
        let stalled = writing_since
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|(start, sequence)| {
                (start.elapsed() >= Duration::from_secs(2)).then_some(*sequence)
            });
        if let Some(sequence) = stalled {
            if diagnostic_timings {
                eprintln!(
                    "{}",
                    json!({"phase":"output_watchdog","sequence":sequence,"outcome":"timeout"})
                );
            }
            stopped.store(true, Ordering::SeqCst);
            break;
        }
        match input_rx.recv_timeout(Duration::from_millis(2)) {
            Ok(mut request) => {
                request_sequence = request_sequence.and_then(|sequence| sequence.checked_add(1));
                let diagnostic_started_ms = diagnostic_timings
                    .then(|| output_tx.diagnostic_ms())
                    .flatten();
                let request_started = std::time::Instant::now();
                let id = request["id"].clone();
                let response = if request["protocol_version"] != 1
                    || request["session_id"] != session
                    || !id.as_str().is_some_and(|s| !s.is_empty() && s.len() <= 128)
                {
                    Err("invalid version, session or request identity".into())
                } else {
                    (|| -> Result<Value, String> {
                        let token = request["payload"]
                            .get("source_binding_token")
                            .map(|value| {
                                value
                                    .as_str()
                                    .ok_or("source_binding_token must be a string")
                            })
                            .transpose()?
                            .map(str::to_owned);
                        if let Some(token) = token.as_deref() {
                            SubmissionBindings::validate_token(token)?;
                        }
                        if request["type"] == "configure_display_candidates" {
                            let capability = controller.display_candidate_capability();
                            if request["payload"]["capability"] != capability {
                                return Err("unsupported display candidate capability".into());
                            }
                            let enabled = request["payload"]["enabled"]
                                .as_bool()
                                .ok_or("enabled must be boolean")?;
                            if enabled && request["payload"]["renderer_support_confirmed"] != true {
                                return Err(
                                    "explicit renderer support confirmation required".into()
                                );
                            }
                            let preview_error = controller.configure_display_candidates(enabled)?;
                            output_epoch = output_tx.reset_optional();
                            return Ok(
                                json!({"capability":capability,"enabled":enabled,"preview_error":preview_error}),
                            );
                        }
                        if request["type"] == "configure_completed_snapshots" {
                            if request["payload"]["capability"] != CAPABILITY {
                                return Err("unsupported completed snapshot capability".into());
                            }
                            let enabled = request["payload"]["enabled"]
                                .as_bool()
                                .ok_or("enabled must be boolean")?;
                            controller.configure_completed_snapshots(enabled)?;
                            bindings.configure(enabled)?;
                            output_epoch = output_tx.reset_optional();
                            return Ok(json!({"capability":CAPABILITY,"enabled":enabled}));
                        }
                        if request["type"] == "restart" || request["type"] == "close" {
                            controller.configure_completed_snapshots(false)?;
                            bindings.configure(false)?;
                            output_epoch = output_tx.reset_optional();
                        }
                        let before = controller.compile_revision();
                        let result = handle(
                            &mut controller,
                            &mut reviews,
                            &mut request,
                            compiler.as_deref(),
                            &limits,
                            file_project.as_ref(),
                        );
                        let after = controller.compile_revision();
                        if after != before && bindings.enabled() {
                            if let Some(token) = token.as_deref() {
                                // Synchronous request handling captured this exact admitted generation.
                                // Optional metadata failure must not replace a durable operation's reply.
                                let _ = bindings.record(after, token);
                            }
                        }
                        result
                    })()
                };
                let output = match response {
                    Ok(payload) => wire::envelope(&session, id, "result", payload),
                    Err(reason) => failure(&session, id, reason),
                };
                let handling_ms = request_started.elapsed().as_secs_f64() * 1000.0;
                let serialization_started = std::time::Instant::now();
                emit(&output_tx, &stopped, output);
                if diagnostic_timings {
                    eprintln!(
                        "{}",
                        json!({"phase":"request","sequence":request_sequence,"started_ms":diagnostic_started_ms,"handling_ms":handling_ms,
                        "response_serialization_ms":serialization_started.elapsed().as_secs_f64()*1000.0})
                    );
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let poll_started = std::time::Instant::now();
        let updates = controller.poll();
        let poll_ms = poll_started.elapsed().as_secs_f64() * 1000.0;
        if diagnostic_timings {
            if let Some(profile) = controller.last_display_profile() {
                let key = (
                    profile.request_id.clone(),
                    profile.revision,
                    profile.display_epoch,
                );
                if last_display_profile_key.as_ref() != Some(&key) {
                    eprintln!("{}", json!({"phase":"display_transport","profile":profile}));
                    last_display_profile_key = Some(key);
                }
            } else {
                last_display_profile_key = None;
            }
        }
        // Candidate-only processing and discarded-value destruction may produce
        // no events. Capture slow owner turns without logging every idle poll.
        if diagnostic_timings && (!updates.is_empty() || poll_ms >= 1.0) {
            eprintln!(
                "{}",
                json!({"phase":"compiler_poll","events":updates.len(),
                "duration_ms":poll_ms})
            );
        }
        let historical = controller.take_completed_snapshot().and_then(|snapshot| {
            let token = bindings.take(bindings.epoch(), snapshot.compile_revision());
            if diagnostic_timings && token.is_none() {
                eprintln!(
                    "{}",
                    json!({"phase":"historical_eligibility",
                    "compile_revision":snapshot.compile_revision(),"outcome":"binding_unavailable"})
                );
            }
            token.map(|token| (snapshot, token))
        });
        for update in updates {
            // A negotiated historical frame replaces its legacy stale notification.
            // Do not enqueue that notification ahead of its own optional replacement.
            if matches!(&update, Update::Runtime(Event::Stale { id, .. })
                if historical.as_ref().is_some_and(|(snapshot, _)| snapshot.request_id() == id))
            {
                continue;
            }
            let payload = match update {
                Update::Preview(preview) => {
                    bindings.retire_through(preview.compile_revision);
                    wire::preview_payload(preview)
                }
                Update::Discarded {
                    request_id,
                    compile_revision,
                } => {
                    json!({"kind":"discarded","request_id":request_id,"compile_revision":compile_revision})
                }
                Update::Runtime(event) => match event {
                    Event::Superseded { id, by_id } => {
                        json!({"kind":"superseded","request_id":id,"by_id":by_id})
                    }
                    Event::Stale { id, revision } => {
                        json!({"kind":"stale","request_id":id,"compile_revision":revision})
                    }
                    Event::Cancelled { id } => json!({"kind":"cancelled","request_id":id}),
                    Event::Failed { id, reason } => {
                        json!({"kind":"failed","request_id":id,"reason":reason})
                    }
                    Event::Preview { .. } => unreachable!("controller unwraps previews"),
                },
            };
            emit(
                &output_tx,
                &stopped,
                wire::envelope(&session, Value::Null, "update", payload),
            );
        }
        if let Some((snapshot, token)) = historical {
            let eligible = output_tx.can_offer(output_epoch);
            let claimed = eligible && controller.claim_historical_display(&snapshot);
            if diagnostic_timings && !claimed {
                eprintln!(
                    "{}",
                    json!({"phase":"historical_eligibility",
                    "compile_revision":snapshot.compile_revision(),
                    "outcome":if eligible {"claim_refused"} else {"queue_ineligible"}})
                );
            }
            if claimed {
                let generation = snapshot.compile_revision();
                let mut payload = json!({"kind":"completed_snapshot",
                        "project_id":snapshot.source_versions().project_id,
                        "session_id":session,"source_versions":snapshot.source_versions().documents,
                        "request_id":snapshot.request_id(),"compile_revision":snapshot.compile_revision(),
                        "current_compile_revision":controller.compile_revision(),
                        "is_current":false,"source_actions_enabled":false,"source_binding_token":token});
                payload["result"] = snapshot.into_result();
                let value = wire::envelope(&session, Value::Null, "update", payload);
                let started = std::time::Instant::now();
                let outcome = optional_output::offer_with_generation(
                    &output_tx,
                    output_epoch,
                    &value,
                    MAX_OUTPUT_BYTES,
                    Some(generation),
                );
                if diagnostic_timings {
                    eprintln!(
                        "{}",
                        json!({"phase":"optional_output","kind":"completed_snapshot","compile_revision":generation,
                        "outcome":outcome.label(),"serialization_ms":started.elapsed().as_secs_f64()*1000.0})
                    );
                }
            }
        }
        if output_tx.can_offer(output_epoch) {
            if let Some(payload) = controller.take_current_raw_display_payload() {
                let value = raw_wire::envelope(&session, &payload);
                let started = std::time::Instant::now();
                let outcome =
                    optional_output::offer(&output_tx, output_epoch, &value, MAX_OUTPUT_BYTES);
                if diagnostic_timings {
                    eprintln!(
                        "{}",
                        json!({"phase":"optional_output","kind":"display_candidate",
                        "transport":"raw-prototype","outcome":outcome.label(),
                        "serialization_ms":started.elapsed().as_secs_f64()*1000.0})
                    );
                }
            }
        }
        if output_tx.can_offer(output_epoch) {
            if let Some(payload) = controller.take_current_display_payload() {
                let value = wire::envelope(&session, Value::Null, "update", payload);
                let started = std::time::Instant::now();
                let outcome =
                    optional_output::offer(&output_tx, output_epoch, &value, MAX_OUTPUT_BYTES);
                if diagnostic_timings {
                    eprintln!(
                        "{}",
                        json!({"phase":"optional_output","kind":"display_candidate",
                        "outcome":outcome.label(),"serialization_ms":started.elapsed().as_secs_f64()*1000.0})
                    );
                }
            }
        }
    }
    // Drain normal EOF replies, bounded even if the native reader stopped.
    drop(output_tx);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !output_done.load(Ordering::SeqCst)
        && !stopped.load(Ordering::SeqCst)
        && std::time::Instant::now() < deadline
    {
        thread::sleep(Duration::from_millis(2));
    }
    if stopped.load(Ordering::SeqCst) || !output_done.load(Ordering::SeqCst) {
        return Err(
            "output stalled; delivery uncertain, recover durable source and receipts".into(),
        );
    }
    Ok(())
}
fn source_json(source: &SourceSpan) -> Value {
    json!({"path":source.file,"revision":source.revision,"start_byte":source.start_byte,"end_byte":source.end_byte})
}
struct OwnedEditInput {
    path: String,
    revision: u64,
    sha256: String,
    text: String,
    metadata_only: bool,
}
fn take_edit_input(payload: &mut Value) -> Result<OwnedEditInput, String> {
    let metadata_only = metadata_response_mode(payload)?;
    let path = string(payload, "path")?.to_owned();
    let revision = number(payload, "expected_revision")?;
    let sha256 = string(payload, "expected_sha256")?.to_owned();
    string(payload, "text")?; // Validate every field before consuming owned text.
    let Value::String(text) = payload["text"].take() else {
        unreachable!("validated string")
    };
    Ok(OwnedEditInput {
        path,
        revision,
        sha256,
        text,
        metadata_only,
    })
}
struct OwnedHistoryInput {
    path: String,
    metadata_only: bool,
    action: HistoryAction,
}
fn take_history_input(request: &mut Value) -> Result<OwnedHistoryInput, String> {
    let kind = match string(request, "type")? {
        "apply_group" => 0,
        "undo" => 1,
        "redo" => 2,
        _ => return Err("unsupported history action".into()),
    };
    let payload = &mut request["payload"];
    let metadata_only = metadata_response_mode(payload)?;
    let path = string(payload, "path")?.to_owned();
    let command = payload["command"].take();
    let action = match kind {
        0 => HistoryAction::Group(serde_json::from_value(command).map_err(|e| e.to_string())?),
        1 => HistoryAction::Undo(serde_json::from_value(command).map_err(|e| e.to_string())?),
        _ => HistoryAction::Redo(serde_json::from_value(command).map_err(|e| e.to_string())?),
    };
    Ok(OwnedHistoryInput {
        path,
        metadata_only,
        action,
    })
}
fn handle(
    controller: &mut Controller,
    reviews: &mut BTreeMap<String, PreparedEdit>,
    request: &mut Value,
    compiler: Option<&str>,
    limits: &Limits,
    file_project: Option<&FileProject>,
) -> Result<Value, String> {
    let p = &request["payload"];
    match string(request, "type")? {
        "document" => Ok(json!({"document":controller.document(string(p,"path")?)?})),
        "snapshot" => {
            let snapshot = controller.index().snapshot();
            let document_kinds = snapshot
                .documents
                .keys()
                .map(|path| {
                    let kind = controller
                        .index()
                        .document_kind(&snapshot, path)
                        .map_err(|e| e.to_string())?;
                    Ok((
                        path.clone(),
                        match kind {
                            flashtex_project_index::DocumentKind::Latex => "latex",
                            flashtex_project_index::DocumentKind::Bibliography => "bibliography",
                        },
                    ))
                })
                .collect::<Result<BTreeMap<_, _>, String>>()?;
            Ok(
                json!({"project_id":snapshot.project_id,"source_versions":snapshot.documents,"membership_generation":snapshot.generation,"document_kinds":document_kinds}),
            )
        }
        "plan_literal_replacement" | "plan_citation_rename" | "plan_citation_rename_at" => {
            source_plans::handle(controller.index(), string(request, "type")?, p)
        }
        "search_literal" => {
            let snapshot = controller.index().snapshot();
            if p["source_versions"] != json!(snapshot.documents) {
                return Err("source versions changed; refresh snapshot before searching".into());
            }
            let max_matches =
                usize::try_from(number(p, "max_matches")?).map_err(|_| "invalid match limit")?;
            let max_work =
                usize::try_from(number(p, "max_work")?).map_err(|_| "invalid work limit")?;
            if max_matches == 0 || max_matches > 1000 || max_work == 0 || max_work > 1_000_000 {
                return Err("search requires 1..1000 matches and 1..1000000 work budget".into());
            }
            let mut search = SearchRequest::literal(string(p, "literal")?);
            search.max_matches = max_matches;
            search.max_work = max_work;
            search.documents =
                serde_json::from_value(p.get("documents").cloned().unwrap_or(Value::Null))
                    .map_err(|e| e.to_string())?;
            let result = controller
                .index()
                .search_literal(&snapshot, &search, || false)
                .map_err(|e| e.to_string())?;
            let termination = match result.termination {
                SearchTermination::Complete => "complete",
                SearchTermination::MatchLimit => "match_limit",
                SearchTermination::WorkLimit => "work_limit",
                SearchTermination::Cancelled => "cancelled",
            };
            Ok(
                json!({"source_versions":snapshot.documents,"matches":result.matches.iter().map(source_json).collect::<Vec<_>>(),"termination":termination,"work_used":result.work_used}),
            )
        }
        "complete" | "navigate" => {
            let snapshot = controller.index().snapshot();
            if p["source_versions"] != json!(snapshot.documents) {
                return Err("source versions changed; refresh snapshot before querying".into());
            }
            if request["type"] == "complete" {
                let category = match string(p, "category")? {
                    "label" => Category::Label,
                    "citation" => Category::Citation,
                    "command" => Category::Command,
                    _ => return Err("unknown completion category".into()),
                };
                let limit =
                    usize::try_from(number(p, "limit")?).map_err(|_| "invalid completion limit")?;
                if limit == 0 || limit > 100 {
                    return Err("completion limit must be 1..100".into());
                }
                let results = controller
                    .index()
                    .complete(&snapshot, category, string(p, "prefix")?, limit)
                    .map_err(|e| e.to_string())?;
                let results: Vec<Value> = results.into_iter().map(|item|json!({"name":item.name,"definitions":item.definitions.iter().take(100).map(source_json).collect::<Vec<_>>(),"occurrences":item.occurrences.iter().take(100).map(source_json).collect::<Vec<_>>(),"locations_truncated":item.definitions.len()>100 || item.occurrences.len()>100})).collect();
                Ok(json!({"source_versions":snapshot.documents,"completions":results}))
            } else {
                let offset =
                    usize::try_from(number(p, "byte_offset")?).map_err(|_| "invalid offset")?;
                let navigation = controller
                    .index()
                    .navigate(&snapshot, string(p, "path")?, offset)
                    .map_err(|e| e.to_string())?;
                Ok(
                    json!({"source_versions":snapshot.documents,"navigation":navigation.map(|item|json!({"origin":source_json(&item.origin.source),"name":item.origin.name,"definitions":item.definitions.iter().take(100).map(|symbol|source_json(&symbol.source)).collect::<Vec<_>>(),"definitions_truncated":item.definitions.len()>100}))}),
                )
            }
        }

        "edit" => {
            let edit = take_edit_input(&mut request["payload"])?;
            if edit.metadata_only {
                let result = controller.replace_document_metadata(
                    &edit.path,
                    edit.revision,
                    &edit.sha256,
                    edit.text,
                )?;
                return Ok(
                    json!({"response_mode":"metadata", "document":result.document,
                    "compile_request_id":result.compile_admission.as_ref().map(|a| &a.request_id),"compile_revision":result.compile_admission.as_ref().map(|a| a.compile_revision),"preview_error":result.preview_error,"save_and_submit_ms":result.save_and_submit_ms}),
                );
            }
            let result =
                controller.replace_document(&edit.path, edit.revision, &edit.sha256, edit.text)?;
            Ok(
                json!({"document":result.document,"compile_request_id":result.compile_admission.as_ref().map(|a| &a.request_id),"compile_revision":result.compile_admission.as_ref().map(|a| a.compile_revision),"preview_error":result.preview_error,"save_and_submit_ms":result.save_and_submit_ms}),
            )
        }
        "project_status" => {
            let snapshot = controller.index().snapshot();
            let max = match p.get("max_documents") {
                None => 256,
                Some(value) => value
                    .as_u64()
                    .filter(|n| (1..=256).contains(n))
                    .ok_or("max_documents must be 1..256")? as usize,
            };
            let documents = snapshot.documents.keys().take(max).map(|path| {
                let document = controller.document(path)?;
                Ok(json!({"path":path,"revision":document.revision,"sha256":document.source_sha256,"bytes":document.text.len()}))
            }).collect::<Result<Vec<Value>, String>>()?;
            Ok(
                json!({"project_id":snapshot.project_id,"source_versions":snapshot.documents,
                "membership_generation":snapshot.generation,"documents":documents,
                "total_documents":snapshot.documents.len(),"truncated":snapshot.documents.len()>max,
                "scope":"active_sources_only","disk_tree_enumerated":false}),
            )
        }
        "open_document" | "detach_document" => {
            let expected = controller.index().snapshot();
            if p["source_versions"] != json!(expected.documents)
                || p["membership_generation"].as_u64() != Some(expected.generation)
            {
                return Err("project membership snapshot is stale".into());
            }
            let path = string(p, "path")?;
            let (document, preview_error) = if request["type"] == "open_document" {
                let kind = match p.get("document_kind").and_then(Value::as_str) {
                    None if p.get("document_kind").is_none() => {
                        flashtex_project_index::DocumentKind::Latex
                    }
                    Some("latex") => flashtex_project_index::DocumentKind::Latex,
                    Some("bibliography") => flashtex_project_index::DocumentKind::Bibliography,
                    _ => return Err("document_kind must be latex or bibliography".into()),
                };
                let result = file_project
                    .ok_or("helper was not opened from a file project")?
                    .open_document_with_kind(controller, &expected, path, kind)?;
                (Some(result.document), result.preview_error)
            } else {
                (None, controller.detach_document(&expected, path)?)
            };
            let current = controller.index().snapshot();
            Ok(
                json!({"document":document,"preview_error":preview_error,"source_versions":current.documents,"membership_generation":current.generation}),
            )
        }
        "file_status" => {
            let files = file_project.ok_or("helper was not opened from a file project")?;
            let state = match files.inspect(controller, string(p, "path")?)? {
                DiskState::MatchesSource { sha256 } => {
                    json!({"state":"matches_source","sha256":sha256})
                }
                DiskState::DiffersFromSource {
                    disk_sha256,
                    source_sha256,
                } => {
                    json!({"state":"differs_from_source","disk_sha256":disk_sha256,"source_sha256":source_sha256})
                }
                DiskState::Missing => json!({"state":"missing"}),
                DiskState::Unavailable { reason } => json!({"state":"unavailable","reason":reason}),
            };
            Ok(
                json!({"path":string(p,"path")?,"disk":state,"discovery_diagnostics":files.diagnostics(),"export_available":true}),
            )
        }
        "reload" => {
            if p["user_approved"] != true {
                return Err("explicit reload approval required".into());
            }
            let result = file_project
                .ok_or("helper was not opened from a file project")?
                .reload_explicitly(
                    controller,
                    string(p, "path")?,
                    p["expected_revision"]
                        .as_u64()
                        .ok_or("expected_revision required")?,
                    string(p, "expected_sha256")?,
                    string(p, "expected_disk_sha256")?,
                )?;
            Ok(
                json!({"document":result.document,"preview_error":result.preview_error,"save_and_submit_ms":result.save_and_submit_ms}),
            )
        }
        "export" => {
            let receipt = file_project
                .ok_or("helper was not opened from a file project")?
                .export(
                    controller,
                    string(p, "path")?,
                    p["expected_revision"]
                        .as_u64()
                        .ok_or("expected_revision required")?,
                    string(p, "expected_sha256")?,
                    match p.get("expected_disk_sha256") {
                        Some(Value::Null) => None,
                        Some(Value::String(hash)) => Some(hash.as_str()),
                        _ => {
                            return Err("expected_disk_sha256 must be explicit null or hash".into())
                        }
                    },
                )?;
            Ok(
                json!({"exported":true,"path":receipt.path.as_str(),"sha256":receipt.sha256_hex(),"bytes":receipt.bytes}),
            )
        }
        "history_status" => {
            use flashtex_edit_ledger::history::{
                MAX_HISTORY_BYTES, MAX_HISTORY_COMMAND_IDS, MAX_HISTORY_ENTRIES,
            };
            let path = string(p, "path")?;
            let history = controller.history_status(path)?;
            let document = controller.document(path)?;
            // Both reads occur on the same owner turn; callers can use this exact
            // revision/hash for a later guarded undo, without fetching full text.
            Ok(json!({"history":history,
                "document":{"project_id":document.project_id,"path":document.path,
                    "revision":document.revision,"source_sha256":document.source_sha256},
                "limits":{"history_bytes":MAX_HISTORY_BYTES,"history_entries":MAX_HISTORY_ENTRIES,
                    "permanent_command_ids":MAX_HISTORY_COMMAND_IDS}}))
        }
        "apply_group" | "undo" | "redo" => {
            let input = take_history_input(request)?;
            let is_group = matches!(&input.action, HistoryAction::Group(_));
            let (mut payload, admission) = if input.metadata_only {
                let result = controller.apply_history_metadata(&input.path, input.action)?;
                (
                    json!({"response_mode":"metadata","history":result.history,
                    "preview_error":result.preview_error,"save_and_submit_ms":result.save_and_submit_ms}),
                    result.compile_admission,
                )
            } else {
                let outcome = controller.apply_history(&input.path, input.action)?;
                (
                    json!({"history":outcome.history,"preview_error":outcome.source.preview_error,"save_and_submit_ms":outcome.source.save_and_submit_ms}),
                    outcome.source.compile_admission,
                )
            };
            if is_group {
                payload["compile_request_id"] = json!(admission.as_ref().map(|a| &a.request_id));
                payload["compile_revision"] = json!(admission.as_ref().map(|a| a.compile_revision));
            }
            Ok(payload)
        }
        "configure_layout" => {
            if p["renderer_support_confirmed"] != true {
                return Err("explicit native renderer support confirmation required".into());
            }
            let capabilities: Vec<String> =
                serde_json::from_value(p["layout_capabilities"].clone())
                    .map_err(|e| e.to_string())?;
            controller.configure_layout(capabilities)?;
            Ok(json!({"submitted":true}))
        }
        "compile" => {
            controller.compile_current()?;
            Ok(json!({"submitted":true}))
        }
        "restart" => {
            let command = producer_command(
                compiler.ok_or("compiler not configured")?,
                limits,
                controller.project_root(),
            );
            controller.restart(command, limits.clone())?;
            Ok(json!({"submitted":true}))
        }
        "close" => {
            controller.close()?;
            reviews.clear();
            Ok(json!({"closed":true}))
        }
        "review" => {
            if reviews.len() >= 128 {
                return Err("review capacity reached; retire a review first".into());
            }
            let edit: PreparedEdit =
                serde_json::from_value(p["edit"].clone()).map_err(|e| e.to_string())?;
            let current = controller.document(&edit.path)?;
            if current.project_id != edit.project_id
                || current.revision != edit.expected_revision
                || current.source_sha256 != edit.document_before_sha256
                || current.text.get(edit.start_byte..edit.end_byte)
                    != Some(edit.removed_text.as_str())
            {
                return Err("review differs from current source".into());
            }
            let mut random = [0u8; 32];
            getrandom::fill(&mut random).map_err(|e| e.to_string())?;
            let token = random
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            reviews.insert(token.clone(), edit.clone());
            Ok(json!({"approval_token":token,"edit":edit,"requires_explicit_user_approval":true}))
        }
        "apply_reviewed" => {
            if p["user_approved"] != true {
                return Err("explicit user approval required".into());
            }
            let edit = reviews
                .get(string(p, "approval_token")?)
                .ok_or("unknown review token")?
                .clone();
            let result =
                controller.apply_reviewed(ApprovedEdit::from_explicit_user_approval(edit))?;
            Ok(
                json!({"receipt":result.receipt,"document":result.source.document,"preview_error":result.source.preview_error}),
            )
        }
        "retire_review" => {
            reviews.remove(string(p, "approval_token")?);
            Ok(json!({"retired":true}))
        }
        "recovery" => Ok(json!({"transactions":controller.recovery(string(p,"path")?)?})),
        "confirm_receipt" => {
            let receipt: AppliedReceipt =
                serde_json::from_value(p["receipt"].clone()).map_err(|e| e.to_string())?;
            controller.confirm_receipt(string(p, "path")?, &receipt)?;
            Ok(json!({"confirmed":true}))
        }
        _ => Err("unknown operation".into()),
    }
}
fn main() {
    let result = std::env::args_os()
        .nth(1)
        .ok_or("usage: flashtex-preview-controller CONFIG.json".to_owned())
        .and_then(|path| {
            let file = File::open(path).map_err(|e| e.to_string())?;
            let mut bytes = Vec::new();
            BufReader::new(file)
                .take(MAX_FRAME as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > MAX_FRAME {
                return Err("oversized configuration".into());
            }
            run(serde_json::from_slice(&bytes).map_err(|e| e.to_string())?)
        });
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod configuration_tests {
    use super::*;
    #[test]
    fn owned_history_input_reuses_large_replacement_and_preserves_envelope() {
        let source = json!({"protocol_version":1,"session_id":"s","id":"request",
            "type":"apply_group","payload":{"path":"main.tex","response_mode":"metadata",
            "source_binding_token":"binding","command":{"command_id":"group","expected_revision":7,
            "expected_sha256":"a".repeat(64),"label":"large replacement",
            "edits":[{"start_byte":0,"end_byte":0,"removed_text":"","replacement":"β".repeat(32768)}]}}});
        let mut request: Value =
            serde_json::from_slice(&serde_json::to_vec(&source).unwrap()).unwrap();
        let pointer = request["payload"]["command"]["edits"][0]["replacement"]
            .as_str()
            .unwrap()
            .as_ptr();
        let old_copy = request["payload"]["command"].clone();
        assert_ne!(
            pointer,
            old_copy["edits"][0]["replacement"]
                .as_str()
                .unwrap()
                .as_ptr()
        );
        let input = take_history_input(&mut request).unwrap();
        let HistoryAction::Group(group) = input.action else {
            panic!("wrong action")
        };
        assert_eq!(group.edits[0].replacement.as_ptr(), pointer);
        assert_eq!(group.edits[0].replacement.len(), 65536);
        assert_eq!(serde_json::to_value(&group).unwrap(), old_copy);
        assert_eq!(input.path, "main.tex");
        assert!(input.metadata_only);
        assert!(request["payload"]["command"].is_null());
        assert_eq!(request["payload"]["source_binding_token"], "binding");
        for key in ["protocol_version", "session_id", "id", "type"] {
            assert_eq!(request[key], source[key]);
        }
        eprintln!("history replacement moved in place:65536bytes; prior Value clone retained distinct65536byte text");
        for (field, bad) in [("response_mode", json!(false)), ("path", Value::Null)] {
            let mut invalid = source.clone();
            invalid["payload"][field] = bad;
            let before = invalid.clone();
            assert!(take_history_input(&mut invalid).is_err());
            assert!(invalid == before, "invalid policy/path consumed command");
        }
        for kind in ["undo", "redo"] {
            let mut r = json!({"type":kind,"payload":{"path":"main.tex","command":{
                "command_id":"history","expected_revision":8,"expected_sha256":"b".repeat(64)}}});
            let parsed = take_history_input(&mut r).unwrap();
            assert!(!parsed.metadata_only);
            match (kind, parsed.action) {
                ("undo", HistoryAction::Undo(c)) | ("redo", HistoryAction::Redo(c)) => {
                    assert_eq!(c.command_id, "history");
                    assert_eq!(c.expected_revision, 8);
                }
                _ => panic!("wrong action"),
            }
        }
        let mut malformed = source;
        malformed["payload"]["command"]["edits"] = json!(true);
        assert!(take_history_input(&mut malformed).is_err());
    }
    #[test]
    fn owned_edit_input_moves_parsed_source_after_validation() {
        let wire = serde_json::to_vec(&json!({"path":"main.tex","expected_revision":1,
            "expected_sha256":"a".repeat(64),"text":"α".repeat(250_000),"response_mode":"metadata"})).unwrap();
        let mut payload: Value = serde_json::from_slice(&wire).unwrap();
        let parsed = payload["text"].as_str().unwrap();
        let pointer = parsed.as_ptr();
        let old_copy = parsed.to_owned();
        assert_ne!(pointer, old_copy.as_ptr());
        let input = take_edit_input(&mut payload).unwrap();
        assert_eq!(input.text.as_ptr(), pointer);
        assert_eq!(input.text, old_copy);
        assert!(input.metadata_only);
        assert_eq!(input.revision, 1);
        assert_eq!(input.path, "main.tex");
        assert_eq!(input.sha256, "a".repeat(64));
        assert!(payload["text"].is_null());
        eprintln!(
            "parsed_source_bytes={} moved_capacity={} avoided_clone_capacity={}",
            input.text.len(),
            input.text.capacity(),
            old_copy.capacity()
        );
        for (key, bad) in [
            ("response_mode", json!(true)),
            ("path", Value::Null),
            ("expected_revision", json!(-1)),
            ("expected_sha256", json!(42)),
            ("text", json!(false)),
        ] {
            let mut invalid: Value = serde_json::from_slice(&wire).unwrap();
            invalid[key] = bad;
            let before = invalid.clone();
            assert!(take_edit_input(&mut invalid).is_err());
            assert!(invalid == before, "invalid input was consumed at {key}");
        }
        for policy in [None, Some(json!("full"))] {
            let mut p: Value = serde_json::from_slice(&wire).unwrap();
            if let Some(policy) = policy {
                p["response_mode"] = policy;
            } else {
                p.as_object_mut().unwrap().remove("response_mode");
            }
            assert!(!take_edit_input(&mut p).unwrap().metadata_only);
        }
    }
    #[test]
    fn oversized_result_error_does_not_retain_large_output_allocation() {
        let (tx, rx) = output_delivery::channel(1);
        let stopped = AtomicBool::new(false);
        emit(&tx, &stopped, Value::String("x".repeat(MAX_OUTPUT_BYTES)));
        let bytes = rx.next(Duration::ZERO).unwrap().bytes;
        assert!(!stopped.load(Ordering::SeqCst));
        assert!(bytes.capacity() < 4096);
        assert_eq!(bytes.last(), Some(&b'\n'));
        let error: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(error["type"], "error");
        assert!(error["payload"]["message"]
            .as_str()
            .unwrap()
            .contains("durable"));
    }

    #[test]
    fn compiler_result_and_metadata_share_complete_output_budget_without_partial_frame() {
        let (tx, rx) = output_delivery::channel(2);
        let stopped = AtomicBool::new(false);
        // A result admitted below the compiler ceiling can still have too much
        // helper metadata. No fixed reserve can guarantee arbitrary path lengths.
        let mut payload = json!({"kind":"preview","result":"r".repeat(MAX_COMPILER_FRAME-128)});
        payload["source_versions"] = json!({"long-path":"m".repeat(COMPILER_ENVELOPE_RESERVE+256)});
        emit(
            &tx,
            &stopped,
            wire::envelope("s", Value::Null, "update", payload),
        );
        emit(
            &tx,
            &stopped,
            wire::envelope("s", json!("saved"), "result", json!({"durable":true})),
        );
        let rejected = rx.next(Duration::ZERO).unwrap();
        assert!(rejected.bytes.capacity() < 4096);
        let error: Value = serde_json::from_slice(&rejected.bytes).unwrap();
        assert_eq!(error["type"], "error");
        assert_eq!(error["session_id"], "s");
        rx.written(&rejected);
        let ack = rx.next(Duration::ZERO).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&ack.bytes).unwrap()["id"],
            "saved"
        );
        assert!(!stopped.load(Ordering::SeqCst));
    }

    /// A system shell plus the arguments that make it print
    /// `FLASHTEX_MAX_REPLY_BYTES` from *its own* environment, so the assertion
    /// covers what a spawned producer would really inherit rather than what the
    /// builder recorded.
    ///
    /// Both spellings are always present: `/bin/sh` is required by POSIX, and
    /// `cmd.exe` resolves through `PATH`/`ComSpec` on every Windows install. The
    /// Windows form prints a trailing CRLF (`echo` always terminates the line)
    /// and `printf` does not, so the caller trims line endings instead of
    /// comparing them. `%VAR%` is expanded by the child `cmd` against the
    /// environment block we hand it, which is exactly the value under test.
    fn echo_reply_budget_argv() -> (&'static str, [&'static str; 2]) {
        #[cfg(unix)]
        {
            ("/bin/sh", ["-c", "printf '%s' \"$FLASHTEX_MAX_REPLY_BYTES\""])
        }
        #[cfg(windows)]
        {
            ("cmd", ["/c", "echo %FLASHTEX_MAX_REPLY_BYTES%"])
        }
    }
    #[test]
    fn producer_launch_bounds_reply_and_preserves_stricter_settings() {
        let limits = compiler_limits(&json!({"compiler_max_frame_bytes":4096})).unwrap();
        for (inherited, expected) in [
            (None, "4095"),
            (Some("8192"), "4095"),
            (Some("2048"), "2048"),
            (Some("1"), "1"),
            (Some("0"), "4095"),
            (Some("invalid"), "4095"),
            (Some(" 7"), "4095"),
            (Some("7 "), "4095"),
            (Some("-1"), "4095"),
            (Some("+7"), "7"),
            (Some("0007"), "7"),
            (Some("99999999999999999999999999999999999"), "4095"),
        ] {
            let (shell, args) = echo_reply_budget_argv();
            let mut command =
                producer_command_with_limit(shell, &limits, inherited.map(std::ffi::OsStr::new));
            // Exercise the actual child environment without changing the test
            // process environment or racing other test threads.
            let output = command.args(args).output().unwrap();
            assert!(output.status.success());
            let printed = String::from_utf8(output.stdout).unwrap();
            assert_eq!(printed.trim_end_matches(['\r', '\n']), expected);
        }
    }
    #[test]
    fn producer_launch_forwards_project_root_only_when_present() {
        let limits = Limits::default();
        let mut plain = producer_command_with_limit("unused", &limits, None);
        append_project_root(&mut plain, None);
        assert_eq!(plain.get_args().count(), 0);
        let mut rooted = producer_command_with_limit("unused", &limits, None);
        append_project_root(&mut rooted, Some("/Users/me/paper dir"));
        let args: Vec<_> = rooted.get_args().collect();
        assert_eq!(
            args,
            [
                std::ffi::OsStr::new("--project-root"),
                std::ffi::OsStr::new("/Users/me/paper dir")
            ]
        );
    }
    /// An `OsString` that is deliberately *not* valid UTF-8, spelled the way
    /// each platform can actually represent one.
    ///
    /// Measured, not assumed: `OsStr` has no portable "arbitrary bytes"
    /// constructor. On Unix an `OsStr` is a bag of bytes, so a lone `0xff` (never
    /// a legal UTF-8 lead byte) is the shortest non-UTF-8 value and
    /// `OsStrExt::from_bytes` accepts it. On Windows an `OsStr` is WTF-8 over
    /// UTF-16, `std::os::unix` does not exist, and there is no `from_bytes`; the
    /// representable equivalent is an *unpaired surrogate*, which
    /// `OsStringExt::from_wide` accepts and `to_str` still rejects. The caller
    /// asserts that rejection so this stays a real non-UTF-8 value on both
    /// platforms rather than quietly degrading into an ordinary string on one.
    ///
    /// Both spellings exercise the same launcher branch: an inherited
    /// `FLASHTEX_MAX_REPLY_BYTES` that cannot be read as a number must be
    /// replaced by the computed budget, never forwarded to the producer.
    fn non_utf8_setting() -> std::ffi::OsString {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            std::ffi::OsString::from_vec(vec![0xff])
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            std::ffi::OsString::from_wide(&[0xd800])
        }
    }
    #[test]
    fn producer_budget_boundary_and_non_utf8_settings() {
        let non_utf8 = non_utf8_setting();
        assert!(
            non_utf8.to_str().is_none(),
            "the fixture must stay non-UTF-8 on this platform"
        );
        for frame in [128, 8 * 1024 * 1024, MAX_COMPILER_FRAME] {
            let limits = compiler_limits(&json!({"compiler_max_frame_bytes":frame})).unwrap();
            let equal = (frame - 1).to_string();
            for inherited in [
                None,
                Some(std::ffi::OsStr::new("")),
                Some(non_utf8.as_os_str()),
                Some(std::ffi::OsStr::new(&equal)),
            ] {
                let command = producer_command_with_limit("unused", &limits, inherited);
                let budget = command
                    .get_envs()
                    .find(|(key, _)| *key == "FLASHTEX_MAX_REPLY_BYTES")
                    .unwrap()
                    .1
                    .unwrap();
                assert_eq!(budget, std::ffi::OsStr::new(&equal));
            }
        }
    }
    #[test]
    fn compiler_frame_configuration_preserves_helper_headroom() {
        assert_eq!(
            compiler_limits(&json!({})).unwrap().max_frame,
            8 * 1024 * 1024
        );
        assert_eq!(
            compiler_limits(&json!({"compiler_max_frame_bytes":MAX_COMPILER_FRAME}))
                .unwrap()
                .max_frame,
            MAX_COMPILER_FRAME
        );
        for value in [
            json!(0),
            json!(-1),
            json!(1.5),
            json!("large"),
            Value::Null,
            json!(MAX_COMPILER_FRAME + 1),
        ] {
            assert!(compiler_limits(&json!({"compiler_max_frame_bytes":value})).is_err());
        }
    }
    #[test]
    fn required_serialization_refusal_delivers_error_then_next_ack() {
        let (tx, rx) = output_delivery::channel(2);
        let stopped = AtomicBool::new(false);
        emit_with_limit(
            &tx,
            &stopped,
            wire::envelope(
                "s",
                json!("large"),
                "result",
                json!({"body":"x".repeat(20000)}),
            ),
            512,
        );
        emit_with_limit(
            &tx,
            &stopped,
            wire::envelope("s", json!("ack"), "result", json!({"durable":true})),
            512,
        );
        assert!(!stopped.load(Ordering::SeqCst));
        let first = rx.next(Duration::ZERO).unwrap();
        let error: Value = serde_json::from_slice(&first.bytes).unwrap();
        assert_eq!(error["type"], "error");
        assert_eq!(error["id"], "large");
        assert!(error["payload"]["message"]
            .as_str()
            .unwrap()
            .contains("source may already be durable"));
        rx.written(&first);
        let second = rx.next(Duration::ZERO).unwrap();
        let ack: Value = serde_json::from_slice(&second.bytes).unwrap();
        assert_eq!(ack["id"], "ack");
        assert_eq!(ack["payload"]["durable"], true);
        rx.written(&second);
        assert!(rx.next(Duration::ZERO).is_err());
        emit_with_limit(&tx, &stopped, json!({"too":"large"}), 1);
        assert!(stopped.load(Ordering::SeqCst));
        assert!(rx.next(Duration::ZERO).is_err());
    }
}
