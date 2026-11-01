"""Bidi test vectors from the system ICU (macOS libicucore) through ctypes.

usage: python3 -I icu_oracle.py icu-vectors.txt (on macOS; no build)
Each line: paraLevel(hex) TAB text as hex UTF-16 units TAB direction TAB
levels (ubidi_getLevels, hex bytes) TAB visual runs "dir:start:len,...".
"""
import ctypes
import random
import sys

icu = ctypes.CDLL("/usr/lib/libicucore.dylib")
U16 = ctypes.c_uint16
icu.ubidi_open.restype = ctypes.c_void_p
icu.ubidi_setPara.argtypes = [ctypes.c_void_p, ctypes.POINTER(U16), ctypes.c_int32,
                              ctypes.c_uint8, ctypes.c_void_p, ctypes.POINTER(ctypes.c_int)]
icu.ubidi_getDirection.argtypes = [ctypes.c_void_p]
icu.ubidi_getDirection.restype = ctypes.c_int
icu.ubidi_countRuns.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_int)]
icu.ubidi_countRuns.restype = ctypes.c_int32
icu.ubidi_getVisualRun.argtypes = [ctypes.c_void_p, ctypes.c_int32,
                                   ctypes.POINTER(ctypes.c_int32), ctypes.POINTER(ctypes.c_int32)]
icu.ubidi_getVisualRun.restype = ctypes.c_int
icu.ubidi_getLevels.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_int)]
icu.ubidi_getLevels.restype = ctypes.POINTER(ctypes.c_uint8)
icu.ubidi_close.argtypes = [ctypes.c_void_p]
icu.u_getVersion.argtypes = [ctypes.c_uint8 * 4]


def run(units, level):
    b = icu.ubidi_open()
    arr = (U16 * max(1, len(units)))(*units)
    err = ctypes.c_int(0)
    icu.ubidi_setPara(b, arr, len(units), level, None, ctypes.byref(err))
    assert err.value <= 0, err.value
    d = icu.ubidi_getDirection(b)
    err = ctypes.c_int(0)
    n = icu.ubidi_countRuns(b, ctypes.byref(err))
    runs = []
    for i in range(n):
        s, l = ctypes.c_int32(), ctypes.c_int32()
        rd = icu.ubidi_getVisualRun(b, i, ctypes.byref(s), ctypes.byref(l))
        runs.append(f"{rd}:{s.value}:{l.value}")
    err = ctypes.c_int(0)
    lv = icu.ubidi_getLevels(b, ctypes.byref(err)) if units else None
    levels = "".join(f"{lv[i]:02x}" for i in range(len(units))) if units else ""
    icu.ubidi_close(b)
    return d, levels, ",".join(runs)


def utf16(s):
    out = []
    for ch in s:
        c = ord(ch)
        if c > 0xFFFF:
            c -= 0x10000
            out += [0xD800 + (c >> 10), 0xDC00 + (c & 0x3FF)]
        else:
            out.append(c)
    return out


HAND = [
    "", "office", "שלום", "שלום עולם", "abc שלום def", "שלום abc", "123", "שלום123",
    "מספרים 123 ו", "הדגשה.", "(שלום)", "abc (שלום) def", "שלום (abc) עולם",
    "مرحبا", "مرحبا بالعالم.", "أرقام ١٢٣٤.", "عربي 123", "1.5", "a1.5", "ש1.5",
    "‫abc‬", "‮abc‬ def", "⁧abc⁩ ש", "⁨שלום⁩ abc",
    "a⁦b⁩c", "abc⁧", "x \t y", "ש \t a", "א b", "a א", "a\r\nא",
    "[(a)ש]", "a[ש(b)]c", "ש[a(ש)]", "〈a〉 ש", "〈ש〉", "áשִ",
    "\U00010900\U00010901 a", "a \U0001F600 ש", "$12 ש", "ש -12%", "ש 1,234.56 abc",
    "a‍b", "ש‍ש", " ש ", "   ", "!?", "ש!", "a!", "١٢", "١٢ a", "ש ١٢ a",
]

ALPHABET = list("aZ09+-$%,.:/ \t!()[]{}<>\"'") + [
    "א", "ב", "ִ", "ا", "ب", "١", "٢", "ً",
    "۱", "‍", "‎", "‏", "‪", "‫", "‬", "‭",
    "‮", "⁦", "⁧", "⁨", "⁩", " ", " ", "〈",
    "〉", "〈", "〉", "°", "،", "̀", "­",
    "\U00010900", "\U0001F600", "\U00020000", "\r", "\n", "\u001c", "\u0085", "−",
]


def main():
    v = (ctypes.c_uint8 * 4)()
    icu.u_getVersion(v)
    rnd = random.Random(20261012)
    cases = list(HAND)
    for _ in range(1000):
        n = rnd.randint(1, 24)
        cases.append("".join(rnd.choice(ALPHABET) for _ in range(n)))
    with open(sys.argv[1], "w") as f:
        f.write(f"# ICU {v[0]}.{v[1]}.{v[2]} (macOS libicucore), tools: icu_oracle.py\n")
        for s in cases:
            u = utf16(s)
            for level in (0xFE, 0xFF):
                d, lv, runs = run(u, level)
                f.write(f"{level:02x}\t{''.join(f'{x:04x}' for x in u)}\t{d}\t{lv}\t{runs}\n")


main()
