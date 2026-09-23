#!/usr/bin/env python3
"""TEST DOUBLE for the FT-007 capture bridge (transfer-v1 JSON Lines).

Not the real bridge: no image decoding, no Grok, one JSON file instead of the
real per-capture journal. It mirrors the request/reply table of
docs/contracts/transfer-v1.md and the checks in crates/bridge/src/lib.rs
(bridge commit b5ca96b) so the Mac shell's review -> prepare -> apply ->
applied flow, its error paths and its crash recovery can be exercised hermetically. Conversion is deterministic:
    latex = "\\fakecapture{<capture_id>}", one ambiguity, no dependencies.

Usage: python3 fake_bridge.py --store <dir> [--enable-grok]

Test directives in `capture_submit.instructions`:
    %error:<code>   reply with an error envelope carrying that code
    %garbage        emit a non-JSON line
    %huge           emit one line larger than 12 MiB
    %trailing       emit a partial line and exit
    %exit           exit without replying
    %stall:<sec>    stop reading stdin for that long before replying (slow conversion)
    %crash          exit abnormally (status 3) before journaling, no reply (every time)
    %crash-once     like %crash, but only the first time for this capture ID
                    (marker file in the store); the identical resubmission succeeds
    %crash-once-journaled
                    journal the capture durably, then exit 3 without replying, once;
                    the identical resubmission returns the journaled record
Test directives in `capture_status.capture_id`:
    err-<code>-*    reply with an error envelope carrying <code>
    garbage-*       emit a non-JSON line instead of a reply
    stall-<sec>-*   sleep before answering normally
    crash-status-*  exit abnormally (status 3) without replying
Test directives in `capture_applied.capture_id`:
    crash-applied-before-once-*
                    exit 3 before recording the receipt, once (the retry succeeds)
    crash-applied-once-*
                    record the receipt durably, then exit 3 without replying, once

Like the real bridge, the capture journal is durable in `--store`
(`fake_journal.json`, rewritten atomically on every change) so a relaunched
process answers `capture_status` for earlier captures; documents and anchors
are in memory and must be resynchronized after a restart.
"""
import base64
import hashlib
import json
import os
import re
import sys
import tempfile
import time

MAX_FRAME = 12 * 1024 * 1024
IDENT = re.compile(r"^[A-Za-z0-9_-]{1,128}$")
CRASH_STATUS = 3

documents = {}   # (project, path) -> {"revision": int, "text": bytes}
anchors = {}     # destination_id -> anchor dict (offsets in bytes)
journal = {}     # capture_id -> record dict
enable_grok = "--enable-grok" in sys.argv[1:]
store_dir = None


def journal_path():
    return os.path.join(store_dir, "fake_journal.json")


def load_journal():
    global journal
    try:
        with open(journal_path(), "rb") as f:
            journal = json.loads(f.read().decode("utf-8"))
    except FileNotFoundError:
        journal = {}


def persist_journal():
    """Atomic rewrite (temp file + fsync + rename), as the real bridge does per record."""
    data = json.dumps(journal).encode("utf-8")
    fd, tmp = tempfile.mkstemp(dir=store_dir)
    with os.fdopen(fd, "wb") as f:
        f.write(data)
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, journal_path())


def crash():
    sys.stdout.flush()
    os._exit(CRASH_STATUS)


def crash_once(key):
    """Crashes the first time `key` is seen for this store; later calls proceed."""
    marker = os.path.join(store_dir, "fake_crashed_" + hashlib.sha256(key.encode("utf-8")).hexdigest()[:16])
    if os.path.exists(marker):
        return
    with open(marker, "w") as f:
        f.write(key)
    crash()


class Err(Exception):
    def __init__(self, code, message):
        super().__init__(message)
        self.code, self.message = code, message


def sha(b):
    return hashlib.sha256(b).hexdigest()


def ident(s):
    if not isinstance(s, str) or not IDENT.match(s):
        raise Err("invalid_id", "IDs require 1-128 ASCII letters, digits, hyphens or underscores")


def relpath(p):
    if not p or p.startswith("/") or "\\" in p or ":" in p or "\0" in p or \
            any(seg in ("", ".", "..") for seg in p.split("/")):
        raise Err("invalid_path", "Expected a normalized project-relative path")


def boundary(text, i):
    return i == len(text) or (0 <= i < len(text) and (text[i] & 0xC0) != 0x80)


def check_range(text, start, end):
    if not (isinstance(start, int) and isinstance(end, int)) or start > end or end > len(text) \
            or not boundary(text, start) or not boundary(text, end):
        raise Err("invalid_source_range", "Offsets must delimit complete UTF-8 scalars in the current source")


def document(project, path):
    doc = documents.get((project, path))
    if doc is None:
        raise Err("document_missing", "Open the source snapshot on the Mac first")
    return doc


def require(capture_id):
    ident(capture_id)
    rec = journal.get(capture_id)
    if rec is None:
        raise Err("capture_missing", "Capture has not been durably received")
    return rec


def capture_anchor(capture):
    a = anchors.get(capture["destination_id"])
    if a is None or not a["valid"]:
        raise Err("destination_reselection_required", "The pinned target is missing or changed ambiguously")
    if a.get("mode") == "caret":
        # transfer-v1 additive: the automatic destination is "the caret now";
        # the companion's base_revision and the receipt binding are informational.
        return a
    if capture["base_revision"] != a["pinned_revision"]:
        raise Err("revision_conflict", "Capture does not refer to the pinned destination revision")
    rec = journal.get(capture["capture_id"])
    if rec is not None and rec.get("destination_binding") != a["binding"]:
        raise Err("destination_reselection_required",
                  "Restored destination does not match this capture's durably bound source and range")
    return a


def apply_edit(project, path, base, revision, start, end, replacement):
    doc = document(project, path)
    if doc["revision"] != base or revision <= base:
        raise Err("revision_conflict", "Edit must advance the exact current revision")
    check_range(doc["text"], start, end)
    rep = replacement.encode("utf-8")
    shift = len(rep) - (end - start)
    for a in anchors.values():
        if a["project_id"] != project or a["path"] != path or not a["valid"]:
            continue
        if a.get("mode") == "caret":
            # Follows the caret: shifted by text before/at it, collapsed after a spanning edit.
            if end <= a["start_byte"]:
                a["start_byte"] += shift
                a["end_byte"] += shift
            elif start <= a["end_byte"]:
                a["start_byte"] = a["end_byte"] = start + len(rep)
            a["current_revision"] = revision
            continue
        if (start == end and a["start_byte"] <= start <= a["end_byte"]) or \
                (start < a["end_byte"] and end > a["start_byte"]) or \
                (a["start_byte"] == a["end_byte"] and start <= a["start_byte"] < end):
            a["valid"] = False
        elif end <= a["start_byte"]:
            a["start_byte"] += shift
            a["end_byte"] += shift
        a["current_revision"] = revision
    doc["text"] = doc["text"][:start] + rep + doc["text"][end:]
    doc["revision"] = revision


def dispatch(kind, p):
    if kind == "document_open":
        ident(p["project_id"]); relpath(p["path"])
        key = (p["project_id"], p["path"])
        text = p["text"].encode("utf-8")
        old = documents.get(key)
        if old is not None:
            if old["revision"] == p["revision"] and old["text"] == text:
                return "document_opened", {}
            if p["revision"] <= old["revision"]:
                raise Err("revision_conflict", "Snapshots must advance revision when source changes")
            for a in anchors.values():
                if (a["project_id"], a["path"]) == key:
                    a["valid"] = False
        documents[key] = {"revision": p["revision"], "text": text}
        return "document_opened", {}
    if kind == "document_edit":
        apply_edit(p["project_id"], p["path"], p["base_revision"], p["revision"],
                   p["start_byte"], p["end_byte"], p["replacement"])
        return "document_updated", {"revision": p["revision"]}
    if kind == "destination_pin":
        ident(p["destination_id"])
        doc = document(p["project_id"], p["path"])
        if doc["revision"] != p["revision"]:
            raise Err("revision_conflict", "Pin against the current source revision")
        check_range(doc["text"], p["start_byte"], p["end_byte"])
        mode = p.get("mode", "fixed")
        anchor = {"destination_id": p["destination_id"], "project_id": p["project_id"], "path": p["path"],
                  "pinned_revision": p["revision"], "current_revision": p["revision"],
                  "start_byte": p["start_byte"], "end_byte": p["end_byte"], "valid": True,
                  "binding": {"project_id": p["project_id"], "path": p["path"], "revision": p["revision"],
                              "start_byte": p["start_byte"], "end_byte": p["end_byte"],
                              "source_sha256": sha(doc["text"])},
                  "mode": mode}
        old = anchors.get(p["destination_id"])
        strict = old is not None and old["valid"] and old.get("mode", "fixed") == "fixed" and mode == "fixed"
        if strict and old != anchor:
            raise Err("destination_conflict", "Use a new destination ID when repinning a different target")
        anchors[p["destination_id"]] = anchor
        return "destination_pinned", anchor
    if kind == "capture_submit":
        ident(p["capture_id"]); ident(p["destination_id"])
        ins = p.get("instructions", "")
        if ins.startswith("%error:"):
            raise Err(ins[len("%error:"):].strip(), "requested failure")
        if ins.startswith("%garbage"):
            return None, "this is not json"
        if ins.startswith("%huge"):
            return None, '{"protocol_version":1,"id":"x","type":"capture_received","payload":{"x":"' + "x" * (MAX_FRAME + 64) + '"}}'
        if ins.startswith("%trailing"):
            sys.stdout.write('{"protocol_version":1,"id":"x","type":"capture_re')
            sys.stdout.flush()
            sys.exit(0)
        if ins.startswith("%exit"):
            sys.exit(0)
        if ins.startswith("%crash-once-journaled"):
            pass  # journaled below, then crashes once
        elif ins.startswith("%crash-once"):
            crash_once("submit:" + p["capture_id"])
        elif ins.startswith("%crash"):
            crash()
        if ins.startswith("%stall:"):
            time.sleep(float(ins[len("%stall:"):].split()[0]))
        if len(ins.encode("utf-8")) > 4096:
            raise Err("instructions_too_large", "Instructions exceed 4096 UTF-8 bytes")
        if p["image"]["mime_type"] not in ("image/png", "image/jpeg"):
            raise Err("unsupported_image", "Only PNG and JPEG captures are accepted")
        try:
            raw = base64.b64decode(p["image"]["data_base64"], validate=True)
        except Exception:
            raise Err("invalid_image", "Invalid base64 image")
        if not raw or len(raw) > 8 * 1024 * 1024:
            raise Err("image_too_large", "Image must contain 1-8 MiB of encoded image bytes")
        old = journal.get(p["capture_id"])
        if old is not None:
            if old["capture"] != p:
                raise Err("capture_id_conflict", "Capture ID already belongs to different content")
        else:
            binding = capture_anchor(p)["binding"]
            old = {"capture": p, "proposal": None, "context": None, "prepared": None, "applied": None,
                   "rejected": False, "destination_binding": dict(binding)}
            journal[p["capture_id"]] = old
            persist_journal()
        if ins.startswith("%crash-once-journaled"):
            crash_once("submit-journaled:" + p["capture_id"])
        return "capture_received", {"capture_id": p["capture_id"], "durable": True,
                                    "has_proposal": old["proposal"] is not None,
                                    "applied": old["applied"] is not None}
    if kind == "capture_convert":
        rec = require(p["capture_id"])
        if rec["rejected"]:
            raise Err("capture_rejected", "This capture was rejected during review")
        if rec["capture"].get("instructions", "").startswith("%provider_disabled"):
            raise Err("provider_disabled", "Grok conversion requires explicitly enabled provider configuration")
        if rec["capture"].get("instructions", "").startswith("%provider_auth_missing"):
            raise Err("provider_auth_missing", "Supply the Mac's authorized Grok key through its credential adapter")
        # `%grokgate`: the real bridge's gates (main.rs capture_convert): --enable-grok
        # then XAI_API_KEY in the environment; the proposal then names the model
        # and the key's presence (never its value).
        grok_gate = rec["capture"].get("instructions", "").startswith("%grokgate")
        if grok_gate and not enable_grok:
            raise Err("provider_disabled", "Grok conversion requires explicitly enabled provider configuration")
        if grok_gate and not os.environ.get("XAI_API_KEY"):
            raise Err("provider_auth_missing", "Supply the Mac's authorized Grok key through its credential adapter")
        if rec["proposal"] is None:
            a = capture_anchor(rec["capture"])
            doc = document(a["project_id"], a["path"])
            rec["context"] = {"revision": doc["revision"]}
            ambiguities = ["fake bridge: deterministic transcription, not a real conversion"]
            if grok_gate:
                ambiguities.append("model:%s key:present" % os.environ.get("FLASHTEX_GROK_MODEL", "grok-4.6"))
            rec["proposal"] = {"latex": "\\fakecapture{%s}" % p["capture_id"],
                               "ambiguities": ambiguities,
                               "required_dependencies": []}
            persist_journal()
        pr = rec["proposal"]
        return "capture_proposal", {"capture_id": p["capture_id"], "latex": pr["latex"],
                                    "ambiguities": pr["ambiguities"],
                                    "required_dependencies": pr["required_dependencies"],
                                    "context_revision": rec["context"]["revision"]}
    if kind == "capture_prepare_insert":
        if not p.get("approved"):
            raise Err("review_required", "Explicit review approval is required")
        rec = require(p["capture_id"])
        if rec["rejected"]:
            raise Err("capture_rejected", "This capture was rejected during review")
        if rec["applied"] is not None:
            raise Err("already_applied", "This capture was already inserted")
        expected = p["expected_revision"]
        if rec["prepared"] is not None:
            e = rec["prepared"]
            doc = document(e["project_id"], e["path"])
            if e["expected_revision"] == expected and doc["revision"] == expected and \
                    sha(doc["text"]) == e["document_before_sha256"]:
                return "capture_edit", e
            raise Err("revision_conflict", "Previously prepared edit requires receipt reconciliation")
        a = capture_anchor(rec["capture"])
        doc = document(a["project_id"], a["path"])
        if doc["revision"] != expected:
            raise Err("revision_conflict", "Review and prepare against the current Mac source")
        if rec["proposal"] is None:
            raise Err("proposal_missing", "Convert the capture before reviewing insertion")
        wrap = p.get("wrap")
        replacement = rec["proposal"]["latex"]
        if wrap is not None:
            replacement = wrap.get("prefix", "") + replacement + wrap.get("suffix", "")
        edit = {"capture_id": p["capture_id"], "edit_id": "capture-%s" % p["capture_id"],
                "project_id": a["project_id"], "path": a["path"], "expected_revision": expected,
                "start_byte": a["start_byte"], "end_byte": a["end_byte"],
                "removed_text": doc["text"][a["start_byte"]:a["end_byte"]].decode("utf-8"),
                "replacement": replacement, "document_before_sha256": sha(doc["text"])}
        if wrap is not None:
            edit["wrap"] = wrap
        rec["prepared"] = edit
        persist_journal()
        return "capture_edit", edit
    if kind == "capture_applied":
        rec = require(p["capture_id"])
        if rec["applied"] is not None:
            if rec["applied"]["edit_id"] == p["edit_id"] and rec["applied"]["new_revision"] == p["new_revision"]:
                return "capture_application_received", {"capture_id": p["capture_id"], **rec["applied"]}
            raise Err("receipt_conflict", "Capture already has a different insertion receipt")
        e = rec["prepared"]
        if e is None:
            raise Err("review_required", "Prepare a reviewed edit before confirming it")
        if e["edit_id"] != p["edit_id"] or p["new_revision"] <= e["expected_revision"]:
            raise Err("receipt_conflict", "Invalid edit receipt")
        doc = document(e["project_id"], e["path"])
        if doc["revision"] != e["expected_revision"] or sha(doc["text"]) != e["document_before_sha256"]:
            raise Err("revision_conflict", "Source changed after insertion was prepared; reconcile Mac edit ledger")
        if p["capture_id"].startswith("crash-applied-before-once-"):
            crash_once("applied-before:" + p["capture_id"])
        rec["applied"] = {"edit_id": p["edit_id"], "new_revision": p["new_revision"]}
        persist_journal()
        if p["capture_id"].startswith("crash-applied-once-"):
            crash_once("applied-after:" + p["capture_id"])
        apply_edit(e["project_id"], e["path"], e["expected_revision"], p["new_revision"],
                   e["start_byte"], e["end_byte"], e["replacement"])
        return "capture_application_received", {"capture_id": p["capture_id"], **rec["applied"]}
    if kind == "capture_status":
        cid = p["capture_id"]
        if cid.startswith("err-"):
            raise Err(cid.split("-")[1], "requested status failure")
        if cid.startswith("garbage-"):
            return None, "status garbage"
        if cid.startswith("stall-"):
            time.sleep(float(cid.split("-")[1]))
        if cid.startswith("crash-status-"):
            crash()
        rec = require(p["capture_id"])
        return "capture_status", {"capture_id": p["capture_id"], "proposal": rec["proposal"],
                                  "prepared": rec["prepared"], "applied": rec["applied"],
                                  "rejected": rec["rejected"]}
    if kind == "capture_reject":
        rec = require(p["capture_id"])
        if rec["prepared"] is not None or rec["applied"] is not None:
            raise Err("receipt_conflict", "A prepared edit requires Mac ledger reconciliation; rejection cannot revoke an issued edit")
        rec["rejected"] = True
        persist_journal()
        return "capture_rejected", {"capture_id": p["capture_id"]}
    raise Err("unsupported_type", "Unknown bridge request type")


def reply(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


def main():
    global store_dir
    args = sys.argv[1:]
    if "--store" not in args or args.index("--store") + 1 >= len(args):
        print("invalid_arguments: --store DIRECTORY required", file=sys.stderr)
        sys.exit(1)
    store_dir = args[args.index("--store") + 1]
    os.makedirs(store_dir, mode=0o700, exist_ok=True)
    load_journal()
    print("fake_bridge: started (test double, not the real bridge; %d journaled capture(s))" % len(journal),
          file=sys.stderr, flush=True)
    for raw in sys.stdin.buffer:
        if len(raw) > MAX_FRAME:
            reply({"protocol_version": 1, "id": None, "type": "error",
                   "payload": {"code": "message_too_large", "message": "Bridge message exceeded 12 MiB; record discarded"}})
            continue
        raw = raw.strip()
        if not raw:
            continue
        try:
            env = json.loads(raw)
        except ValueError:
            reply({"protocol_version": 1, "id": None, "type": "error",
                   "payload": {"code": "invalid_json", "message": "Malformed UTF-8 JSON request"}})
            continue
        rid = env.get("id")
        if not isinstance(rid, str) or not rid or len(rid.encode("utf-8")) > 128:
            reply({"protocol_version": 1, "id": None, "type": "error",
                   "payload": {"code": "invalid_id", "message": "Every request needs a bounded nonempty string ID"}})
            continue
        try:
            if env.get("protocol_version") != 1:
                raise Err("unsupported_version", "Only protocol_version 1 is supported")
            kind, payload = dispatch(env.get("type"), env.get("payload") or {})
            if kind is None:
                sys.stdout.write(payload + "\n")
                sys.stdout.flush()
                continue
            reply({"protocol_version": 1, "id": rid, "type": kind, "payload": payload})
        except Err as e:
            reply({"protocol_version": 1, "id": rid, "type": "error", "payload": {"code": e.code, "message": e.message}})
        except (KeyError, TypeError) as e:
            reply({"protocol_version": 1, "id": rid, "type": "error",
                   "payload": {"code": "invalid_json", "message": "missing or mistyped field: %s" % e}})


if __name__ == "__main__":
    main()
