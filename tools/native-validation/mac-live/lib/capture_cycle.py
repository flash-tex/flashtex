#!/usr/bin/env python3
"""Packaged-app capture cycle through the helpers bundled in FlashTeX.app.

Drives the four binaries inside `FlashTeX.app/Contents/MacOS` (flashtex-bridge,
flashtex-edit-ledger, flashtex-compiler, flashtex-pdf) over their real JSON Lines
protocols, in the order the Mac shell uses them (transfer-v1 / runtime-v1 /
edit-ledger service), without any UI scripting (Accessibility is not granted and
the app window must never be activated) and without any network call:

  1. bridge: document_open -> destination_pin -> capture_submit (the shared
     protocol/fixtures/capture-submission.json image) -> duplicate retry ->
     conflicting retry -> capture_convert (must be refused `provider_disabled`:
     the bridge is launched without --enable-grok, exactly as make-app bundles
     it) -> capture_status -> capture_prepare_insert (must be refused
     `proposal_missing`) -> SIGKILL the bridge -> restart on the same journal
     -> capture_status still durable -> capture_reject -> terminal.
  2. offline proposal review: a fixture capture_proposal (fixtures/) is checked
     against the transfer-v1 bounds and "approved" at the pinned destination;
     the durable insertion goes through flashtex-edit-ledger: initialize ->
     apply -> identical retry (same receipt, no second insertion) -> status
     (receipt pending: no bridge ever prepared this edit, so no capture_applied
     is sent) -> SIGKILL the ledger -> restart -> document and pending receipt
     survive.
  3. export: the post-insertion document is compiled by flashtex-compiler
     (runtime-v1 `compile`) and the compile_result is piped to
     `flashtex-pdf --out <pdf> --verify`; page count and sha256 are recorded.

Every request/reply is journaled to <work>/transcript.jsonl. Stdlib only.
"""
import argparse
import base64
import hashlib
import json
import os
import queue
import re
import shutil
import signal
import subprocess
import sys
import threading
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from hashes import describe  # noqa: E402


class LineProcess:
    """One JSON Lines child; replies are read on a thread with a timeout."""

    def __init__(self, name, argv, transcript, env=None):
        self.name = name
        self.argv = argv
        self.transcript = transcript
        self.proc = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                     env=env, cwd=os.path.dirname(argv[0]))
        self.q = queue.Queue()
        self.stderr = []
        threading.Thread(target=self._reader, daemon=True).start()
        threading.Thread(target=self._stderr, daemon=True).start()
        self.transcript.write(json.dumps({"process": name, "pid": self.proc.pid, "argv": argv, "t": time.time()}) + "\n")

    def _reader(self):
        for line in self.proc.stdout:
            self.q.put(line)
        self.q.put(None)

    def _stderr(self):
        for line in self.proc.stderr:
            self.stderr.append(line.decode("utf-8", "replace").rstrip())

    def send(self, obj, timeout=30):
        line = json.dumps(obj, separators=(",", ":")) + "\n"
        self.transcript.write(json.dumps({"to": self.name, "request": obj}, default=str) + "\n")
        t0 = time.monotonic()
        self.proc.stdin.write(line.encode("utf-8"))
        self.proc.stdin.flush()
        try:
            raw = self.q.get(timeout=timeout)
        except queue.Empty:
            raise RuntimeError("%s: no reply within %ss for %s" % (self.name, timeout, obj.get("type") or obj.get("operation")))
        if raw is None:
            raise RuntimeError("%s: exited (code %s) before replying; stderr tail: %s" % (self.name, self.proc.poll(), self.stderr[-5:]))
        ms = (time.monotonic() - t0) * 1000
        reply = json.loads(raw)
        self.transcript.write(json.dumps({"from": self.name, "ms": round(ms, 3), "reply": reply}) + "\n")
        return reply, ms

    def kill(self):
        self.proc.send_signal(signal.SIGKILL)
        try:
            self.proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            pass
        self.transcript.write(json.dumps({"process": self.name, "pid": self.proc.pid, "sigkill": True, "exit": self.proc.returncode}) + "\n")
        return self.proc.returncode

    def close(self):
        try:
            self.proc.stdin.close()
        except Exception:
            pass
        try:
            self.proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.proc.kill()
        self.transcript.write(json.dumps({"process": self.name, "pid": self.proc.pid, "closed": True, "exit": self.proc.returncode}) + "\n")


def sha256_text(s):
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def sha256_file(p):
    return hashlib.sha256(open(p, "rb").read()).hexdigest()


class Cycle:
    def __init__(self, a):
        self.a = a
        self.macos = os.path.join(a.app, "Contents", "MacOS")
        self.bins = {n: os.path.join(self.macos, n) for n in ("flashtex-bridge", "flashtex-edit-ledger", "flashtex-compiler", "flashtex-pdf")}
        os.makedirs(a.work, exist_ok=True)
        self.transcript = open(os.path.join(a.work, "transcript.jsonl"), "w", encoding="utf-8")
        self.checks = []
        self.timings = {}
        # It drives the bundled helpers directly, the compiler among them, so
        # every result is the previous engine's (retirement plan #1236, S3r).
        self.record = {"app": a.app, "engine": "previous", "binaries": {}, "checks": self.checks, "timings_ms": self.timings, "network": {}}
        for n, p in self.bins.items():
            self.record["binaries"][n] = describe(p)

    def check(self, name, ok, detail=""):
        self.checks.append({"name": name, "ok": bool(ok), "detail": str(detail)[:600]})
        print("    %s %s%s" % ("PASS" if ok else "FAIL", name, (" -- " + str(detail)[:200]) if detail else ""))
        return ok

    def env(self):
        # No XAI_API_KEY and no proxy: even a misconfigured bridge cannot reach a provider.
        return {k: v for k, v in os.environ.items() if not k.startswith(("XAI_", "HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy"))}

    # ------------------------------------------------------------ 1. bridge
    def bridge_stage(self):
        a = self.a
        req = json.load(open(os.path.join(a.fixtures, "compile-request.json")))
        fixture_sub = json.load(open(os.path.join(a.fixtures, "capture-submission.json")))
        project = req["payload"]["project_id"]
        path = req["payload"]["entry_path"]
        text = next(d["text"] for d in req["payload"]["documents"] if d["path"] == path)
        self.project, self.path, self.text = project, path, text
        self.pin_byte = len(text.encode("utf-8"))  # end of the fixture document
        store = os.path.join(a.work, "captures")
        os.makedirs(store, exist_ok=True)
        argv = [self.bins["flashtex-bridge"], "--store", store]
        self.record["bridge_argv"] = argv
        self.record["network"]["bridge_enable_grok"] = "--enable-grok" in argv
        self.record["network"]["xai_api_key_in_env"] = "XAI_API_KEY" in self.env()
        b = LineProcess("bridge", argv, self.transcript, env=self.env())
        n = [0]

        def ask(kind, payload, timeout=30):
            n[0] += 1
            rid = "mac-live-%s-%d" % (kind, n[0])
            reply, ms = b.send({"protocol_version": 1, "id": rid, "type": kind, "payload": payload}, timeout)
            self.timings.setdefault("bridge_" + kind, []).append(round(ms, 3))
            if reply.get("id") != rid:
                raise RuntimeError("bridge reply id %r != %r" % (reply.get("id"), rid))
            return reply

        r = ask("document_open", {"project_id": project, "path": path, "revision": 1, "text": text})
        self.check("bridge document_open", r["type"] == "document_opened", r)
        r = ask("destination_pin", {"destination_id": "mac-live-dest-1", "project_id": project, "path": path, "revision": 1,
                                    "start_byte": self.pin_byte, "end_byte": self.pin_byte})
        anchor = r.get("payload", {})
        self.check("bridge destination_pinned", r["type"] == "destination_pinned" and anchor.get("valid") is True, r)
        self.check("bridge anchor binding sha256 == sha256(document)", anchor.get("binding", {}).get("source_sha256") == sha256_text(text),
                   anchor.get("binding", {}).get("source_sha256"))
        self.destination = anchor
        submit = dict(fixture_sub["payload"])
        submit["capture_id"] = "mac-live-capture-1"
        submit["destination_id"] = "mac-live-dest-1"
        submit["base_revision"] = anchor.get("pinned_revision", 1)
        self.record["capture_image"] = {"mime_type": submit["image"]["mime_type"],
                                        "encoded_bytes": len(submit["image"]["data_base64"]),
                                        "decoded_bytes": len(base64.b64decode(submit["image"]["data_base64"])),
                                        "sha256": hashlib.sha256(base64.b64decode(submit["image"]["data_base64"])).hexdigest(),
                                        "source": "protocol/fixtures/capture-submission.json"}
        r = ask("capture_submit", submit)
        p = r.get("payload", {})
        received = r["type"] == "capture_received" and p.get("durable") is True and p.get("has_proposal") is False and p.get("applied") is False
        self.check("bridge capture_received durable (no proposal, not applied)", received, r)
        journal = os.path.join(store, "mac-live-capture-1.json")
        self.check("bridge journal file exists after receipt", os.path.isfile(journal), journal)
        self.record["journal_file"] = journal
        r2 = ask("capture_submit", submit)
        self.check("bridge identical retry returns the same receipt", r2.get("payload") == p, r2)
        conflict = dict(submit)
        conflict["instructions"] = submit["instructions"] + " (different)"
        r3 = ask("capture_submit", conflict)
        self.check("bridge conflicting retry -> capture_id_conflict",
                   r3["type"] == "error" and r3.get("payload", {}).get("code") == "capture_id_conflict", r3)
        r4 = ask("capture_convert", {"capture_id": "mac-live-capture-1", "supported_features": []}, timeout=95)
        code = r4.get("payload", {}).get("code")
        self.record["convert_error"] = r4.get("payload")
        self.check("bridge capture_convert refused as provider_disabled (no --enable-grok, no network)",
                   r4["type"] == "error" and code == "provider_disabled", r4)
        r5 = ask("capture_status", {"capture_id": "mac-live-capture-1"})
        s = r5.get("payload", {})
        self.check("bridge capture_status: no proposal / prepared / applied, not rejected",
                   r5["type"] == "capture_status" and s.get("proposal") is None and s.get("prepared") is None
                   and s.get("applied") is None and s.get("rejected") is False, r5)
        r6 = ask("capture_prepare_insert", {"capture_id": "mac-live-capture-1", "expected_revision": 1, "approved": True})
        self.check("bridge capture_prepare_insert without proposal -> proposal_missing",
                   r6["type"] == "error" and r6.get("payload", {}).get("code") == "proposal_missing", r6)
        # Crash: SIGKILL the bridge, restart on the same journal.
        pid1 = b.proc.pid
        code1 = b.kill()
        self.record["bridge_sigkill"] = {"pid": pid1, "exit": code1}
        self.check("bridge SIGKILL observed (exit -9)", code1 == -9, code1)
        b = LineProcess("bridge-restarted", argv, self.transcript, env=self.env())
        self.record["bridge_restart_pid"] = b.proc.pid
        r7 = ask("capture_status", {"capture_id": "mac-live-capture-1"})
        s = r7.get("payload", {})
        self.check("bridge journal survives SIGKILL + restart (capture_status durable)",
                   r7["type"] == "capture_status" and s.get("rejected") is False and s.get("proposal") is None, r7)
        # Anchors/documents are in memory (transfer-v1): reopening is the Mac's job after restart.
        r8 = ask("document_open", {"project_id": project, "path": path, "revision": 1, "text": text})
        self.check("bridge document reopened after restart", r8["type"] == "document_opened", r8)
        r9 = ask("capture_reject", {"capture_id": "mac-live-capture-1"})
        self.check("bridge capture_reject durable", r9["type"] == "capture_rejected", r9)
        r10 = ask("capture_status", {"capture_id": "mac-live-capture-1"})
        self.check("bridge capture_status rejected == true", r10.get("payload", {}).get("rejected") is True, r10)
        r11 = ask("capture_prepare_insert", {"capture_id": "mac-live-capture-1", "expected_revision": 1, "approved": True})
        self.check("bridge prepare after reject -> capture_rejected (terminal)",
                   r11["type"] == "error" and r11.get("payload", {}).get("code") == "capture_rejected", r11)
        r12 = ask("capture_status", {"capture_id": "never-submitted"})
        self.check("bridge unknown capture -> capture_missing",
                   r12["type"] == "error" and r12.get("payload", {}).get("code") == "capture_missing", r12)
        b.close()
        self.record["bridge_stderr_tail"] = b.stderr[-10:]
        self.check("bridge stderr has no provider/network line",
                   not any(re.search(r"x\.ai|https?://|reqwest|grok", l, re.I) for l in b.stderr), b.stderr[-3:])

    # ------------------------------------------------ 2. offline review + ledger
    def review_stage(self):
        a = self.a
        env = json.load(open(a.proposal))
        prop = env["payload"]
        latex = prop["latex"]
        ok = (env.get("type") == "capture_proposal" and env.get("protocol_version") == 1
              and 1 <= len(latex.encode("utf-8")) <= 65536
              and len(prop.get("ambiguities", [])) <= 32 and len(prop.get("required_dependencies", [])) <= 32
              and all(len(s.encode("utf-8")) <= 2048 for s in prop.get("ambiguities", []) + prop.get("required_dependencies", [])))
        self.check("review: fixture capture_proposal within transfer-v1 bounds", ok, {"latex_bytes": len(latex.encode("utf-8")),
                   "ambiguities": prop.get("ambiguities"), "required_dependencies": prop.get("required_dependencies")})
        self.record["proposal"] = {"file": a.proposal, "capture_id": prop["capture_id"], "latex": latex,
                                   "ambiguities": prop.get("ambiguities"), "required_dependencies": prop.get("required_dependencies"),
                                   "reviewed": "offline fixture; explicitly not a provider result"}
        # Approval = insert the journaled LaTeX unchanged at the pinned destination.
        before = self.text
        start = end = self.pin_byte
        after = before.encode("utf-8")[:start].decode("utf-8") + latex + before.encode("utf-8")[end:].decode("utf-8")
        self.after_text = after
        edit = {"capture_id": prop["capture_id"], "edit_id": "mac-live-edit-1", "project_id": self.project, "path": self.path,
                "expected_revision": 1, "start_byte": start, "end_byte": end, "removed_text": "", "replacement": latex,
                "document_before_sha256": sha256_text(before)}
        self.record["prepared_edit"] = edit
        store = os.path.join(a.work, "ledger")
        os.makedirs(store, exist_ok=True)
        argv = [self.bins["flashtex-edit-ledger"], "--store", store]
        self.record["ledger_argv"] = argv
        led = LineProcess("ledger", argv, self.transcript)
        n = [0]

        def ask(op, extra=None, timeout=30):
            n[0] += 1
            rid = "mac-live-%s-%d" % (op, n[0])
            reply, ms = led.send(dict({"id": rid, "operation": op}, **(extra or {})), timeout)
            self.timings.setdefault("ledger_" + op, []).append(round(ms, 3))
            if reply.get("id") != rid:
                raise RuntimeError("ledger reply id %r != %r" % (reply.get("id"), rid))
            return reply

        doc = {"project_id": self.project, "path": self.path, "revision": 1, "text": before, "source_sha256": sha256_text(before)}
        r = ask("initialize", {"document": doc})
        self.check("ledger initialize", r.get("command_succeeded") is True and r.get("document_revision") == 1, r)
        r = ask("apply", {"edit": edit})
        receipt = (r.get("payload") or {}).get("receipt")
        applied_doc = (r.get("payload") or {}).get("document") or {}
        self.check("ledger apply -> receipt + durable document with the insertion",
                   r.get("command_succeeded") is True and receipt is not None and applied_doc.get("text") == after
                   and applied_doc.get("source_sha256") == sha256_text(after), r)
        self.record["ledger_receipt"] = receipt
        self.new_revision = applied_doc.get("revision")
        r2 = ask("apply", {"edit": edit})
        self.check("ledger identical retry returns the same receipt without a second insertion",
                   (r2.get("payload") or {}).get("receipt") == receipt and ((r2.get("payload") or {}).get("document") or {}).get("text") == after, r2)
        r3 = ask("status")
        pend = (r3.get("payload") or {}).get("pending_receipts") or []
        self.check("ledger status: one pending (unconfirmed) receipt for the edit",
                   len(pend) == 1 and pend[0].get("receipt") == receipt and pend[0].get("confirmed") is False, r3)
        pid = led.proc.pid
        code = led.kill()
        self.record["ledger_sigkill"] = {"pid": pid, "exit": code}
        self.check("ledger SIGKILL observed (exit -9)", code == -9, code)
        led = LineProcess("ledger-restarted", argv, self.transcript)
        r4 = ask("status")
        d = (r4.get("payload") or {}).get("document") or {}
        pend = (r4.get("payload") or {}).get("pending_receipts") or []
        self.check("ledger document + pending receipt survive SIGKILL + restart",
                   d.get("text") == after and d.get("revision") == self.new_revision and len(pend) == 1 and pend[0].get("receipt") == receipt, r4)
        led.close()
        self.record["ledger_document_after"] = {"revision": d.get("revision"), "source_sha256": d.get("source_sha256"), "bytes": len(after.encode("utf-8"))}
        self.record["bridge_capture_applied"] = "not sent: the bridge never prepared this edit (no provider proposal); the receipt stays pending in the ledger by design"

    # ---------------------------------------------------------- 3. export
    def export_stage(self):
        a = self.a
        argv = [self.bins["flashtex-compiler"]]
        comp = LineProcess("compiler", argv, self.transcript)
        req = {"protocol_version": 1, "id": "mac-live-compile-1", "type": "compile",
               "payload": {"project_id": self.project, "revision": self.new_revision or 2, "entry_path": self.path,
                           "documents": [{"path": self.path, "text": self.after_text}]}}
        r, ms = comp.send(req, timeout=60)
        self.timings["compiler_compile"] = [round(ms, 3)]
        comp.close()
        p = r.get("payload") or {}
        pages = p.get("pages") or []
        diags = p.get("diagnostics") or []
        self.record["compile"] = {"status": p.get("status"), "pages": len(pages), "items": sum(len(pg.get("items", [])) for pg in pages),
                                  "diagnostics": diags[:10], "diagnostic_count": len(diags), "revision": p.get("revision"),
                                  "round_trip_ms": round(ms, 3)}
        self.check("compiler compile_result correlated (id, project, revision)",
                   r.get("type") == "compile_result" and r.get("id") == "mac-live-compile-1" and p.get("project_id") == self.project
                   and p.get("revision") == req["payload"]["revision"], {k: r.get(k) for k in ("id", "type")})
        self.check("compiler status ok/recovered with >= 1 page", p.get("status") in ("ok", "recovered") and len(pages) >= 1,
                   {"status": p.get("status"), "pages": len(pages), "diagnostics": len(diags)})
        texts = " ".join(str(it.get("text", "")) for pg in pages for it in pg.get("items", []))
        self.check("compiled items include the original fixture text", "Hello" in texts, texts[:120])
        self.record["compile"]["item_text_excerpt"] = texts[:200]
        json.dump(r, open(os.path.join(a.work, "compile_result.json"), "w"))
        pdf = os.path.join(a.work, "export.pdf")
        argv = [self.bins["flashtex-pdf"], "--out", pdf, "--verify"]
        self.record["pdf_argv"] = argv
        t0 = time.monotonic()
        proc = subprocess.run(argv, input=(json.dumps(r, separators=(",", ":")) + "\n").encode("utf-8"), capture_output=True, timeout=60)
        ms = (time.monotonic() - t0) * 1000
        self.timings["pdf_export"] = [round(ms, 3)]
        out = (proc.stdout + proc.stderr).decode("utf-8", "replace")
        self.transcript.write(json.dumps({"process": "pdf", "argv": argv, "exit": proc.returncode, "output": out[-2000:]}) + "\n")
        self.check("flashtex-pdf --verify exit 0", proc.returncode == 0, out[-300:])
        data = open(pdf, "rb").read() if os.path.isfile(pdf) else b""
        page_objs = len(re.findall(rb"/Type\s*/Page\b(?!s)", data))
        self.record["pdf"] = {"path": pdf, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest() if data else None,
                              "header": data[:8].decode("latin-1") if data else None, "page_objects": page_objs,
                              "writer_output": out[-800:], "export_ms": round(ms, 3)}
        self.check("PDF header %PDF- and trailer %%EOF", data.startswith(b"%PDF-") and b"%%EOF" in data[-64:], self.record["pdf"]["header"])
        self.check("PDF page objects == compiled pages", page_objs == len(pages), {"pdf": page_objs, "compiled": len(pages)})

    def run(self):
        stages = [("bridge", self.bridge_stage), ("review", self.review_stage), ("export", self.export_stage)]
        for name, fn in stages:
            print("==> capture cycle: %s" % name)
            try:
                fn()
            except Exception as e:  # keep going so the report shows every stage
                self.check("%s stage completed without driver error" % name, False, repr(e))
        self.transcript.close()
        self.record["passed"] = sum(1 for c in self.checks if c["ok"])
        self.record["failed"] = sum(1 for c in self.checks if not c["ok"])
        self.record["transcript"] = os.path.join(self.a.work, "transcript.jsonl")
        json.dump(self.record, open(self.a.out, "w"), indent=1, sort_keys=True, default=str)
        print("capture cycle: %d passed, %d failed" % (self.record["passed"], self.record["failed"]))
        # Private journals are scratch: keep the transcript, PDF and compile result only.
        for d in ("captures", "ledger"):
            shutil.rmtree(os.path.join(self.a.work, d), ignore_errors=True)
        return 0 if self.record["failed"] == 0 else 1


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--app", required=True, help="packaged FlashTeX.app")
    ap.add_argument("--fixtures", required=True, help="protocol/fixtures directory")
    ap.add_argument("--proposal", required=True, help="fixture capture_proposal envelope JSON")
    ap.add_argument("--work", required=True, help="scratch directory (journals, transcript, pdf)")
    ap.add_argument("--out", required=True, help="JSON summary path")
    a = ap.parse_args()
    return Cycle(a).run()


if __name__ == "__main__":
    sys.exit(main())
