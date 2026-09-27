#!/usr/bin/env python3
r"""Generate src/cm_tfm.rs from the Computer Modern TFM files shipped with a TeX
distribution.

This is a development-time data extraction tool, not part of the product path:
the crate never reads TFM files or runs TeX at runtime. It embeds the fontdimen
parameters and per-character width/height/depth/italic-correction fixwords of
cmr/cmmi/cmsy at 10/7/5 pt (LaTeX 10pt), cmr/cmmi at 12/8/6 pt with cmsy8/6
(LaTeX 12pt), cmex10, cmr9/cmmi9/cmsy9 and cmr17 (the remaining designs the
kernel `.fd` files load at `\DeclareMathSizes` sizes) and cmex7/8/9
(amsmath/amsfonts), plus each character's next-larger
successor, extensible recipe, and the kern against the font's skew character
(plain.tex: \skewchar\tenmi='177, \skewchar\tensy='60), and each font's
ligature/kern program and kern table, which TeX's `make_ord` (tex.web §752)
reads between adjacent math characters of one family.

Usage: python3 tools/gen_cm_tfm.py [kpsewhich-path] > src/cm_tfm.rs
       python3 tools/gen_cm_tfm.py [kpsewhich-path] --ams > src/ams_tfm.rs
       python3 tools/gen_cm_tfm.py [kpsewhich-path] [--ams] --check
(`--check` compares with the committed src/cm_tfm.rs or src/ams_tfm.rs.)

TFM format reference: TeX: The Program, part 30 ("Font metric data"), and
tftopl.web. Values are emitted as raw fixwords (signed 32-bit, value/2^20 of
the font size) so that `TfmFont::scale` can reproduce TeX's own integer
scaling (tex.web §571-572) and match TeX's box dimensions to the scaled point.
"""
import os
import struct
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

FONTS = ["cmr10", "cmr7", "cmr5", "cmmi10", "cmmi7", "cmmi5",
         "cmr12", "cmr8", "cmr6", "cmmi12", "cmmi8", "cmmi6", "cmsy8", "cmsy6",
         "cmsy10", "cmsy7", "cmsy5", "cmex10",
         # Designs LaTeX loads at the other `\DeclareMathSizes` sizes
         # (`\small` 9pt, `\LARGE`..`\Huge` cmr17) and amsmath/amsfonts'
         # sized OMX/cmex (`<-7.5>cmex7 <7.5-8.5>cmex8 <8.5-9.5>cmex9`).
         "cmr9", "cmr17", "cmmi9", "cmsy9", "cmex7", "cmex8", "cmex9"]
# umsa.fd / umsb.fd: `<-6>msam5 <6-8>msam7 <8->msam10` (and msbm alike).
AMS_FONTS = ["msam5", "msam7", "msam10", "msbm5", "msbm7", "msbm10"]
SKEW = {"cmmi10": 0o177, "cmmi7": 0o177, "cmmi5": 0o177,
        "cmmi12": 0o177, "cmmi8": 0o177, "cmmi6": 0o177, "cmsy8": 0o60, "cmsy6": 0o60,
        "cmsy10": 0o60, "cmsy7": 0o60, "cmsy5": 0o60,
        "cmmi9": 0o177, "cmsy9": 0o60}


def fix(w):
    """Raw fixword as a signed 32-bit integer (value = w / 2^20)."""
    if w >= 1 << 31:
        w -= 1 << 32
    return w


def parse(path):
    b = open(path, "rb").read()
    lf, lh, bc, ec, nw, nh, nd, ni, nl, nk, ne, np_ = struct.unpack(">12H", b[:24])
    words = [struct.unpack(">I", b[24 + 4 * i:28 + 4 * i])[0] for i in range(lf - 6)]
    o = 0
    header = words[o:o + lh]; o += lh
    ci = words[o:o + ec - bc + 1]; o += ec - bc + 1
    W = [fix(x) for x in words[o:o + nw]]; o += nw
    H = [fix(x) for x in words[o:o + nh]]; o += nh
    D = [fix(x) for x in words[o:o + nd]]; o += nd
    I = [fix(x) for x in words[o:o + ni]]; o += ni
    LK = words[o:o + nl]; o += nl
    K = [fix(x) for x in words[o:o + nk]]; o += nk
    E = words[o:o + ne]; o += ne
    P = [fix(x) for x in words[o:o + np_]]
    chars = {}
    for c in range(bc, ec + 1):
        w = ci[c - bc]
        wi = w >> 24; hi = (w >> 20) & 15; di = (w >> 16) & 15
        ii = (w >> 10) & 63; tag = (w >> 8) & 3; rem = w & 255
        if wi == 0:
            continue
        e = {"w": W[wi], "h": H[hi], "d": D[di], "i": I[ii], "next": None, "ext": None, "kern": {},
             "lig_start": None}
        if tag == 2:
            e["next"] = rem
        if tag == 3:
            x = E[rem]
            e["ext"] = [(x >> 24) & 255, (x >> 16) & 255, (x >> 8) & 255, x & 255]
        if tag == 1:
            j = rem
            first = LK[j]
            if (first >> 24) > 128:
                j = 256 * ((first >> 8) & 255) + (first & 255)
            # The program's first instruction, after a far-away restart.
            e["lig_start"] = j
            while True:
                ins = LK[j]; skip = ins >> 24; nxt = (ins >> 16) & 255
                op = (ins >> 8) & 255; r = ins & 255
                if op >= 128:
                    e["kern"].setdefault(nxt, K[256 * (op - 128) + r])
                if skip >= 128:
                    break
                j += skip + 1
        chars[c] = e
    return {"design": fix(header[1]), "params": P, "chars": chars, "lig_kern": LK, "kerns": K}


def write_or_check(path, text, check):
    """Write `text` to `path`, or with `--check` compare instead.

    `--check` exit codes (the same for every generator; see
    scripts/check-generated.py): 0 the committed file is what this generator
    produces now, 1 it is stale (a diff excerpt is printed), 2 it cannot be
    checked here (a tool or input is missing; the message says which).
    """
    data = text.encode("utf-8")
    rel = os.path.relpath(path)
    if not check:
        with open(path, "wb") as fh:
            fh.write(data)
        return 0
    try:
        with open(path, "rb") as fh:
            old = fh.read()
    except FileNotFoundError:
        print("STALE: %s does not exist" % rel)
        return 1
    if old == data:
        print("up to date: %s" % rel)
        return 0
    import difflib
    diff = list(difflib.unified_diff(old.decode("utf-8", "replace").splitlines(),
                                     text.splitlines(), "committed", "regenerated",
                                     lineterm="", n=0))
    for line in diff[:40]:
        print(line)
    if len(diff) > 40:
        print("... %d more diff lines" % (len(diff) - 40))
    print("STALE: %s differs from what %s generates now" % (rel, os.path.relpath(sys.argv[0])))
    return 1


def require_tools(*tools):
    """Exit 2 (cannot check here) when a TeX tool is not on PATH."""
    import shutil
    missing = [t for t in tools if not shutil.which(t)]
    if missing:
        print("CANNOT CHECK: %s not found on PATH (needs TeX Live 2026)" % ", ".join(missing))
        sys.exit(2)


def rustfmt(text):
    """`text` as rustfmt (edition 2024, like the crate) prints it; exit 2 without rustfmt."""
    try:
        return subprocess.run(["rustfmt", "--edition", "2024", "--emit", "stdout"], input=text,
                              capture_output=True, text=True, check=True).stdout
    except (OSError, subprocess.CalledProcessError) as e:
        print("CANNOT CHECK: rustfmt failed (%s)" % e)
        sys.exit(2)


def main():
    positional = [a for a in sys.argv[1:] if not a.startswith("--")]
    kpsewhich = positional[0] if positional else "kpsewhich"
    check = "--check" in sys.argv[1:]
    if check:
        require_tools(kpsewhich)
    # `--ams`: the AMS symbol fonts `amsfonts.sty` declares as the `AMSa`/`AMSb`
    # symbol fonts (`umsa.fd`/`umsb.fd`: msam/msbm 5, 7 and 10), for
    # `src/ams_tfm.rs`. Without it, the Computer Modern set for `src/cm_tfm.rs`.
    ams = "--ams" in sys.argv[1:]
    fonts = AMS_FONTS if ams else FONTS
    package = "`amsfonts`" if ams else "`cm`"
    title = "AMS symbol font" if ams else "Computer Modern"
    out = []
    out.append(f"//! {title} TFM metrics, generated by `tools/gen_cm_tfm.py{' --ams' if ams else ''}`.")
    out.append("//!")
    out.append("//! Do not edit by hand. Values are raw TFM fixwords (units of 2^-20 of the")
    out.append("//! font size); scale them with `TfmFont::scale`, which reproduces TeX's")
    out.append(f"//! integer fixword arithmetic. Provenance: the `.tfm` files of the {package}")
    out.append("//! package as resolved by `kpsewhich` at generation time; see the generator.")
    out.append("#![allow(clippy::unreadable_literal)]")
    out.append("")
    out.append("use super::tfm::{c, TfmFont};")
    out.append("")
    out.append("const N: u8 = u8::MAX;")
    out.append("")
    for f in fonts:
        path = subprocess.check_output([kpsewhich, f + ".tfm"]).decode().strip()
        t = parse(path)
        skew = SKEW.get(f)
        out.append(f"/// `{f}.tfm` (design size {t['design'] / 2**20:g} pt), from `{path}`.")
        out.append(f"pub static {f.upper()}: TfmFont = TfmFont {{")
        out.append(f"    name: \"{f}\",")
        out.append(f"    design_size: {t['design'] / 2**20},")
        out.append(f"    skew_char: {skew if skew is not None else 'u8::MAX'},")
        params = ", ".join(str(p) for p in t["params"])
        out.append(f"    params: &[{params}],")
        # `lig_kern_starts`: `(code << 16) | index` of each character's first
        # lig/kern instruction, sorted by code. `lig_kern` is the raw
        # instruction words (skip, next char, op, remainder: tex.web §545) and
        # `kerns` the kern table, so `TfmFont::lig_kern` walks the program as
        # TeX does.
        starts = ", ".join(str((c << 16) | t["chars"][c]["lig_start"])
                           for c in sorted(t["chars"]) if t["chars"][c]["lig_start"] is not None)
        out.append(f"    lig_kern_starts: &[{starts}],")
        out.append(f"    lig_kern: &[{', '.join(str(x) for x in t['lig_kern'])}],")
        out.append(f"    kerns: &[{', '.join(str(x) for x in t['kerns'])}],")
        out.append("    chars: &[")
        for c in sorted(t["chars"]):
            e = t["chars"][c]
            nxt = e["next"] if e["next"] is not None else "N"
            ext = ", ".join(map(str, e["ext"])) if e["ext"] else "N, N, N, N"
            sk = e["kern"].get(skew, 0) if skew is not None else 0
            out.append(
                f"        c({c}, {e['w']}, {e['h']}, {e['d']}, {e['i']}, "
                f"{sk}, {nxt}, [{ext}]),")
        out.append("    ],")
        out.append("};")
        out.append("")
    if check:
        # The committed cm_tfm.rs went through `cargo fmt` (ams_tfm.rs did
        # not), so both sides are compared as rustfmt prints them: a data
        # change is stale, a layout-only difference is not.
        dest = os.path.join(HERE, "..", "src", "ams_tfm.rs" if ams else "cm_tfm.rs")
        with open(dest, encoding="utf-8") as fh:
            committed = rustfmt(fh.read())
        regenerated = rustfmt("\n".join(out))
        tmp = dest + ".check-committed"
        with open(tmp, "w", encoding="utf-8") as fh:
            fh.write(committed)
        try:
            rc = write_or_check(tmp, regenerated, True)
        finally:
            os.remove(tmp)
        print("(compared after rustfmt: %s)" % os.path.relpath(dest))
        return rc
    sys.stdout.write("\n".join(out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
