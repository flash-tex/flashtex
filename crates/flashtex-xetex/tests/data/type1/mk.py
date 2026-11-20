#!/usr/bin/env python3
"""Make FlashTeX's test Type 1 font `ftxbox` (ftxbox.pfb) and its PL file
(ftxbox.tfm is `pltotf ftxbox.pl`). Usage: mk.py OUTDIR. Public domain.

Glyphs, for the FontDescriptor's /CapHeight, /Ascent and /Descent:
  .notdef  0 500 hsbw endchar
  H        a box with side bearing 40, its top 1367 2 div (683.5)
  p        from 0 -194 (plus the side bearing) up, then a curve whose
           control points go down to -369 while it ends at -194
  b        hints, then its path in Subr 5 (callsubr), top 694
  space    0 250 hsbw endchar
"""
import struct, sys

def enc_num(v):
    if -107 <= v <= 107:
        return bytes([v + 139])
    if 108 <= v <= 1131:
        w = v - 108
        return bytes([247 + w // 256, w % 256])
    if -1131 <= v <= -108:
        w = -v - 108
        return bytes([251 + w // 256, w % 256])
    return b'\xff' + struct.pack('>i', v)

OPS = {'hstem': 1, 'vstem': 3, 'vmoveto': 4, 'rlineto': 5, 'hlineto': 6, 'vlineto': 7,
       'rrcurveto': 8, 'closepath': 9, 'callsubr': 10, 'return': 11, 'hsbw': 13,
       'endchar': 14, 'rmoveto': 21, 'hmoveto': 22, 'vhcurveto': 30, 'hvcurveto': 31,
       'div': (12, 12), 'callothersubr': (12, 16), 'pop': (12, 17), 'setcurrentpoint': (12, 33)}

def cs(prog):
    out = b''
    for tok in prog.split():
        if tok in OPS:
            o = OPS[tok]
            out += bytes(o) if isinstance(o, tuple) else bytes([o])
        else:
            out += enc_num(int(tok))
    return out

def encrypt(plain, r, prefix=b''):
    c1, c2 = 52845, 22719
    out = bytearray()
    for b in prefix + plain:
        c = b ^ (r >> 8)
        r = ((c + r) * c1 + c2) & 0xffff
        out.append(c)
    return bytes(out)

GLYPHS = {
    '.notdef': '0 500 hsbw endchar',
    'H': '40 700 hsbw 0 0 rmoveto 620 0 rlineto 0 1367 2 div rlineto -620 0 rlineto closepath endchar',
    'p': '55 556 hsbw 0 -194 rmoveto 0 625 rlineto 100 50 300 50 400 -100 rrcurveto 0 -300 vlineto -100 -500 -300 0 -400 175 rrcurveto closepath endchar',
    'b': '30 556 hsbw 0 20 hstem 650 44 hstem 5 callsubr endchar',
    'space': '0 250 hsbw endchar',
}
SUBRS = [
    '3 0 callothersubr pop pop setcurrentpoint return',
    '0 1 callothersubr return',
    '0 2 callothersubr return',
    'return',
    '1 3 callothersubr pop callsubr return',
    '0 0 rmoveto 0 694 rlineto 400 -694 rlineto closepath return',
]

def pfb():
    clear = b"""%!PS-AdobeFont-1.0: FTXBox 001.000
%%Title: FTXBox, a test font for FlashTeX (public domain)
12 dict begin
/FontInfo 4 dict dup begin
/version (001.000) readonly def
/FamilyName (FTXBox) readonly def
/ItalicAngle 0 def
/isFixedPitch false def
end readonly def
/FontName /FTXBox def
/PaintType 0 def
/FontType 1 def
/FontMatrix [0.001 0 0 0.001 0 0] readonly def
/Encoding StandardEncoding def
/FontBBox {-10 -250 1000 900} readonly def
currentdict end
currentfile eexec
"""
    priv = b"dup /Private 8 dict dup begin\n/RD{string currentfile exch readstring pop}executeonly def\n" \
           b"/ND{noaccess def}executeonly def\n/NP{noaccess put}executeonly def\n" \
           b"/BlueValues [-20 0 683 703] def\n/StdVW [80] def\n/MinFeature{16 16} def\n/password 5839 def\n"
    priv += b"/Subrs %d array\n" % len(SUBRS)
    for i, s in enumerate(SUBRS):
        e = encrypt(cs(s), 4330, b'\0\0\0\0')
        priv += b"dup %d %d RD " % (i, len(e)) + e + b" NP\n"
    priv += b"ND\n2 index /CharStrings %d dict dup begin\n" % len(GLYPHS)
    for n, s in GLYPHS.items():
        e = encrypt(cs(s), 4330, b'\0\0\0\0')
        priv += b"/%s %d RD " % (n.encode(), len(e)) + e + b" ND\n"
    priv += b"end\nend\nreadonly put\nnoaccess put\ndup /FontName get exch definefont pop\nmark currentfile closefile\n"
    enc = encrypt(priv, 55665, b'\0\0\0\0')
    trailer = (b"0" * 64 + b"\n") * 8 + b"cleartomark\n"
    seg = lambda t, d: bytes([0x80, t]) + struct.pack('<I', len(d)) + d
    return seg(1, clear) + seg(2, enc) + seg(1, trailer) + b'\x80\x03'

PL = """(FAMILY FTXBOX)
(DESIGNSIZE R 10.0)
(FONTDIMEN
   (SLANT R 0.0)
   (SPACE R 0.25)
   (XHEIGHT R 0.43)
   (QUAD R 1.0)
   )
(CHARACTER C H (CHARWD R 0.7) (CHARHT R 0.6835))
(CHARACTER C p (CHARWD R 0.556) (CHARHT R 0.43) (CHARDP R 0.194))
(CHARACTER C b (CHARWD R 0.556) (CHARHT R 0.694))
"""

open(sys.argv[1] + '/ftxbox.pfb', 'wb').write(pfb())
open(sys.argv[1] + '/ftxbox.pl', 'w').write(PL)
