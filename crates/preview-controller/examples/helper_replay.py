#!/usr/bin/env python3
"""Bounded real-helper replay; temporary project only, no network/provider calls."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import tempfile
import time


class Client:
    def __init__(self, binary, config, capture_diagnostics=False, capture_wire=False):
        self.diagnostic_file = tempfile.TemporaryFile() if capture_diagnostics else None
        self.proc = subprocess.Popen([binary, str(config)], stdin=subprocess.PIPE,
                                     stdout=subprocess.PIPE, stderr=self.diagnostic_file or subprocess.DEVNULL)
        self.buffer = bytearray()
        self.capture_wire = capture_wire
        self.capture_diagnostics = capture_diagnostics
        self.last_wire = None
        self.last_read_timing = None
        self.receive_ordinal = 0
        self.receiver_timings = []
        self.receiver_timings_dropped = 0
        self.last_diagnostics_raw = b''
        self.diagnostics_status = dict(captured=False, scope="no diagnostic snapshot taken")

    def send(self, identity, kind, payload):
        value = dict(protocol_version=1, session_id="benchmark", id=identity,
                     type=kind, payload=payload)
        self.proc.stdin.write(json.dumps(value).encode() + b"\n")
        self.proc.stdin.flush()

    def read(self):
        read_started = time.monotonic()
        deadline = read_started + 15
        while b"\n" not in self.buffer:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not select.select([self.proc.stdout], [], [], remaining)[0]:
                raise RuntimeError("helper response deadline")
            data = os.read(self.proc.stdout.fileno(), 65536)
            if not data:
                raise RuntimeError("helper EOF")
            self.buffer.extend(data)
            if len(self.buffer) > 17 * 1024 * 1024:
                raise RuntimeError("helper read buffer bound")
        line, _, tail = self.buffer.partition(b"\n")
        self.buffer = bytearray(tail)
        received = time.monotonic()
        if self.capture_wire:
            self.last_wire = bytes(line) + b"\n"
        decode_started = time.monotonic()
        decoded = json.loads(line)
        self.receive_ordinal += 1
        decode_finished = time.monotonic()
        if self.capture_diagnostics:
            self.last_read_timing = dict(sequence=self.receive_ordinal, read_started=read_started, frame_received=received,
                decode_started=decode_started,
                decode_finished=decode_finished, read_ms=(received-read_started)*1000,
                decode_ms=(decode_finished-decode_started)*1000, bytes=len(line)+1)
            if len(self.receiver_timings) < 4096:
                self.receiver_timings.append(self.last_read_timing)
            else:
                self.receiver_timings_dropped += 1
        return decoded, received

    def memory_snapshot(self):
        status = Path(f"/proc/{self.proc.pid}/status")
        if not status.exists():
            return None
        wanted = {"VmRSS", "VmHWM", "Threads"}
        return {name: value.strip() for line in status.read_text().splitlines()
                if ":" in line for name, value in [line.split(":", 1)] if name in wanted}

    def diagnostic_bytes(self):
        if self.diagnostic_file is None:
            return b''
        # stderr and this file share an open-file description. Positional reads
        # must not seek the helper's concurrently used write offset.
        raw = os.pread(self.diagnostic_file.fileno(), 1024 * 1024 + 1, 0)
        self.last_diagnostics_raw = raw
        self.diagnostics_status = dict(captured=True, truncated=len(raw)>1024*1024,
            partial_tail=bool(raw and not raw.endswith(b'\n')), non_json=False,
            scope='live positional snapshot; later stderr may be absent')
        return raw

    def diagnostics(self):
        raw = self.diagnostic_bytes()
        if len(raw) > 1024 * 1024:
            raise RuntimeError("diagnostic capture limit")
        # A concurrent write can leave a partial final line in this snapshot.
        # Retain it verbatim and report it, but parse only completed records.
        complete = raw[:raw.rfind(b'\n')+1]
        try:
            return [json.loads(line) for line in complete.splitlines() if line]
        except (ValueError, UnicodeDecodeError):
            self.diagnostics_status['non_json'] = True
            raise

    def stop(self):
        self.proc.kill()
        self.proc.wait(timeout=5)
        self.proc.stdin.close()
        self.proc.stdout.close()
        if self.diagnostic_file is not None:
            self.diagnostic_file.close()


def snapshot_after_initial_preview(client):
    assert client.read()[0]["type"] == "ready"
    client.send("snapshot-document", "document", dict(path="main.tex"))
    document = None
    preview = False
    while document is None or not preview:
        event, _ = client.read()
        if event.get("type") == "error":
            raise RuntimeError(event.get("payload", {}).get("message", "helper startup error"))
        if event.get("payload", {}).get("kind") == "failed":
            raise RuntimeError(event["payload"]["reason"])
        if event.get("id") == "snapshot-document":
            document = event["payload"]["document"]
        if event.get("payload", {}).get("kind") == "preview":
            preview = True
    return document


def lost_reply_recovery(helper, config):
    client = Client(helper, config)
    try:
        before = snapshot_after_initial_preview(client)
        edited = before["text"].replace("\\end{document}", "Lost reply recovery.\n\\end{document}")
        assert edited != before["text"]
        payload = dict(path="main.tex", expected_revision=before["revision"],
                       expected_sha256=before["source_sha256"], text=edited)
        assert not client.buffer
        client.send("lost-edit", "edit", payload)
        # Observe queued output without consuming the acknowledgement, then kill.
        # Reopened state below proves whether this particular edit was durable.
        if not select.select([client.proc.stdout], [], [], 15)[0]:
            raise RuntimeError("lost-reply output deadline")
    finally:
        client.stop()
    client = Client(helper, config)
    try:
        recovered = snapshot_after_initial_preview(client)
        assert recovered["text"] == edited
        assert recovered["revision"] == before["revision"] + 1
        assert recovered["source_sha256"] == hashlib.sha256(edited.encode()).hexdigest()
        client.send("lost-edit", "edit", payload)
        while True:
            event, _ = client.read()
            if event.get("id") == "lost-edit":
                assert event["type"] == "error", "stale full-source retry must not apply twice"
                break
        client.send("after-retry", "document", dict(path="main.tex"))
        while True:
            event, _ = client.read()
            if event.get("id") == "after-retry":
                assert event["payload"]["document"] == recovered
                break
    finally:
        client.stop()
    return dict(unread_ack_kill=True, exact_recovered_source=True,
                revision_advanced_once=True, stale_retry_rejected=True,
                retry_preserved_document=True)


def run(args):
    helper = str(Path(args.helper).resolve())
    compiler = str(Path(args.compiler).resolve())
    prefix = "\\documentclass{article}\n\\begin{document}\n"
    text = prefix + ("Alpha beta gamma delta.\n" * (args.size // 23)) + "\\end{document}\n"
    with tempfile.TemporaryDirectory(prefix="flashtex-helper-bench-") as directory:
        root = Path(directory)
        project = root / "project"
        project.mkdir()
        (root / "ledger").mkdir()
        (project / "main.tex").write_text(text)
        config = root / "config.json"
        config.write_text(json.dumps(dict(session_id="benchmark", project_id="p",
            entry_path="main.tex", project_root=str(project),
            private_ledger_root=str(root / "ledger"), compiler_path=compiler,
            compiler_max_frame_bytes=12 * 1024 * 1024)))
        client = Client(helper, config)
        document = None
        try:
            assert client.read()[0]["type"] == "ready"
            client.send("document", "document", dict(path="main.tex"))
            initial_preview = False
            while document is None or not initial_preview:
                event, _ = client.read()
                if event.get("id") == "document":
                    document = event["payload"]["document"]
                if event.get("payload", {}).get("kind") == "preview":
                    initial_preview = True
            for index in range(args.edits):
                edited = text.replace("Alpha", "Omega" if index % 2 == 0 else "Sigma", 1)
                identity = f"edit-{index}"
                started = time.monotonic()
                client.send(identity, "edit", dict(path="main.tex",
                    expected_revision=document["revision"],
                    expected_sha256=document["source_sha256"], text=edited))
                acknowledgement = preview = None
                while acknowledgement is None or preview is None:
                    event, received = client.read()
                    if event.get("type") == "error":
                        raise RuntimeError(event["payload"]["message"])
                    if event.get("id") == identity:
                        acknowledgement = event["payload"]
                        document = acknowledgement["document"]
                        ack_ms = (received - started) * 1000
                    payload = event.get("payload", {})
                    if payload.get("kind") == "failed":
                        raise RuntimeError(payload["reason"])
                    if payload.get("kind") == "preview":
                        preview = payload
                        wire_ms = (received - started) * 1000
                assert document["text"] == edited
                assert preview["source_versions"] == {"main.tex": document["revision"]}
                result = preview["result"]
                request = dict(protocol_version=1, id=result["id"], type="compile", payload=dict(
                    project_id="p", revision=result["payload"]["revision"],
                    entry_path="main.tex", documents=[dict(path="main.tex", text=edited)]))
                fresh = subprocess.run([compiler], input=json.dumps(request).encode() + b"\n",
                                       capture_output=True, timeout=15, check=True)
                assert json.loads(fresh.stdout) == result
                print(json.dumps(dict(type="sample", index=index, source_bytes=len(edited.encode()),
                    ack_wire_ms=ack_ms, preview_wire_ms=wire_ms,
                    save_and_submit_ms=acknowledgement["save_and_submit_ms"],
                    runtime_total_ms=preview["runtime_total_ms"],
                    controller_total_ms=preview["controller_total_ms"], exact_clean_equal=True)), flush=True)
        finally:
            client.stop()
        # Kill/reopen proves the measured acknowledged edits remain authoritative.
        client = Client(helper, config)
        try:
            assert client.read()[0]["type"] == "ready"
            client.send("reopen", "document", dict(path="main.tex"))
            while True:
                event, _ = client.read()
                if event.get("id") == "reopen":
                    assert event["payload"]["document"] == document
                    break
        finally:
            client.stop()
        recovery = lost_reply_recovery(helper, config) if args.lost_reply else None
        print(json.dumps(dict(type="summary", edits=args.edits, exact_reopen=True,
            helper_sha256=hashlib.sha256(Path(helper).read_bytes()).hexdigest(),
            compiler_sha256=hashlib.sha256(Path(compiler).read_bytes()).hexdigest(),
            native_paint_measured=False, lost_reply_retry_measured=args.lost_reply,
            lost_reply_evidence=recovery)))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--helper", required=True)
    parser.add_argument("--compiler", required=True)
    parser.add_argument("--size", type=int, default=5000)
    parser.add_argument("--edits", type=int, default=5)
    parser.add_argument("--lost-reply", action="store_true",
                        help="kill before reading an edit acknowledgement and verify recovery/retry")
    args = parser.parse_args()
    if not 100 <= args.size <= 500000 or not 1 <= args.edits <= 20:
        parser.error("size must be100..500000 and edits1..20")
    run(args)
