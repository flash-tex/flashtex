#!/usr/bin/env python3
"""Positions checker for display-list-v3 (DESIGN.md §6.1, lane P3-DISPLAYLIST).

For every parity fixture: the engine under test compiles the document to
convergence, writing a display list (`FLASHTEX_DISPLAY_LIST`) on each pass;
the display list of the converged pass must place every glyph and rule
exactly where **pdflatex's** PDF (the pinned oracle pdfTeX, cached by
tools/parity/tiers.py) places it.

The reference positions are computed here from the qpdf-normalised content
streams of the oracle PDF, independently of the engine: a small PDF content
interpreter in exact rational arithmetic (`fractions.Fraction`) that follows
the text matrix through `Td`/`TD`/`Tm`/`T*`, each glyph's `/Widths` advance
and every `TJ` adjustment, and the CTM through `cm`/`q`/`Q`. Positions are
then rounded once to scaled points exactly as the spec says
(docs/protocol/display-list-v3.md §4.2): x = round(X · 65781.76),
y = round((H − Y) · 65781.76), halves away from zero, H the page's MediaBox
height (a form's BBox height). Rules are recognised as the spec defines
(§4.4). The comparison is exact (0 sp), in stream order, per page and per
form XObject.

    python3 tools/displaylist/check_positions.py --engine target/release/flashtex-initex \
        --formats /path/to/fmtdir [--only hw1] [-j 6] [--json out.json]

Oracle tooling only: pdflatex (MacTeX) is never in the product path.
"""

import argparse
import concurrent.futures
import json
import os
import re
import subprocess
import sys
import time
from fractions import Fraction as Fr

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(REPO, "tools", "parity"))
sys.path.insert(0, os.path.join(REPO, "tools", "visual-oracle"))
sys.path.insert(0, os.path.join(REPO, "tools", "real-world-corpus"))

import parity  # noqa: E402  (fixture discovery, tree hashes)
import pdftext  # noqa: E402
import tiers  # noqa: E402  (the cached oracle, running an engine to convergence)

SP_PER_BP = Fr(6578176, 100)


def rnd(x):
    """Round a Fraction to the nearest integer, halves away from zero."""
    n, d = x.numerator, x.denominator
    q, r = divmod(abs(n), d)
    if 2 * r >= d:
        q += 1
    return q if n >= 0 else -q


def fr(v):
    """A PDF number pdftext read as int/float, back to its exact decimal."""
    if isinstance(v, int):
        return Fr(v)
    return Fr(repr(v))


# ----------------------------------------------------------------------------
# content streams

TOKEN = re.compile(rb"""
    (?P<ws>[\x00\t\n\x0c\r ]+)
  | (?P<comment>%[^\r\n]*)
  | (?P<dict><<|>>)
  | (?P<arr>[\[\]])
  | (?P<name>/[^\x00\t\n\x0c\r ()<>\[\]{}/%]*)
  | (?P<num>[+-]?(?:\d+\.?\d*|\.\d+))(?=[\x00\t\n\x0c\r ()<>\[\]{}/%]|$)
  | (?P<op>[^\x00\t\n\x0c\r ()<>\[\]{}/%]+)
""", re.X)


def lit_string(s, i):
    """A literal string at s[i] == '('; returns (bytes, end)."""
    i += 1
    depth, out = 1, bytearray()
    esc = {ord("n"): 10, ord("r"): 13, ord("t"): 9, ord("b"): 8, ord("f"): 12}
    while i < len(s):
        c = s[i]
        i += 1
        if c == 0x5C:  # backslash
            e = s[i]
            i += 1
            if e in esc:
                out.append(esc[e])
            elif 0x30 <= e <= 0x37:
                v = e - 0x30
                for _ in range(2):
                    if i < len(s) and 0x30 <= s[i] <= 0x37:
                        v = v * 8 + s[i] - 0x30
                        i += 1
                    else:
                        break
                out.append(v & 0xFF)
            elif e == 13:
                if i < len(s) and s[i] == 10:
                    i += 1
            elif e == 10:
                pass
            else:
                out.append(e)
        elif c == 0x28:
            depth += 1
            out.append(c)
        elif c == 0x29:
            depth -= 1
            if depth == 0:
                return bytes(out), i
            out.append(c)
        elif c == 13:
            if i < len(s) and s[i] == 10:
                i += 1
            out.append(10)
        else:
            out.append(c)
    return bytes(out), i


def tokens(s):
    """(kind, value) tokens: num (Fraction), name, str (bytes), '[', ']', dict, op."""
    i = 0
    n = len(s)
    while i < n:
        c = s[i]
        if c == 0x28:
            b, i = lit_string(s, i)
            yield "str", b
            continue
        if c == 0x3C and s[i:i + 2] != b"<<":
            j = s.index(b">", i)
            h = re.sub(rb"[^0-9A-Fa-f]", b"", s[i + 1:j])
            if len(h) % 2:
                h += b"0"
            yield "str", bytes.fromhex(h.decode())
            i = j + 1
            continue
        m = TOKEN.match(s, i)
        if not m:
            i += 1
            continue
        i = m.end()
        k = m.lastgroup
        v = m.group(k)
        if k in ("ws", "comment"):
            continue
        if k == "num":
            yield "num", Fr(v.decode())
        elif k == "name":
            yield "name", v[1:].decode("latin-1")
        elif k == "arr":
            yield v.decode(), None
        elif k == "dict":
            yield "dict", v
        else:
            if v == b"ID":  # inline image data: skip to EI
                j = s.find(b"EI", i)
                while j > 0 and not (s[j - 1:j] in (b" ", b"\n", b"\r") and s[j + 2:j + 3] in (b" ", b"\n", b"\r", b"")):
                    j = s.find(b"EI", j + 2)
                i = j + 2 if j > 0 else n
                continue
            yield "op", v.decode("latin-1")


IDENT = (Fr(1), Fr(0), Fr(0), Fr(1), Fr(0), Fr(0))


def mul(m, n):
    """m then n (row vectors), 2x3 affine."""
    a, b, c, d, e, f = m
    a2, b2, c2, d2, e2, f2 = n
    return (a * a2 + b * c2, a * b2 + b * d2, c * a2 + d * c2, c * b2 + d * d2,
            e * a2 + f * c2 + e2, e * b2 + f * d2 + f2)


def apply(m, x, y):
    a, b, c, d, e, f = m
    return x * a + y * c + e, x * b + y * d + f


def interpret(content, height, widths):
    """Glyphs [(font, code, x, y)], rules [(kind, x, y, w, h)] and XObject
    placements [(name, matrix)] of one content stream; `widths(fontname,
    code)` is the /Widths entry (a Fraction) or None."""
    glyphs, rules, xobjects = [], [], []
    gs = {"ctm": IDENT, "lw": Fr(1), "cap": 0, "dash": [], "tc": Fr(0), "tw": Fr(0), "tz": Fr(100),
          "tl": Fr(0), "font": None, "fs": Fr(0), "ts": Fr(0)}
    stack = []
    tm = tlm = IDENT
    path, only_re = [], None
    clip = False
    ops = []
    arrays = []
    depth = 0

    def ysp(y):
        return rnd((height - y) * SP_PER_BP)

    def xsp(x):
        return rnd(x * SP_PER_BP)

    def show(b):
        nonlocal tm
        font = gs["font"]
        for code in b:
            trm = mul(tm, gs["ctm"])
            x, y = apply(trm, Fr(0), gs["ts"])
            glyphs.append((font, code, xsp(x), ysp(y)))
            w = widths(font, code)
            tx = (w * gs["fs"] / 1000 if w is not None else Fr(0)) + gs["tc"]
            if code == 32:
                tx += gs["tw"]
            tx = tx * gs["tz"] / 100
            tm = mul((Fr(1), Fr(0), Fr(0), Fr(1), tx, Fr(0)), tm)

    def edges(x0, y0, x1, y1, e, f):
        l, r = min(x0, x1), max(x0, x1)
        b, t = min(y0, y1), max(y0, y1)
        left, right = xsp(l + e), xsp(r + e)
        top, bottom = ysp(t + f), ysp(b + f)
        return left, top, right - left, bottom - top

    def paint(bits_fill, bits_stroke):
        nonlocal path, only_re, clip
        ctm = gs["ctm"]
        if path and not clip and ctm[:4] == IDENT[:4]:
            e, f = ctm[4], ctm[5]
            if bits_fill and not bits_stroke and only_re is not None:
                x, y, w, h = only_re
                rules.append((0,) + edges(x, y, x + w, y + h, e, f))
            elif bits_stroke and not bits_fill and gs["cap"] == 0 and not gs["dash"] and len(path) == 2 \
                    and path[0][0] == "m" and path[1][0] == "l":
                (_, x0, y0), (_, x1, y1) = path
                hw = gs["lw"] / 2
                if y0 == y1 and x0 != x1:
                    rules.append((1,) + edges(x0, y0 - hw, x1, y0 + hw, e, f))
                elif x0 == x1 and y0 != y1:
                    rules.append((2,) + edges(x0 - hw, y0, x0 + hw, y1, e, f))
        path, only_re, clip = [], None, False

    for kind, v in tokens(content):
        if kind == "dict":
            depth += 1 if v == b"<<" else -1
            if depth == 0:
                (arrays[-1] if arrays else ops).append(("dict", None))
            continue
        if depth > 0:
            continue
        if kind == "[":
            arrays.append([])
            continue
        if kind == "]":
            if arrays:
                a = arrays.pop()
                (arrays[-1] if arrays else ops).append(("arr", a))
            continue
        if kind != "op":
            (arrays[-1] if arrays else ops).append((kind, v))
            continue
        if arrays:
            arrays[-1].append(("op", v))
            continue
        op = v
        nums = [x for k, x in ops if k == "num"]
        if op == "q":
            stack.append(dict(gs))
        elif op == "Q":
            if stack:
                gs = stack.pop()
        elif op == "cm" and len(nums) >= 6:
            gs["ctm"] = mul(tuple(nums[-6:]), gs["ctm"])
        elif op == "w" and nums:
            gs["lw"] = nums[-1]
        elif op == "J" and nums:
            gs["cap"] = int(nums[-1])
        elif op == "d" and ops and ops[0][0] == "arr":
            gs["dash"] = [x for k, x in ops[0][1] if k == "num"]
        elif op in ("m", "l") and len(nums) >= 2:
            path.append((op, nums[-2], nums[-1]))
            only_re = None
            path_has_other = True
        elif op in ("c", "v", "y", "h"):
            path.append((op, None, None))
            only_re = None
        elif op == "re" and len(nums) >= 4:
            only_re = tuple(nums[-4:]) if not path else None
            path.append(("re", None, None))
        elif op in ("W", "W*"):
            clip = True
        elif op in ("S", "s", "f", "F", "f*", "B", "B*", "b", "b*", "n"):
            if op in ("s", "b", "b*"):
                only_re = None
                path.append(("h", None, None))
            fill = op in ("f", "F", "f*", "B", "B*", "b", "b*")
            stroke = op in ("S", "s", "B", "B*", "b", "b*")
            if op == "n":
                path, only_re, clip = [], None, False
            else:
                paint(fill, stroke)
        elif op == "BT":
            tm = tlm = IDENT
        elif op == "Tc" and nums:
            gs["tc"] = nums[-1]
        elif op == "Tw" and nums:
            gs["tw"] = nums[-1]
        elif op == "Tz" and nums:
            gs["tz"] = nums[-1]
        elif op == "TL" and nums:
            gs["tl"] = nums[-1]
        elif op == "Ts" and nums:
            gs["ts"] = nums[-1]
        elif op == "Tf" and ops and ops[0][0] == "name" and nums:
            gs["font"], gs["fs"] = ops[0][1], nums[-1]
        elif op in ("Td", "TD") and len(nums) >= 2:
            tx, ty = nums[-2], nums[-1]
            if op == "TD":
                gs["tl"] = -ty
            tlm = mul((Fr(1), Fr(0), Fr(0), Fr(1), tx, ty), tlm)
            tm = tlm
        elif op == "Tm" and len(nums) >= 6:
            tlm = tm = tuple(nums[-6:])
        elif op in ("T*", "'", '"'):
            tlm = mul((Fr(1), Fr(0), Fr(0), Fr(1), Fr(0), -gs["tl"]), tlm)
            tm = tlm
            if op == '"' and len(nums) >= 2:
                gs["tw"], gs["tc"] = nums[-2], nums[-1]
            if op != "T*" and ops and ops[-1][0] == "str":
                show(ops[-1][1])
        elif op == "Tj" and ops and ops[-1][0] == "str":
            show(ops[-1][1])
        elif op == "TJ" and ops and ops[-1][0] == "arr":
            for k, x in ops[-1][1]:
                if k == "str":
                    show(x)
                elif k == "num":
                    tx = -x * gs["fs"] / 1000 * gs["tz"] / 100
                    tm = mul((Fr(1), Fr(0), Fr(0), Fr(1), tx, Fr(0)), tm)
        elif op == "Do" and ops and ops[-1][0] == "name":
            xobjects.append((ops[-1][1], gs["ctm"]))
        ops = []
    return glyphs, rules, xobjects


# ----------------------------------------------------------------------------
# the oracle's PDF


def font_number(name):
    m = re.match(r"^F(\d+)", name or "")
    return int(m.group(1)) if m else None


def reference(pdf, work):
    """Per page: {glyphs, rules, xobjects, forms: {n: (glyphs, rules)}}."""
    os.makedirs(work, exist_ok=True)
    q = os.path.join(work, "oracle.qdf.pdf")
    tiers.normalise_pdf(pdf, q)
    doc = pdftext.PdfDocument.load(q)
    r = doc.resolve
    pages = []
    form_cache = {}

    def width_fn(resources):
        fonts = r((r(resources) or {}).get("Font")) or {}
        cache = {}

        def widths(name, code):
            if name not in cache:
                fd = r(fonts.get(name)) or {}
                first = fd.get("FirstChar", 0)
                ws = r(fd.get("Widths")) or []
                # in thousandths of text space: a Type 3 font's widths are
                # in its glyph space, which its /FontMatrix maps
                scale = Fr(1)
                if fd.get("Subtype") == "Type3":
                    scale = fr(r(r(fd.get("FontMatrix"))[0])) * 1000
                cache[name] = (first, [fr(w) * scale for w in ws])
            first, ws = cache[name]
            i = code - first
            return ws[i] if 0 <= i < len(ws) else None
        return widths

    def forms_of(resources, out):
        xo = r((r(resources) or {}).get("XObject")) or {}
        for name, ref in xo.items():
            m = re.match(r"^Fm(\d+)", name)
            if not m:
                continue
            n = int(m.group(1))
            if n in out:
                continue
            obj = r(ref)
            if not isinstance(obj, dict) or obj.get("Subtype") != "Form":
                continue
            bbox = [fr(v) for v in r(obj.get("BBox"))]
            content = doc.stream_of(ref)
            g, ru, x = interpret(content, bbox[3] - bbox[1], width_fn(obj.get("Resources")))
            out[n] = {"glyphs": g, "rules": ru, "xobjects": x}
            forms_of(obj.get("Resources"), out)

    for p in doc.pages():
        mb = [fr(v) for v in r(p.get("MediaBox"))]
        c = r(p.get("Contents"))
        refs = c if isinstance(c, list) else [p.get("Contents")]
        content = b"\n".join(doc.stream_of(x) for x in refs if isinstance(x, pdftext.Ref))
        g, ru, x = interpret(content, mb[3] - mb[1], width_fn(p.get("Resources")))
        forms_of(p.get("Resources"), form_cache)
        pages.append({"glyphs": g, "rules": ru, "xobjects": x})
    os.remove(q)
    return pages, form_cache


# ----------------------------------------------------------------------------
# the display list


def display_list(dl3, dump):
    """Pages and forms from `dl3-dump`: glyphs and rules in stream order."""
    out = subprocess.run([dump, dl3], capture_output=True, check=False)
    if out.returncode != 0:
        raise RuntimeError("dl3-dump: " + out.stderr.decode("utf-8", "replace")[:300])
    pages, forms, fonts = [], {}, {}
    for line in out.stdout.decode("utf-8").splitlines():
        j = json.loads(line)
        if j["kind"] == "font":
            fonts[j["id"]] = j["info"]["pdf_name"]
        if j["kind"] not in ("page", "form"):
            continue
        g = [(it[1], it[2], it[3], it[4]) for it in j["items"] if it[0] == "g"]
        ru = [tuple(it[1:6]) for it in j["items"] if it[0] == "r"]
        rec = {"glyphs": g, "rules": ru, "unsupported": j["unsupported"], "flags": j["flags"], "index": j["index"],
               "items": len(j["items"]), "hash": j["hash"]}
        if j["kind"] == "page":
            pages.append(rec)
        else:
            forms[j["index"]] = rec
    return pages, forms, fonts


def compare(ref, dl, what):
    """None if equal, else the first difference."""
    rg = [(font_number(f), c, x, y) for f, c, x, y in ref["glyphs"]]
    dg = [tuple(g) for g in dl["glyphs"]]
    if rg != dg:
        for i, (a, b) in enumerate(zip(rg, dg)):
            if a != b:
                return {"where": what, "glyph": i, "oracle": a, "display_list": b}
        return {"where": what, "glyphs": [len(rg), len(dg)]}
    rr = [tuple(x) for x in ref["rules"]]
    dr = [tuple(x) for x in dl["rules"]]
    if rr != dr:
        for i, (a, b) in enumerate(zip(rr, dr)):
            if a != b:
                return {"where": what, "rule": i, "oracle": a, "display_list": b}
        return {"where": what, "rules": [len(rr), len(dr)]}
    return None


def check(doc, cfg):
    t0 = time.time()
    name = doc["id"]
    work = os.path.join(cfg["work"], name.replace("/", "__"))
    rec = {"id": name, "ok": False}
    meta, _, ref_pdf = tiers.oracle(doc, cfg["oracle"], cfg["cache"], False, parity.tree_hash(doc["dir"]))
    if not ref_pdf:
        rec["why"] = "oracle: " + str(meta.get("why"))
        return rec
    dl3 = os.path.join(work, "display.dl3")
    env = dict(cfg["engine_env"], FLASHTEX_DISPLAY_LIST=dl3)
    cmeta, _, cand_pdf = tiers.run_tex(doc, cfg["engine"], os.path.join(work, "src"), trace=False, extra_env=env)
    if not cand_pdf or not os.path.isfile(dl3):
        rec["why"] = "engine: " + str(cmeta.get("why"))
        return rec
    try:
        rpages, rforms = reference(ref_pdf, work)
        dpages, dforms, _ = display_list(dl3, cfg["dump"])
    except Exception as e:  # noqa: BLE001  (reported per document)
        rec["why"] = f"{type(e).__name__}: {e}"[:300]
        return rec
    rec["pages"] = [len(rpages), len(dpages)]
    rec["glyphs"] = sum(len(p["glyphs"]) for p in rpages)
    rec["rules"] = sum(len(p["rules"]) for p in rpages)
    rec["forms"] = [len(rforms), len(dforms)]
    rec["unsupported"] = sorted({u for p in dpages for u in p["unsupported"]})
    rec["bytes"] = os.path.getsize(dl3)
    if len(rpages) != len(dpages):
        rec["why"] = f"page count: oracle {len(rpages)}, display list {len(dpages)}"
        return rec
    for i, (a, b) in enumerate(zip(rpages, dpages)):
        d = compare(a, b, f"page {i + 1}")
        if d:
            rec["why"] = "position"
            rec["first"] = d
            return rec
    for n, f in rforms.items():
        if n not in dforms:
            rec["why"] = f"form {n} missing from the display list"
            return rec
        d = compare(f, dforms[n], f"form {n}")
        if d:
            rec["why"] = "position"
            rec["first"] = d
            return rec
    rec["ok"] = True
    rec["seconds"] = round(time.time() - t0, 2)
    return rec


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--engine", required=True, help="the engine binary (flashtex-initex)")
    ap.add_argument("--formats", required=True, help="directory with the engine's pdflatex.fmt (FLASHTEX_FORMATS)")
    ap.add_argument("--pool", default=os.path.join(REPO, "crates", "flashtex-engine", "pdftex.pool"))
    ap.add_argument("--dump", default=os.path.join(REPO, "target", "release", "dl3-dump"))
    ap.add_argument("--oracle", default=tiers.DEFAULT_ORACLE)
    ap.add_argument("--cache", default=os.path.expanduser("~/.cache/flashtex-parity"))
    ap.add_argument("--work", default=os.path.join(REPO, "target", "dl3-positions"))
    ap.add_argument("--only", action="append", default=[])
    ap.add_argument("--root", action="append", default=[],
                    help="a directory of fixture documents (default: the parity fixtures)")
    ap.add_argument("-j", type=int, default=4)
    ap.add_argument("--json", default=None, help="write the per-document results here")
    a = ap.parse_args()
    cfg = {"engine": os.path.abspath(a.engine), "oracle": a.oracle, "cache": a.cache, "work": a.work,
           "dump": os.path.abspath(a.dump),
           "engine_env": {"FLASHTEX_FORMATS": os.path.abspath(a.formats), "FLASHTEX_POOL": os.path.abspath(a.pool)}}
    docs = parity.fixture_documents(roots=tuple(a.root) or parity.FIXTURE_ROOTS, only=tuple(a.only))
    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=a.j) as ex:
        futs = {ex.submit(check, d, cfg): d for d in docs}
        for f in concurrent.futures.as_completed(futs):
            d = futs[f]
            try:
                r = f.result()
            except Exception as e:  # noqa: BLE001
                r = {"id": d["id"], "ok": False, "why": f"{type(e).__name__}: {e}"[:300]}
            results.append(r)
            mark = "ok  " if r["ok"] else "FAIL"
            extra = "" if r["ok"] else f"  {r.get('why')} {json.dumps(r.get('first')) if r.get('first') else ''}"
            print(f"{mark} {r['id']}  pages={r.get('pages')} glyphs={r.get('glyphs')} rules={r.get('rules')}"
                  f" forms={r.get('forms')}{extra}", flush=True)
    results.sort(key=lambda r: r["id"])
    n_ok = sum(r["ok"] for r in results)
    summary = {"checked": len(results), "ok": n_ok,
               "glyphs": sum(r.get("glyphs") or 0 for r in results if r["ok"]),
               "rules": sum(r.get("rules") or 0 for r in results if r["ok"]),
               "pages": sum((r.get("pages") or [0])[0] for r in results if r["ok"])}
    print(f"positions: {n_ok}/{len(results)} documents exact (0 sp); "
          f"{summary['pages']} pages, {summary['glyphs']} glyphs, {summary['rules']} rules")
    if a.json:
        with open(a.json, "w", encoding="utf-8") as f:
            json.dump({"summary": summary, "results": results}, f, indent=1)
    return 0 if n_ok == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
