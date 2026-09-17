#!/usr/bin/env python3
"""Test double for `flashtex-preview-controller` (native helper protocol v1,
crates/preview-controller/STDIO.md) plus the PROPOSED `completed_snapshot`
side channel (crates/preview-controller/docs/completed-snapshot-proposal.md,
accepted with conditions on issue #2 comment 5645120420). Not a compiler, not
durable: everything lives in memory for one launch.

Launch: `fake_preview_controller.py CONFIG.json` exactly like the real helper.
The config's `entry_path` under `project_root` seeds durable revision 1.

Frames (JSON Lines, `protocol_version:1`, configured `session_id`):
  ready                          -> emitted at startup
  document {path}                -> result {document}
  edit {path, expected_revision, expected_sha256, text, source_binding_token?}
                                 -> result {document, preview_error:null,
                                    save_and_submit_ms}, then one compile
  compile {}                     -> result {submitted:true}, then one compile
  configure_layout {…}           -> result {submitted:true}, then one compile
  configure_completed_snapshots {capability:"completed-snapshots-v1", enabled}
                                 -> result {capability, enabled} exactly like
                                    the helper at ab945e6 (error "unsupported
                                    completed snapshot capability" for another
                                    capability; error "unknown operation" when
                                    the environment sets FAKE_PC_REFUSE_SNAPSHOTS=1,
                                    like a helper that predates the channel)
  export {path, expected_revision, expected_sha256, expected_disk_sha256}
                                 -> writes the durable text under project_root,
                                    result {path, sha256, bytes}. With
                                    FAKE_PC_EXPORT_MARK set, that file is created
                                    on receipt; with FAKE_PC_EXPORT_RELEASE set,
                                    the reply (and every later frame) waits until
                                    that file exists — a deterministically slow export
  close {}                       -> result {closed:true}, exit
  anything else                  -> error "unknown operation"

Each compile becomes `update {kind:"preview", request_id, compile_revision,
source_versions, missing_layout_capabilities:[], controller_total_ms,
runtime_total_ms, result:<compile_result envelope>}` whose single text item
is the first line of the entry text (same shape as fake_worker.py).

Scripted history (directives at the START of the entry text, like fake_worker):
  %hold      the compile for this edit is a slow compile A: nothing is emitted
             for it now. It completes when the NEXT compile B is emitted.
  On that next edit B (directive at the start of B's text):
    (default)     A is emitted as `completed_snapshot` BEFORE B's preview
    %after        A is emitted as `completed_snapshot` AFTER B's preview
    %badsession   A's payload `session_id` is "other-session"
    %badproject   A's payload `project_id` is "other-project"
    %badtoken     A's `source_binding_token` is replaced by a foreign token
    %current      A claims `is_current:true` (malformed by contract)
    %actions      A claims `source_actions_enabled:true` (malformed)
  Without an acknowledged `completed-snapshots-v1` negotiation, or when A's
  submission carried no `source_binding_token`, the held compile A is reported
  as `update {kind:"stale", request_id, compile_revision}` only — the wire an
  old helper produces — never as a completed snapshot.
"""
import hashlib
import json
import os
import sys
import time

if len(sys.argv) != 2:
    print("usage: fake_preview_controller.py CONFIG.json", file=sys.stderr)
    sys.exit(2)

with open(sys.argv[1], "r", encoding="utf-8") as fh:
    CONFIG = json.load(fh)
SESSION = CONFIG["session_id"]
PROJECT = CONFIG["project_id"]
ENTRY = CONFIG["entry_path"]
ROOT = CONFIG.get("project_root")
CAPABILITY = "completed-snapshots-v1"
REFUSE_NEGOTIATION = os.environ.get("FAKE_PC_REFUSE_SNAPSHOTS") == "1"
GAP_S = float(os.environ.get("FAKE_PC_GAP_MS", "150")) / 1000.0  # between A and B emissions

DIRECTIVES = ("%hold", "%after", "%badsession", "%badproject", "%badtoken", "%current", "%actions")


def sha256(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def emit(frame):
    sys.stdout.write(json.dumps(frame) + "\n")
    sys.stdout.flush()


def result(rid, payload):
    emit({"protocol_version": 1, "session_id": SESSION, "id": rid, "type": "result", "payload": payload})


def error(rid, message):
    emit({"protocol_version": 1, "session_id": SESSION, "id": rid, "type": "error", "payload": {"message": message}})


def update(payload):
    emit({"protocol_version": 1, "session_id": SESSION, "id": None, "type": "update", "payload": payload})


def initial_text():
    if ROOT:
        try:
            with open(os.path.join(ROOT, ENTRY), "r", encoding="utf-8") as fh:
                return fh.read()
        except OSError:
            pass
    return ""


documents = {ENTRY: {"revision": 1, "text": initial_text()}}
negotiated = False
compile_revision = 0
held = None  # the slow compile A: dict(request_id, compile_revision, source_versions, token, result)


def document_payload(path):
    d = documents[path]
    return {"path": path, "revision": d["revision"], "source_sha256": sha256(d["text"]), "text": d["text"]}


def strip_directives(text):
    """Directives are part of the durable text (they are what the user typed);
    only the rendered first line skips them so item text stays readable."""
    for _ in range(len(DIRECTIVES)):
        for d in DIRECTIVES:
            if text.startswith(d):
                text = text[len(d):]
    return text.lstrip("\n")


def flags(text):
    found = set()
    for _ in range(len(DIRECTIVES)):
        for d in DIRECTIVES:
            if text.startswith(d):
                found.add(d)
                text = text[len(d):]
    return found


def compile_result(rev):
    text = strip_directives(documents[ENTRY]["text"])
    first = text.split("\n", 1)[0]
    offset = len(documents[ENTRY]["text"].encode("utf-8")) - len(text.encode("utf-8"))
    end = offset + len(first.encode("utf-8"))
    return {"protocol_version": 1, "id": "preview-%d" % rev, "type": "compile_result",
            "payload": {"project_id": PROJECT, "revision": rev, "status": "ok",
                        "pages": [{"number": 1, "width_pt": 612, "height_pt": 792,
                                   "items": [{"kind": "text", "text": first, "x_pt": 72, "baseline_y_pt": 84,
                                              "font_size_pt": 12,
                                              "source": {"path": ENTRY, "start_byte": offset, "end_byte": end}}]}],
                        "diagnostics": [], "pdf_path": None}}


def source_versions():
    return {p: d["revision"] for p, d in documents.items()}


def emit_preview(frame):
    update({"kind": "preview", "request_id": frame["request_id"], "compile_revision": frame["compile_revision"],
            "source_versions": frame["source_versions"], "missing_layout_capabilities": [],
            "controller_total_ms": 1.0, "runtime_total_ms": 0.5, "result": frame["result"]})


def emit_history(frame, current_revision, tamper):
    """The held compile A completes while B (current_revision) is the current
    compile: a completed snapshot when negotiated and bound, else `stale`."""
    if not negotiated or frame["token"] is None:
        update({"kind": "stale", "request_id": frame["request_id"], "compile_revision": frame["compile_revision"]})
        print("fake_preview_controller: held compile %s reported stale (negotiated=%s, token=%s)"
              % (frame["request_id"], negotiated, frame["token"] is not None), file=sys.stderr, flush=True)
        return
    payload = {"kind": "completed_snapshot", "project_id": PROJECT, "session_id": SESSION,
               "request_id": frame["request_id"], "compile_revision": frame["compile_revision"],
               "source_versions": frame["source_versions"], "current_compile_revision": current_revision,
               "is_current": False, "source_actions_enabled": False,
               "source_binding_token": frame["token"], "result": frame["result"]}
    if "%badsession" in tamper:
        payload["session_id"] = "other-session"
    if "%badproject" in tamper:
        payload["project_id"] = "other-project"
    if "%badtoken" in tamper:
        payload["source_binding_token"] = "ftx1:deadbeefdeadbeef:99"
    if "%current" in tamper:
        payload["is_current"] = True
    if "%actions" in tamper:
        payload["source_actions_enabled"] = True
    update(payload)


def run_compile(token):
    """One compile of the current source. Honors %hold (retain as A) and, when a
    held A exists, releases it around this compile B per B's directives."""
    global compile_revision, held
    compile_revision += 1
    rev = compile_revision
    frame = {"request_id": "preview-%d" % rev, "compile_revision": rev, "source_versions": source_versions(),
             "token": token, "result": compile_result(rev)}
    text = documents[ENTRY]["text"]
    f = flags(text)
    if "%hold" in f:
        if held is not None:
            # A second slow compile supersedes the first: the runtime keeps one.
            update({"kind": "stale", "request_id": held["request_id"], "compile_revision": held["compile_revision"]})
        held = frame
        print("fake_preview_controller: holding compile %s" % frame["request_id"], file=sys.stderr, flush=True)
        return
    previous, held = held, None
    if previous is not None and "%after" not in f:
        emit_history(previous, rev, f)
        time.sleep(GAP_S)  # let native paint A before B arrives (deterministic order for tests)
    emit_preview(frame)
    if previous is not None and "%after" in f:
        time.sleep(GAP_S)
        emit_history(previous, rev, f)


emit({"protocol_version": 1, "session_id": SESSION, "id": None, "type": "ready",
      "payload": {"compiler_error": None, "compiler_max_frame_bytes": 8 * 1024 * 1024,
                  "helper_max_output_bytes": 16 * 1024 * 1024}})

for raw in sys.stdin:
    raw = raw.strip()
    if not raw:
        continue
    try:
        env = json.loads(raw)
    except ValueError:
        error(None, "invalid JSON frame")
        continue
    rid = env.get("id")
    if env.get("protocol_version") != 1 or env.get("session_id") != SESSION or not rid:
        error(rid, "invalid envelope")
        continue
    op = env.get("type")
    p = env.get("payload") or {}
    if op == "document":
        path = p.get("path")
        if path not in documents:
            error(rid, "unknown document %s" % path)
        else:
            result(rid, {"document": document_payload(path)})
    elif op == "edit":
        path = p.get("path")
        if path not in documents:
            error(rid, "unknown document %s" % path)
            continue
        d = documents[path]
        if p.get("expected_revision") != d["revision"] or p.get("expected_sha256") != sha256(d["text"]):
            error(rid, "document_conflict: expected r%s, durable r%d" % (p.get("expected_revision"), d["revision"]))
            continue
        token = p.get("source_binding_token")
        if token is not None and (not isinstance(token, str) or not token or len(token.encode("utf-8")) > 128):
            error(rid, "source_binding_token must contain 1..128 bytes")
            continue
        print("fake_preview_controller: edit r%d token=%s" % (d["revision"] + 1, "present" if token else "absent"),
              file=sys.stderr, flush=True)
        d["revision"] += 1
        d["text"] = p.get("text", "")
        result(rid, {"document": document_payload(path), "preview_error": None, "save_and_submit_ms": 0.5})
        run_compile(token)
    elif op == "compile":
        result(rid, {"submitted": True})
        run_compile(p.get("source_binding_token"))
    elif op == "configure_layout":
        if p.get("renderer_support_confirmed") is not True:
            error(rid, "explicit native renderer support confirmation required")
            continue
        result(rid, {"submitted": True})
        run_compile(None)
    elif op == "configure_completed_snapshots":
        if REFUSE_NEGOTIATION:
            error(rid, "unknown operation")
            continue
        if p.get("capability") != CAPABILITY:
            error(rid, "unsupported completed snapshot capability")
            continue
        if not isinstance(p.get("enabled"), bool):
            error(rid, "enabled must be boolean")
            continue
        negotiated = p["enabled"]
        # Renegotiation resets retained history, like a helper policy epoch.
        held = None
        print("fake_preview_controller: negotiated=%s" % negotiated, file=sys.stderr, flush=True)
        result(rid, {"capability": CAPABILITY, "enabled": negotiated})
    elif op == "export":
        path = p.get("path")
        if path not in documents or not ROOT:
            error(rid, "unknown document %s" % path)
            continue
        d = documents[path]
        if p.get("expected_revision") != d["revision"] or p.get("expected_sha256") != sha256(d["text"]):
            error(rid, "document_conflict: expected r%s, durable r%d" % (p.get("expected_revision"), d["revision"]))
            continue
        if os.environ.get("FAKE_PC_EXPORT_MARK"):
            open(os.environ["FAKE_PC_EXPORT_MARK"], "w").close()
        release = os.environ.get("FAKE_PC_EXPORT_RELEASE")
        while release and not os.path.exists(release):
            time.sleep(0.01)
        with open(os.path.join(ROOT, path), "w", encoding="utf-8") as fh:
            fh.write(d["text"])
        result(rid, {"path": path, "sha256": sha256(d["text"]), "bytes": len(d["text"].encode("utf-8"))})
    elif op == "restart":
        negotiated = False
        held = None
        result(rid, {"submitted": True})
    elif op == "close":
        negotiated = False
        held = None
        result(rid, {"closed": True})
        break
    else:
        error(rid, "unknown operation")
