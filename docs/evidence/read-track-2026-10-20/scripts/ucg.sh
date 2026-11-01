#!/usr/bin/env bash
# ucg.sh: the Unicode M3 latency rows under callgrind, main against READ-TRACK (one host at a time)
set -u
export PATH=$HOME/texlive/2026/bin/x86_64-linux:/nix/store/39iybd55afmpqlgf979ra06wj3q3wapa-valgrind-3.27.1/bin:$HOME/.nix-profile/bin:$PATH
tl=$HOME/texlive/2026
export FLASHTEX_FONT_DIRS=$tl/texmf-dist/fonts/opentype:$tl/texmf-dist/fonts/truetype
O=~/rt/ucg; rm -rf $O; mkdir -p $O
cd ~/code/flashtex-readtrack
for v in base:$HOME/code/flashtex-readtrack-base new:$HOME/code/flashtex-readtrack; do
  n=${v%%:*}; w=${v#*:}
  for at in end EDITPOINT; do
    for doc in tools/unicode-m3/bench/b*.tex; do
      d=$(basename $doc .tex)
      [ $at = EDITPOINT ] && [ $d != b4-fontspec-pages ] && continue
      export FLASHTEX_CACHE_DIR=$O/cache-$n-$d
      python3 tools/incr-bench/unicode.py $w/crates/flashtex-xetex/target/release/flashtex-host-unicode $doc --doc $d --keys 3 --at $at --callgrind $O/cg-$n-$at-$d --out $O/$n-$at-$d.jsonl > /dev/null 2> $O/$n-$at-$d.err; echo "$n $at $d rc=$?"
    done
  done
done
python3 - $O <<"PY"
import glob, json, os, sys
for f in sorted(glob.glob(os.path.join(sys.argv[1], "*.jsonl"))):
    rows = [json.loads(l) for l in open(f)]
    print(os.path.basename(f)[:-6], " | ".join("%s %s/%s" % (r["tag"], r.get("edited_instructions_m"), r.get("instructions_m")) for r in rows))
PY
