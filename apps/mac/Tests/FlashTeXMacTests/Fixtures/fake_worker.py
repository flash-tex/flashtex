#!/usr/bin/env python3
"""Test double for the Rust worker: runtime v1 JSON Lines over stdin/stdout.

Echoes each `compile` as a `compile_result` whose single text item maps the
first line of the entry document. Not a compiler. Supports a few directives in
the entry text for tests: `%error` -> error envelope, `%garbage` -> non-JSON line.
`%diag:<n>` (anywhere in the entry text, any number of times) -> one error
diagnostic per occurrence, spanning byte `pos+n ..< pos+n+1` where `pos` is the
directive's own UTF-8 byte offset, so a diagnostic after an insertion shifts with
the text the way a real compiler's would. `%slow` at the start -> the reply is
delayed 400 ms (for coalescing tests).

`%caps` at the start -> layout-capability negotiation (runtime-v1-layout-
capabilities.md): the reply echoes back the requested capabilities it knows
(`rules-v1`, `font-hints-v1`) as `layout_capabilities`, adds one `rule` item when
rules-v1 was accepted, and a bold Latin Modern `font` hint on the text item when
font-hints-v1 was accepted. Optional flags after a colon, comma-separated:
`blob` -> also emit an item of unknown kind `blob` with a source range;
`sub` -> the font hint asks for family "Comic Sans" (forces substitution);
`unrequested` -> emit the rule and font hint even when not requested (violation);
`claim` -> claim acceptance of `rules-v1` even when not requested (violation).
`%caps` never speeds up or slows down the reply; combine with `%slow` by
writing `%caps` first. Test double only, not a compiler.
"""
import json
import os
import re
import sys
import time

DIAG = re.compile(rb"%diag:(\d+)")

for raw in sys.stdin:
    raw = raw.strip()
    if not raw:
        continue
    env = json.loads(raw)
    if env.get("protocol_version") != 1 or env.get("type") != "compile":
        out = {"protocol_version": 1, "id": env.get("id", "?"), "type": "error",
               "payload": {"message": "unsupported envelope"}}
        print(json.dumps(out), flush=True)
        continue
    p = env["payload"]
    entry = next((d for d in p["documents"] if d["path"] == p["entry_path"]), None)
    text = entry["text"] if entry else ""
    if text.startswith("%crash"):
        # Simulates a worker crash mid-request: no reply, abnormal exit status.
        sys.stdout.flush()
        os._exit(3)
    if text.startswith("%error"):
        print(json.dumps({"protocol_version": 1, "id": env["id"], "type": "error",
                          "payload": {"message": "requested failure"}}), flush=True)
        continue
    if text.startswith("%garbage"):
        print("this is not json", flush=True)
        continue
    if text.startswith("%huge"):
        # One complete line larger than the shell's 16 MiB cap.
        sys.stdout.write('{"protocol_version":1,"id":"%s","type":"compile_result","payload":{"x":"' % env["id"])
        sys.stdout.write("x" * (16 * 1024 * 1024 + 64))
        sys.stdout.write('"}}\n')
        sys.stdout.flush()
        continue
    if text.startswith("%trailing"):
        # Partial JSON with no newline, then exit: unterminated bytes at EOF.
        sys.stdout.write('{"protocol_version":1,"id":"%s","type":"compile_result","pay' % env["id"])
        sys.stdout.flush()
        sys.exit(0)
    if text.startswith("%wrongid"):
        # Valid-looking result that answers a request nobody sent.
        env["id"] = "never-sent"
        text = text[len("%wrongid"):]
    if text.startswith("%wrongrev"):
        p["revision"] = p["revision"] + 1000
        text = text[len("%wrongrev"):]
    caps_flags, offset = None, 0
    if text.startswith("%caps"):
        head = text.split("\n", 1)[0][len("%caps"):]
        caps_flags = set(head[1:].split(",")) if head.startswith(":") else set()
        rest = text.split("\n", 1)[1] if "\n" in text else ""
        offset = len(text.encode("utf-8")) - len(rest.encode("utf-8"))  # keep spans document-relative
        text = rest
    if text.startswith("%slow"):
        time.sleep(0.4)
    first = text.split("\n", 1)[0]
    end = len(first.encode("utf-8"))
    diagnostics = []
    for m in DIAG.finditer(text.encode("utf-8")):
        at = m.start() + int(m.group(1))
        diagnostics.append({"severity": "error", "message": "fake diagnostic %s" % m.group(1).decode(),
                            "source": {"path": p["entry_path"], "start_byte": at, "end_byte": at + 1},
                            "recovery": "test double: byte skipped"})
    print("fake_worker: compiling revision %d" % p["revision"], file=sys.stderr, flush=True)
    out = {"protocol_version": 1, "id": env["id"], "type": "compile_result",
           "payload": {"project_id": p["project_id"], "revision": p["revision"], "status": "ok",
                       "pages": [{"number": 1, "width_pt": 612, "height_pt": 792,
                                  "items": [{"kind": "text", "text": first, "x_pt": 72,
                                             "baseline_y_pt": 84, "font_size_pt": 12,
                                             "source": {"path": p["entry_path"],
                                                        "start_byte": offset, "end_byte": offset + end}}]}],
                       "diagnostics": diagnostics, "pdf_path": None}}
    if caps_flags is not None:
        requested = p.get("layout_capabilities") or []
        accepted = [c for c in requested if c in ("rules-v1", "font-hints-v1")]
        if "claim" in caps_flags and "rules-v1" not in accepted:
            accepted.append("rules-v1")
        payload = out["payload"]
        if "layout_capabilities" in p or accepted:
            payload["layout_capabilities"] = accepted
        items = payload["pages"][0]["items"]
        src = items[0]["source"]
        if "rules-v1" in accepted or "unrequested" in caps_flags:
            items.append({"kind": "rule", "x_pt": 72, "y_pt": 90, "width_pt": 24, "height_pt": 0.5, "source": src})
        if "font-hints-v1" in accepted or "unrequested" in caps_flags:
            family = "Comic Sans" if "sub" in caps_flags else "Latin Modern Roman"
            items[0]["font"] = {"family": family, "weight": "bold", "style": "normal"}
        if "blob" in caps_flags:
            items.append({"kind": "blob", "source": src})
    print(json.dumps(out), flush=True)
