#!/usr/bin/env python3
"""Test double for a runtime-v1 producer that speaks the negotiated
`display-list-v2` capability (docs/contracts/runtime-v1-display-list-v2.md).
Not a compiler: the compile_result carries one text item for the first line of
the entry text; the display_list sibling line is the rendering-v2 envelope given
as argv[1] (a real `flashtex-render --v2` fixture) rebound to the request: same
`id`, `project_id`, `revision`, and the entry document's revision, SHA-256 and
byte length.

Directives at the start of the entry text (test-only):
  %v2nocap       behave like an old producer: never accept the capability, no line
  %v2failed      status failed, capability accepted, NO display_list line
  %v2stale       the display_list line carries id "never-sent" (stale/unknown)
  %v2mismatch    the display_list payload claims revision + 1000 (correlation mismatch)
  %v2unsolicited emit the line WITHOUT echoing acceptance (protocol violation)
  %v2decline     decline per request: not echoed, warning diagnostic, no line
  %v2window      simulate a document too long for an unwindowed reply
                 (display-list-v2-window proposal): without a positioned
                 window request the compile_result is the producer's
                 over-the-reply-limit failure; with `display-list-v2-window`
                 requested AND `display_list_window` present, the sibling is
                 the template page replicated to 40 document pages, resident
                 only inside the (clamped) window, with the `window` object
                 and the echoed capability.
"""
import hashlib
import json
import sys

CAP = "display-list-v2"
WINDOW_CAP = "display-list-v2-window"
WINDOW_DOC_PAGES = 40
template = json.load(open(sys.argv[1], encoding="utf-8"))

for raw in sys.stdin:
    raw = raw.strip()
    if not raw:
        continue
    env = json.loads(raw)
    if env.get("protocol_version") != 1 or env.get("type") != "compile":
        print(json.dumps({"protocol_version": 1, "id": env.get("id", "?"), "type": "error",
                          "payload": {"message": "unsupported envelope"}}), flush=True)
        continue
    p = env["payload"]
    entry = next((d for d in p["documents"] if d["path"] == p["entry_path"]), None)
    text = entry["text"] if entry else ""
    directive = text.split("\n", 1)[0] if text.startswith("%v2") else ""
    requested = p.get("layout_capabilities") or []
    accept = CAP in requested and directive not in ("%v2nocap", "%v2unsolicited", "%v2decline")
    status = "failed" if directive == "%v2failed" else "ok"
    first = text.split("\n", 1)[0]
    result = {"project_id": p["project_id"], "revision": p["revision"], "status": status,
              "pages": [] if status == "failed" else [{"number": 1, "width_pt": 612, "height_pt": 792, "items": [
                  {"kind": "text", "text": first, "x_pt": 72, "baseline_y_pt": 84, "font_size_pt": 12,
                   "source": {"path": p["entry_path"], "start_byte": 0, "end_byte": len(first.encode("utf-8"))}}]}],
              "diagnostics": [], "pdf_path": None}
    if directive == "%v2failed":
        result["diagnostics"] = [{"severity": "error", "message": "requested failure", "source": None, "recovery": None}]
    if directive == "%v2decline" and CAP in requested:
        result["diagnostics"] = [{"severity": "warning", "message": "display-list-v2 declined: envelope would exceed the line limit (test)", "source": None, "recovery": None}]
    window = None
    if directive == "%v2window" and accept:
        req_window = p.get("display_list_window")
        if WINDOW_CAP in requested and req_window:
            total = WINDOW_DOC_PAGES
            count = min(req_window["page_count"], total)
            first = max(1, min(req_window["first_page"], total - count + 1))
            window = {"first_page": first, "page_count": count, "document_page_count": total}
        else:
            # The producer's actual over-limit refusal (protocol.rs failed()).
            result["status"] = "failed"
            result["pages"] = []
            result["diagnostics"] = [{"severity": "error",
                                      "message": "compile_result would be 20339674 bytes for %d pages, over the 16777216-byte reply limit; split the project or compile fewer pages" % WINDOW_DOC_PAGES,
                                      "source": None, "recovery": None}]
            status = "failed"
    if accept:
        result["layout_capabilities"] = [CAP] + ([WINDOW_CAP] if window else [])
    print(json.dumps({"protocol_version": 1, "id": env["id"], "type": "compile_result", "payload": result}), flush=True)
    emit_line = (accept and status != "failed") or directive == "%v2unsolicited"
    if not emit_line:
        continue
    v2 = json.loads(json.dumps(template))
    v2["id"] = "never-sent" if directive == "%v2stale" else env["id"]
    payload = v2["payload"]
    payload["project_id"] = p["project_id"]
    payload["revision"] = p["revision"] + (1000 if directive == "%v2mismatch" else 0)
    data = text.encode("utf-8")
    for doc in payload["documents"]:
        if doc["path"] == p["entry_path"]:
            doc["revision"] = p["revision"]
            doc["sha256"] = hashlib.sha256(data).hexdigest()
            doc["byte_length"] = len(data)
    if window:
        model = payload["pages"][0]
        pages = []
        for number in range(1, window["document_page_count"] + 1):
            if window["first_page"] <= number < window["first_page"] + window["page_count"]:
                page = json.loads(json.dumps(model))
                page["number"] = number
                pages.append(page)
            else:
                pages.append({"number": number, "width": model["width"], "height": model["height"],
                              "resident": False})
        payload["pages"] = pages
        payload["window"] = window
    print(json.dumps(v2, ensure_ascii=False), flush=True)
