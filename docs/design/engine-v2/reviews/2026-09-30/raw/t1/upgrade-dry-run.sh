#!/usr/bin/env bash
# pdfTeX upgrade dry run (review 2026-09-30, track 1). Run from the repository root.
# Upstream has no pdfTeX newer than the pinned 1.40.29 (the last pdftex.web commit in
# TeX-Live/texlive-source is 2d38d53fe0, 2026-02-15, identical to our pin), so the dry
# run replays the real 1.40.28 -> 1.40.29 delta: our change files and translator against
# pdfTeX 1.40.28 (commit 741a186324).
set -u
U=${U:-/tmp/t1rev/upg}
mkdir -p "$U"
[ -f "$U/pdftex-1.40.28.web" ] || curl -sSfL -o "$U/pdftex-1.40.28.web" \
  https://raw.githubusercontent.com/TeX-Live/texlive-source/741a186324/texk/web2c/pdftexdir/pdftex.web
M=third_party/pdftex/pdftex.web
A=crates/flashtex-engine/web2rust-default.args
H=$(dirname "$0")
echo "# pdfTeX upgrade dry run: 1.40.28 (texlive-source 741a186324) vs the pinned 1.40.29"
echo "upstream delta: $(diff "$U/pdftex-1.40.28.web" $M | grep -c '^[<>]') changed lines in $(diff "$U/pdftex-1.40.28.web" $M | grep -c '^[0-9]') hunks"
echo
echo "## change-file hunks not found in the master (approximate matcher chcheck.py)"
echo "### pinned 1.40.29 (applies cleanly in web2rust; these are artefacts of the approximation)"
python3 "$H/chcheck.py" $M $A . | grep -E 'NO MATCH|TOTAL'
echo "### 1.40.28"
python3 "$H/chcheck.py" "$U/pdftex-1.40.28.web" $A . | grep -E 'NO MATCH|TOTAL'
echo
echo "## web2rust on 1.40.28 with the committed args"
./target/release/web2rust "$U/pdftex-1.40.28.web" @$A --out-dir "$U/gen-a" --pool "$U/a.pool" 2>&1 | tail -2
echo
echo "## after re-indenting that one hunk (precedence.ch change 2)"
python3 - "$U" <<'EOF'
import sys
u = sys.argv[1]
s = open('crates/flashtex-engine/changes/precedence.ch').read()
old = "          or get_font_auto_expand_ratio(f) <> get_font_auto_expand_ratio(pdf_f);\n@y\n          or (get_font_auto_expand_ratio(f)"
assert old in s
open(f'{u}/precedence-128.ch', 'w').write(s.replace(old, old.replace('          or', '        or')))
EOF
sed "s#crates/flashtex-engine/changes/precedence.ch#$U/precedence-128.ch#" $A > "$U/args-128"
rm -rf "$U/gen128"; mkdir -p "$U/gen128"
./target/release/web2rust "$U/pdftex-1.40.28.web" @"$U/args-128" --out-dir "$U/gen128" --pool "$U/pdftex-128.pool" 2>&1 | tail -3
echo "generated diff vs committed src/generated: $(diff -r "$U/gen128" crates/flashtex-engine/src/generated | grep -c '^[<>]') lines in $(diff -rq "$U/gen128" crates/flashtex-engine/src/generated | wc -l) files"
diff -r "$U/gen128" crates/flashtex-engine/src/generated | grep '^[<>]' | python3 -c '
import sys, re, collections
L = [l.rstrip() for l in sys.stdin]
n = lambda s: re.sub(r"\d+(i32)?", "N", s)
o = collections.Counter(n(l[2:]) for l in L if l[0] == "<")
w = collections.Counter(n(l[2:]) for l in L if l[0] == ">")
print("of which differ only in integer literals (string-pool numbers etc.):", 2 * sum((o & w).values()))'
