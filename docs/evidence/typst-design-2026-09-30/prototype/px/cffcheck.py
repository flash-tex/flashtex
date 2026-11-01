"""Compare the CFF private dicts / hinting of the PDF's embedded subsets with
the original font files."""
import io, sys, json
import pikepdf
from fontTools.cffLib import CFFFontSet
from fontTools.ttLib import TTFont

pdf = pikepdf.open(sys.argv[1])
dump = json.load(open(sys.argv[2]))
seen = set()
for page in pdf.pages:
    for name, f in page.Resources.Font.items():
        desc = f.DescendantFonts[0]
        fd = desc.FontDescriptor
        base = str(desc.BaseFont)
        if base in seen:
            continue
        seen.add(base)
        data = fd.FontFile3.read_bytes()
        cff = CFFFontSet()
        cff.decompile(io.BytesIO(data), None)
        top = cff[cff.fontNames[0]]
        priv = top.FDArray[0].Private if hasattr(top, "FDArray") else top.Private
        keys = {k: getattr(priv, k) for k in ("BlueValues", "OtherBlues", "StdHW", "StdVW", "StemSnapH", "StemSnapV", "BlueScale") if hasattr(priv, k)}
        cs = top.CharStrings
        # count hint operators in first 20 glyphs
        hints = 0
        for g in list(cs.keys())[:40]:
            c = cs[g]
            c.decompile()
            hints += sum(1 for t in c.program if isinstance(t, str) and t in ("hstem", "vstem", "hstemhm", "vstemhm", "hintmask", "cntrmask"))
        print(json.dumps({"pdf_font": base, "cid": hasattr(top, "ROS"), "glyphs": len(cs), "fontmatrix": list(getattr(top, "FontMatrix", [])), "private": {k: (list(v) if isinstance(v, list) else v) for k, v in keys.items()}, "hint_ops_first40": hints}))

for f in dump["fonts"]:
    tt = TTFont(f["path"], fontNumber=f["index"])
    if "CFF " not in tt:
        print(json.dumps({"orig": f["family"], "cff": False}))
        continue
    top = tt["CFF "].cff[0]
    priv = top.Private
    keys = {k: getattr(priv, k) for k in ("BlueValues", "OtherBlues", "StdHW", "StdVW", "StemSnapH", "StemSnapV", "BlueScale") if hasattr(priv, k)}
    cs = top.CharStrings
    hints = 0
    for g in list(cs.keys())[:40]:
        c = cs[g]
        c.decompile()
        hints += sum(1 for t in c.program if isinstance(t, str) and t in ("hstem", "vstem", "hstemhm", "vstemhm", "hintmask", "cntrmask"))
    print(json.dumps({"orig": f["family"], "path": f["path"], "fontmatrix": list(top.FontMatrix), "private": {k: (list(v) if isinstance(v, list) else v) for k, v in keys.items()}, "hint_ops_first40": hints}))
