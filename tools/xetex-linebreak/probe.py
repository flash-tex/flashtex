#!/usr/bin/env python3
"""Measure how TeX Live's xetex (an oracle only, never in the product path)
cuts native words under \\XeTeXlinebreaklocale, and write the fixture of
crates/flashtex-xetex/src/native/linebreak.rs's test:

    python3 tools/xetex-linebreak/probe.py --xetex xetex \\
        > crates/flashtex-xetex/src/native/linebreak_xetex.tsv

Each word is one paragraph of Latin Modern Roman with \\hyphenchar -1 and no
spaces, so the whole word reaches ICU's line breaker as one text; the
offsets are where \\showbox shows \\XeTeXlinebreakpenalty between its pieces.
The breaks depend on ICU, not on the font (a missing glyph changes nothing).
The words: one per locale and script, then 240 random ones (seed 20261012).
"""
import argparse
import os
import random
import re
import subprocess
import tempfile
import unicodedata

FIXED = [
    ("en", "ef-gh/ij"), ("en", "don’t-stop(1)2"), ("hi", "हिन्दीभाषा-पाठ"), ("en", "abcd"),
    ("en", "12.5%(kl)[x]{y}"), ("en", "a—b–c‐d"), ("en", "http://x.org/a?b=c&d"),
    ("en", "“quoted”‘s’"), ("en", "1,000.00$€100"), ("en", "a\u200bb\u00adc\u2060d"),
    ("en", "😀😀👍🏽x"), ("en", "Ａ全角，ＢＣ。"),
    ("zh", "七百五十三“中文”。（括号）、ab"), ("zh", "他说：“你好！”——然后…走了。"),
    ("zh", "〜々〻ゝ・：；！？"), ("zh", "2026年10月12日，100%完成"),
    ("zh_TW", "臺灣「繁體」『中文』，測試。"), ("zh-CN", "简体中文“引号”测试"),
    ("ja", "日本語のテキストァィ“中文”。"), ("ja", "ちょっと、キャッシュ〜ゝヽ！？"),
    ("ja", "「カギ括弧」です。ーー"), ("ko", "한국어텍스트입니다.(괄호)"),
    ("ja@lb=strict", "ちょっと、キャッシュ〜ゝ"), ("ja@lb=loose", "ちょっと、キャッシュ〜ゝ・！？100%"),
    ("ja@lb=normal", "ちょっと、キャッシュ〜ゝ・！？“引”"), ("zh@lb=loose", "中文〜々・：；！？“引”100%"),
    ("en@lb=loose", "a-b‐c…d"), ("en@lb=normal", "ァィゥ"),
    ("th", "ภาษาไทยเป็นภาษาที่ไม่มีการเว้นวรรคระหว่างคำ"), ("th", "สวัสดีครับผมชื่อสมชาย"),
    ("km", "ភាសាខ្មែរគឺជាភាសាផ្លូវការ"), ("kh", "ភាសាខ្មែរគឺជាភាសាផ្លូវការ"),
    ("lo", "ພາສາລາວເປັນພາສາທາງການ"), ("my", "မြန်မာဘာသာစကားသည်"),
    ("bo", "བོད་ཀྱི་སྐད་ཡིག"), ("en", "שלום-עולם"), ("ar", "مرحبابالعالم"),
    ("th", "ประเทศไทยมีประชากรประมาณ70ล้านคน"), ("th", "กรุงเทพมหานครเป็นเมืองหลวงของประเทศไทย"),
    ("en", "Thai123ภาษาABCไทย"), ("th", "ๆเด็กๆเล่นกันฯลฯ"), ("th", "ภาษา"), ("th", "ภาษาไ"),
    ("km", "ព្រះរាជាណាចក្រកម្ពុជា"), ("lo", "ສາທາລະນະລັດປະຊາທິປະໄຕປະຊາຊົນລາວ"),
    ("my", "ပြည်ထောင်စုသမ္မတမြန်မာနိုင်ငံတော်"), ("en", "ᨲᩫ᩠ᩅᨾᩮᩬᩥᨦ"),
]

POOL = list("abcXYZéß0123456789.,:;!?-/()[]{}%$€£¥#&*+=<>'\"") + [
    "\u2014", "\u2013", "\u2010", "\u2026", "\u00a0", "\u200b", "\u2060", "\u00ad", "\u201c",
    "\u201d", "\u2018", "\u2019", "\u00ab", "\u00bb", "\u3001", "\u3002", "\uff0c", "\uff08",
    "\uff09", "\u300c", "\u300d", "\u301c", "\u30fc", "\u3005", "\u30a1", "\u3041", "\u3063",
    "\u30c3", "\u30fb", "\uff01", "\uff1f", "\u4e2d", "\u6587", "\u5b57", "\u65e5", "\u672c",
    "\u3042", "\u30a2", "\ud55c", "\uae00", "\u05d0", "\u05e9", "\u0627", "\u0644", "\u0915",
    "\u094d", "\u093f", "\u0f40", "\u0f0b", "\U0001F600", "\U0001F44D", "\U0001F3FD", "\u200d",
    "\u2764", "\ufe0f", "\U0001F1EF", "\U0001F1F5", "\u0301", "\u2060", "\u3000", "\u2028",
    "\u00b0", "\u2032", "\u0e01", "\u0e32", "\u0e40", "\u0e48"]
LOCALES = ["en", "zh", "ja", "ko", "ja@lb=strict", "zh@lb=loose", "en@lb=loose",
           "ja@lb=normal", "de", "zh_TW"]


def random_words():
    rng = random.Random(20261012)
    out = []
    for k in range(240):
        s = "".join(rng.choice(POOL) for _ in range(rng.randint(2, 12)))
        s = s.replace("\u3000", "").replace("\u2028", "")
        if len(s) < 2:
            s = "ab" + s
        out.append((LOCALES[k % len(LOCALES)], s))
    return out


def tex_escape(s):
    return "".join("\\char\"%X " % ord(c) if c in "{}\\#%$&^_~<>[]" else c for c in s)


def tex_source(cases):
    out = [r"\catcode`\{=1 \catcode`\}=2 \catcode`\#=6 \showboxdepth=100 \showboxbreadth=10000",
           r"\hsize=16000pt \parfillskip=0pt plus 1fil \tolerance=10000 \parindent=0pt",
           r'\font\x="[lmroman10-regular.otf]" \x \hyphenchar\x=-1 \XeTeXlinebreakpenalty=1',
           r"\tracinglostchars=0 \tracingonline=1",
           r"\def\T#1#2#3{\setbox0\vbox{\XeTeXlinebreaklocale #2 \noindent #3\par}"
           r"\message{[CASE #1]}\showbox0}",
           r"\catcode`\<=1 \catcode`\>=2 \catcode`\[=12 \catcode`\]=12 \catcode`\{=12 \catcode`\}=12"]
    for i, (loc, s) in enumerate(cases):
        out.append(r'\T<%d><"%s"><%s>' % (i, loc, tex_escape(s)))
    out.append(r"\end")
    return "\n".join(out) + "\n"


def escape(text):
    """Format and separator characters (and any above the BMP) as \\uXXXX
    of their UTF-16 code units."""
    out = []
    for c in text:
        if unicodedata.category(c)[0] in "CZ" or c == "\\":
            for u in range(0, len(c.encode("utf-16-le")), 2):
                out.append("\\u%04x" % int.from_bytes(c.encode("utf-16-le")[u:u + 2], "little"))
        else:
            out.append(c)
    return "".join(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--xetex", default="xetex")
    a = ap.parse_args()
    cases = FIXED + random_words()
    with tempfile.TemporaryDirectory() as d:
        open(os.path.join(d, "p.tex"), "w", encoding="utf-8").write(tex_source(cases))
        subprocess.run([a.xetex, "-ini", "-etex", "-no-pdf", "-interaction=nonstopmode", "p.tex"],
                       cwd=d, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        log = open(os.path.join(d, "p.log"), encoding="utf-8").read()
    ver = log.split("\n", 1)[0]
    parts = re.split(r"\[CASE (\d+)\]", log)
    pieces = {}
    for k in range(1, len(parts), 2):
        pieces[int(parts[k])] = [m.group(1) for m in re.finditer(r"^\.\.\\x (.*)$", parts[k + 1], re.M)]
    print("# %s: \\XeTeXlinebreaklocale LOCALE, then TEXT" % ver.split(" (INITEX)")[0])
    print("# as one native word in a paragraph; the offsets (UTF-16) at which its")
    print("# \\showbox puts \\XeTeXlinebreakpenalty, and the end. Written by")
    print("# tools/xetex-linebreak/probe.py: one word per locale and script, then 240")
    print("# random ones. LOCALE<TAB>TEXT (\\uXXXX escapes)<TAB>OFFSETS")
    for i, (loc, s) in enumerate(cases):
        p = pieces.get(i, [])
        if "".join(p) != s:
            raise SystemExit("case %d (%s %r): xetex's pieces %r do not make the word" % (i, loc, s, p))
        offs, n = [], 0
        for x in p:
            n += len(x.encode("utf-16-le")) // 2
            offs.append(n)
        print("%s\t%s\t%s" % (loc, escape(s), ",".join(map(str, offs))))


if __name__ == "__main__":
    main()
