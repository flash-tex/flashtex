#!/usr/bin/env python3
"""TEST DOUBLE for the FlashTeX edit-ledger helper (crates/edit-ledger at
afb15839f8080f9e86efd6a187e46dfe014ce559, `flashtex-edit-ledger --store DIR`).

Not the real helper: it mirrors its JSON Lines protocol and validation rules
(initialize / status / apply / replace_document / confirm / recovery_export /
recovery_import, one document.json holding source + applied edit IDs, written
atomically with fsync before any reply, replies tagged with session_id,
sequence, document_revision, document_sha256 and command_succeeded) so the Mac
shell's durable-commit ordering can be tested hermetically, including injected
persistence failures (make the store directory read-only). A corrupt or
unreadable document.json fails closed: every request answers `invalid_store`.

Usage: python3 fake_edit_ledger.py --store <dir>

Crash directives (exit status 3 without replying; "once" means a marker file
in the store makes the identical retry proceed normally):
    apply with edit.capture_id `crash-before-commit-*`
                    exit before anything is written, once
    apply with edit.capture_id `crash-after-commit-*`
                    commit the edit durably, then exit before replying, once
    replace_document whose text contains `%ledger-crash-once`
                    exit before committing, once per distinct text
    apply with edit.capture_id `hang-before-commit-*` / `hang-after-commit-*`
                    stay alive but never answer (nor read again), before
                    anything is written / after the durable commit, once
    operation `crash`
                    exit immediately (idle crash)
"""
import hashlib
import json
import os
import re
import sys
import tempfile
import time

CRASH_STATUS = 3


def crash():
    sys.stdout.flush()
    os._exit(CRASH_STATUS)


def hang_once(root, key):
    """Alive, answering nothing, until killed: the first time only (a marker file)."""
    marker = os.path.join(root, "fake_hung_" + hashlib.sha256(key.encode("utf-8")).hexdigest()[:16])
    if os.path.exists(marker):
        return
    with open(marker, "w") as f:
        f.write(key)
    sys.stdout.flush()
    while True:
        time.sleep(3600)


def crash_once(root, key):
    marker = os.path.join(root, "fake_crashed_" + hashlib.sha256(key.encode("utf-8")).hexdigest()[:16])
    if os.path.exists(marker):
        return
    with open(marker, "w") as f:
        f.write(key)
    crash()

MAX_LINE = 12 * 1024 * 1024
IDENT = re.compile(r"^[A-Za-z0-9_-]{1,128}$")


class Err(Exception):
    def __init__(self, code, message):
        super().__init__(message)
        self.code, self.message = code, message


def sha(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def boundary(b, i):
    return i == len(b) or (0 <= i < len(b) and (b[i] & 0xC0) != 0x80)


class Store:
    def __init__(self, root):
        self.root = root
        self.poisoned = False
        self.startup_error = None
        os.makedirs(root, mode=0o700, exist_ok=True)
        self.path = os.path.join(root, "document.json")
        self.state = None
        if os.path.exists(self.path):
            try:
                with open(self.path, "rb") as f:
                    self.state = json.loads(f.read().decode("utf-8"))
                if self.state.get("schema_version") != 1 or "document" not in self.state:
                    raise Err("invalid_store", "unknown schema")
            except Err as e:
                self.startup_error = e
            except Exception as e:
                self.startup_error = Err("invalid_store", "document.json unreadable or corrupt: %s" % e)

    def ready(self):
        if self.startup_error is not None:
            raise self.startup_error
        if self.poisoned:
            raise Err("recovery_required", "persistence failed; drop handle and reopen before retry")

    def snapshot_token(self):
        return sha(json.dumps(self.state, sort_keys=True))

    def export_recovery(self):
        self.ready()
        if self.state is None:
            raise Err("document_missing", "initialize source first")
        return {"snapshot_token": self.snapshot_token(), "current_document": self.state["document"],
                "pending_receipts": self.recovery()}

    def import_recovery(self, recovery):
        exported = self.export_recovery()
        if exported["snapshot_token"] != recovery["snapshot_token"]:
            raise Err("stale_recovery_snapshot", "source or ledger advanced while querying bridge; export again")
        state = json.loads(json.dumps(self.state))
        seen = set()
        actions = []
        changed = False
        for obs in recovery["observations"]:
            status = obs["status"]
            cid = obs["receipt"]["capture_id"] if status == "applied" else obs["edit"]["capture_id"] if status == "prepared" else obs["capture_id"]
            if cid in seen:
                raise Err("duplicate_observation", "one observation per capture is required")
            seen.add(cid)
            tx = next((t for t in state["transactions"].values() if t["edit"]["capture_id"] == cid), None)
            if tx is None:
                raise Err("capture_missing", "observation does not name a locally applied capture")
            if status == "applied":
                if tx["receipt"] != obs["receipt"]:
                    raise Err("receipt_conflict", "bridge applied fields differ from local durable receipt")
                if not tx["confirmed"]:
                    tx["confirmed"] = True
                    tx["document_before"] = None
                    changed = True
                actions.append({"action": "confirmed", "receipt": obs["receipt"]})
            elif status == "prepared":
                if tx["edit"] != obs["edit"]:
                    raise Err("edit_id_conflict", "bridge prepared fields differ from the durable local edit")
                if tx["document_before"] is None:
                    raise Err("bridge_state_regressed", "bridge lost a previously acknowledged receipt")
                actions.append({"action": "replay_receipt", "document_before": tx["document_before"], "receipt": tx["receipt"]})
            elif status == "unavailable":
                actions.append({"action": "retry_status", "capture_id": cid, "reason": obs["reason"][:4096]})
            else:
                raise Err("invalid_request", "unknown observation status")
        if changed:
            self.commit(state)
        return {"actions": actions, "recovery": self.export_recovery()}

    def commit(self, state):
        data = json.dumps(state).encode("utf-8")
        try:
            fd, tmp = tempfile.mkstemp(dir=self.root)
            with os.fdopen(fd, "wb") as f:
                f.write(data)
                f.flush()
                os.fsync(f.fileno())
            os.replace(tmp, self.path)
            dfd = os.open(self.root, os.O_RDONLY)
            try:
                os.fsync(dfd)
            finally:
                os.close(dfd)
        except OSError as e:
            self.poisoned = True
            raise Err("storage_error", str(e))
        self.state = state

    def document(self):
        self.ready()
        return None if self.state is None else self.state["document"]

    def initialize(self, document):
        self.ready()
        if not IDENT.match(document["project_id"]) or not document["path"]:
            raise Err("invalid_id", "bad identifiers")
        if document["source_sha256"] != sha(document["text"]):
            raise Err("invalid_document", "document size/hash invalid")
        if self.state is not None:
            if self.state["document"] == document:
                return
            raise Err("document_exists", "load existing durable source instead of overwriting")
        self.commit({"schema_version": 1, "document": document, "transactions": {}})

    def apply(self, edit):
        self.ready()
        if self.state is None:
            raise Err("document_missing", "initialize source first")
        txs = self.state["transactions"]
        old = txs.get(edit["edit_id"])
        if old is not None:
            if old["edit"] == edit:
                return old["receipt"]
            raise Err("edit_id_conflict", "edit ID already binds different prepared fields")
        if edit["capture_id"].startswith("crash-before-commit-"):
            crash_once(self.root, "apply-before:" + edit["edit_id"])
        if edit["capture_id"].startswith("hang-before-commit-"):
            hang_once(self.root, "apply-before:" + edit["edit_id"])
        if any(t["edit"]["capture_id"] == edit["capture_id"] for t in txs.values()):
            raise Err("capture_id_conflict", "capture already applied under another edit ID")
        doc = self.state["document"]
        if edit["project_id"] != doc["project_id"] or edit["path"] != doc["path"]:
            raise Err("document_conflict", "edit targets another document")
        if edit["expected_revision"] != doc["revision"]:
            raise Err("revision_conflict", "edit revision is stale")
        if edit["document_before_sha256"] != sha(doc["text"]):
            raise Err("source_hash_conflict", "source differs from prepared snapshot")
        b = doc["text"].encode("utf-8")
        s, e = edit["start_byte"], edit["end_byte"]
        if not (0 <= s <= e <= len(b)) or not boundary(b, s) or not boundary(b, e):
            raise Err("invalid_source_range", "range must be ordered UTF-8 scalar boundaries within source")
        if b[s:e].decode("utf-8") != edit["removed_text"]:
            raise Err("removed_text_conflict", "selected source differs from removed_text")
        if len(edit["replacement"].encode("utf-8")) > 64 * 1024:
            raise Err("replacement_too_large", "replacement exceeds 64 KiB")
        text = (b[:s] + edit["replacement"].encode("utf-8") + b[e:]).decode("utf-8")
        after = {"project_id": doc["project_id"], "path": doc["path"], "revision": doc["revision"] + 1,
                 "text": text, "source_sha256": sha(text)}
        receipt = {"capture_id": edit["capture_id"], "edit_id": edit["edit_id"], "new_revision": after["revision"]}
        state = json.loads(json.dumps(self.state))
        state["document"] = after
        state["transactions"][edit["edit_id"]] = {"edit": edit, "receipt": receipt, "document_before": doc,
                                                  "document_after_sha256": after["source_sha256"], "confirmed": False}
        self.commit(state)
        if edit["capture_id"].startswith("crash-after-commit-"):
            crash_once(self.root, "apply-after:" + edit["edit_id"])
        if edit["capture_id"].startswith("hang-after-commit-"):
            hang_once(self.root, "apply-after:" + edit["edit_id"])
        return receipt

    def replace_document(self, expected_revision, expected_sha256, text):
        self.ready()
        if self.state is None:
            raise Err("document_missing", "initialize source first")
        doc = self.state["document"]
        if doc["revision"] != expected_revision or doc["source_sha256"] != expected_sha256:
            raise Err("document_conflict", "ordinary edit snapshot is stale")
        if "%ledger-crash-once" in text:
            crash_once(self.root, "replace:" + sha(text))
        state = json.loads(json.dumps(self.state))
        state["document"] = {"project_id": doc["project_id"], "path": doc["path"], "revision": expected_revision + 1,
                             "text": text, "source_sha256": sha(text)}
        self.commit(state)
        return state["document"]

    def confirm(self, receipt):
        self.ready()
        if self.state is None:
            raise Err("document_missing", "initialize source first")
        tx = self.state["transactions"].get(receipt["edit_id"])
        if tx is None:
            raise Err("edit_missing", "receipt does not name an applied edit")
        if tx["receipt"] != receipt:
            raise Err("receipt_conflict", "acknowledgement differs from durable receipt")
        if tx["confirmed"]:
            return
        state = json.loads(json.dumps(self.state))
        state["transactions"][receipt["edit_id"]]["confirmed"] = True
        state["transactions"][receipt["edit_id"]]["document_before"] = None
        self.commit(state)

    def recovery(self):
        self.ready()
        if self.state is None:
            return []
        pending = [t for t in self.state["transactions"].values() if not t["confirmed"]]
        pending.sort(key=lambda t: t["receipt"]["new_revision"])
        return pending


def main():
    args = sys.argv[1:]
    if len(args) != 2 or args[0] != "--store":
        print("usage: fake_edit_ledger.py --store PRIVATE_DIRECTORY", file=sys.stderr)
        sys.exit(2)
    store = Store(args[1])
    session_id = "%d-%d-1" % (os.getpid(), int(os.times().elapsed * 1e9))
    sequence = 0
    print("fake_edit_ledger: started (test double, not the real helper)", file=sys.stderr, flush=True)

    def reply(rid, payload=None, error=None):
        nonlocal sequence
        sequence += 1
        doc = store.state["document"] if store.state is not None and store.startup_error is None else None
        out = {"session_id": session_id, "sequence": sequence, "id": rid,
               "document_revision": doc["revision"] if doc else None,
               "document_sha256": doc["source_sha256"] if doc else None,
               "command_succeeded": error is None}
        if payload is not None:
            out["payload"] = payload
        if error is not None:
            out["error"] = error
        print(json.dumps(out), flush=True)

    for raw in sys.stdin.buffer:
        if len(raw) > MAX_LINE or not raw.endswith(b"\n"):
            print(json.dumps({"id": None, "error": {"code": "invalid_frame", "message": "expected newline-terminated line of at most 12 MiB"}}), flush=True)
            sys.exit(1)
        try:
            req = json.loads(raw)
        except ValueError as e:
            reply(None, error={"code": "invalid_request", "message": str(e)})
            continue
        rid = req.get("id")
        if not isinstance(rid, str) or not rid or len(rid.encode("utf-8")) > 128:
            reply(None, error={"code": "invalid_id", "message": "request ID must be 1-128 bytes"})
            continue
        try:
            op = req.get("operation")
            if op == "initialize":
                store.initialize(req["document"])
                payload = {"document": store.document()}
            elif op == "apply":
                receipt = store.apply(req["edit"])
                payload = {"receipt": receipt, "document": store.document()}
            elif op == "replace_document":
                payload = {"document": store.replace_document(req["expected_revision"], req["expected_sha256"], req["text"])}
            elif op == "confirm":
                store.confirm(req["receipt"])
                payload = {"confirmed": req["receipt"]}
            elif op == "status":
                payload = {"document": store.document(), "pending_receipts": store.recovery()}
            elif op == "recovery_export":
                payload = store.export_recovery()
            elif op == "recovery_import":
                payload = store.import_recovery(req["recovery"])
            elif op == "crash":
                crash()
            else:
                raise Err("invalid_request", "unknown operation %r" % op)
            reply(rid, payload=payload)
        except Err as e:
            reply(rid, error={"code": e.code, "message": e.message})
        except (KeyError, TypeError) as e:
            reply(rid, error={"code": "invalid_request", "message": "missing field: %s" % e})


if __name__ == "__main__":
    main()
