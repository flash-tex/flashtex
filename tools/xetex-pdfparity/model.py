"""Everything the parity harness compares, extracted from one PDF.

    m = extract("a.pdf")
    m.pages[i].box / .content (content.Page) / .annots
    m.dests, m.outline, m.info, m.catalog, m.fonts
"""
from content import Interp, _r
from fonts import FontCache
from pdfdoc import Doc, Stream, name, text

INFO_KEYS = ("/Title", "/Author", "/Subject", "/Keywords", "/Creator")
BOXES = ("/MediaBox", "/CropBox", "/BleedBox", "/TrimBox", "/ArtBox")


class PageModel:
    def __init__(self, index, box, content, annots):
        self.index, self.box, self.content, self.annots = index, box, content, annots


class Model:
    pass


def _nums(doc, a, nd=4):
    a = doc.get(a)
    if not isinstance(a, list):
        return None
    return [_r(doc.get(x), nd) if isinstance(doc.get(x), (int, float)) else doc.get(x) for x in a]


def dest_desc(doc, d, names=None):
    """A destination: {"page", "kind", "args"} (explicit) or {"name"} (named,
    plus what it resolves to under "to")."""
    d = doc.get(d)
    if isinstance(d, dict) and "/D" in d:
        d = doc.get(d["/D"])
    if isinstance(d, list) and d:
        page = d[0]
        pi = doc.page_of(page)
        out = {"page": (pi + 1) if pi is not None else (doc.get(page) if isinstance(doc.get(page), int) else "?"),
               "kind": name(doc.get(d[1])) if len(d) > 1 else None,
               "args": [None if doc.get(x) is None else _r(doc.get(x), 4) for x in d[2:]]}
        return out
    if isinstance(d, str) and (d.startswith("u:") or d.startswith("b:") or name(d)):
        nm = text(d) if not name(d) else name(d)
        out = {"name": nm}
        if names is not None and nm in names:
            out["to"] = dest_desc(doc, names[nm])
        return out
    return {"other": doc.canon(d)}


def action_desc(doc, a, names):
    a = doc.get(a)
    if not isinstance(a, dict):
        return None
    s = name(doc.get(a.get("/S")))
    if s == "URI":
        return {"URI": text(doc.get(a.get("/URI")))}
    if s == "GoTo":
        return {"GoTo": dest_desc(doc, a.get("/D"), names)}
    if s == "GoToR":
        return {"GoToR": {"file": doc.canon(a.get("/F")), "dest": doc.canon(a.get("/D"))}}
    return {s or "?": doc.canon({k: v for k, v in a.items() if k != "/Next"})}


def annot_desc(doc, a, names):
    a = doc.get(a)
    if not isinstance(a, dict):
        return None
    sub = name(doc.get(a.get("/Subtype")))
    out = {"subtype": sub, "rect": _nums(doc, a.get("/Rect"))}
    if out["rect"]:
        x0, y0, x1, y1 = out["rect"][:4]
        out["rect"] = [min(x0, x1), min(y0, y1), max(x0, x1), max(y0, y1)]
    if sub == "Link":
        if "/A" in a:
            out["action"] = action_desc(doc, a["/A"], names)
        elif "/Dest" in a:
            out["action"] = {"Dest": dest_desc(doc, a["/Dest"], names)}
        else:
            out["action"] = None
        out["border"] = doc.canon(a.get("/Border"))
        out["bs"] = doc.canon(a.get("/BS"))
        out["c"] = doc.canon(a.get("/C"))
        out["h"] = name(doc.get(a.get("/H")))
        out["flags"] = doc.get(a.get("/F"))
    else:
        out["dict"] = doc.canon(a, skip=("/P", "/Parent", "/Length", "/Rect", "/M"))
    return out


def outline_items(doc, names):
    root = doc.get(doc.catalog.get("/Outlines"))
    out = []
    if not isinstance(root, dict):
        return out
    seen = set()

    def walk(first, depth):
        ref = first
        while ref is not None and ref not in seen and len(out) < 100000:
            seen.add(ref)
            it = doc.get(ref)
            if not isinstance(it, dict):
                break
            count = doc.get(it.get("/Count"))
            target = None
            if "/Dest" in it:
                target = {"Dest": dest_desc(doc, it["/Dest"], names)}
            elif "/A" in it:
                target = action_desc(doc, it["/A"], names)
            out.append({"depth": depth, "title": text(doc.get(it.get("/Title"))),
                        "target": target,
                        "open": None if not count else count > 0,
                        "flags": doc.get(it.get("/F")), "color": _nums(doc, it.get("/C"))})
            if "/First" in it:
                walk(it["/First"], depth + 1)
            ref = it.get("/Next")

    walk(root.get("/First"), 0)
    return out


def named_dests(doc):
    names = {}
    cat = doc.catalog
    nd = doc.get(cat.get("/Names"))
    if isinstance(nd, dict) and "/Dests" in nd:
        names.update(doc.name_tree(nd["/Dests"]))
    old = doc.get(cat.get("/Dests"))
    if isinstance(old, dict):
        for k, v in old.items():
            names[name(k) or k] = v
    return names


def extract(path):
    doc = Doc.open(path)
    fonts = FontCache(doc)
    interp = Interp(doc, fonts)
    names = named_dests(doc)
    m = Model()
    m.path = path
    m.doc = doc
    m.pages = []
    for i, (ref, p) in enumerate(doc.pages):
        box = {}
        for k in BOXES:
            v = doc.inherited(p, k) if k in ("/MediaBox", "/CropBox") else doc.get(p.get(k))
            if v is not None:
                box[k[1:]] = _nums(doc, v)
        box.setdefault("CropBox", box.get("MediaBox"))
        box["Rotate"] = (doc.inherited(p, "/Rotate") or 0) % 360
        content = interp.run_page(i)
        annots = [annot_desc(doc, a, names) for a in (doc.get(p.get("/Annots")) or [])]
        m.pages.append(PageModel(i, box, content, [a for a in annots if a]))
    m.dests = {k: dest_desc(doc, v) for k, v in names.items()}
    m.outline = outline_items(doc, names)
    info = doc.info if isinstance(doc.info, dict) else {}
    m.info = {k[1:]: text(doc.get(info.get(k))) for k in INFO_KEYS if k in info}
    cat = doc.catalog
    oa = doc.get(cat.get("/OpenAction"))
    m.catalog = {
        "PageMode": name(doc.get(cat.get("/PageMode"))),
        "PageLayout": name(doc.get(cat.get("/PageLayout"))),
        "OpenAction": (action_desc(doc, oa, names) if isinstance(oa, dict)
                       else dest_desc(doc, oa, names) if oa is not None else None),
        "ViewerPreferences": doc.canon(cat.get("/ViewerPreferences")),
        "PageLabels": doc.canon(cat.get("/PageLabels")),
    }
    m.fonts = fonts
    return m


__all__ = ["extract", "Model", "PageModel", "Stream"]
