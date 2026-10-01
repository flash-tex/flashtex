#!/usr/bin/env bash
# The md5 in the .aux after a .bbl change, through `flashtex-host iserve`.
set -u
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
export LD_LIBRARY_PATH=$HOME/p5x-libxcrypt-legacy/lib
B=/tmp/p5x-dev
SRC=${SRC:-/home/kubar/p5x/corpus/biblatex-01-introduction}
MAIN=${MAIN:-01-introduction}
W=/tmp/p5x-repro
rm -rf $W; mkdir -p $W; cp -r $SRC/. $W/; cd $W
rm -f *.aux *.bbl *.bcf *.run.xml
export FLASHTEX_FORMATS=$B/fmt FLASHTEX_POOL=$B/eng/pdftex.pool FLASHTEX_PIN_CLOCK=0 SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
H="$B/eng/flashtex-host iserve -- -fmt=pdflatex -interaction=nonstopmode -output-directory=$W $MAIN.tex"
mkfifo in.fifo
$H < in.fifo > out.jsonl 2> err.txt &
exec 3> in.fifo
say() { echo "$1" >&3; n=$(( $(wc -l < out.jsonl) + 1 )); until [ $(wc -l < out.jsonl) -ge $n ]; do sleep 0.2; done; tail -1 out.jsonl | python3 -c "import json,sys; r=json.loads(sys.stdin.read()); print({k: r.get(k) for k in ('mode','passes','pass_modes','restart_pages','rerun_pages','l5','cold_reason')})"; }
say compile
grep mdfive $MAIN.aux
biber $MAIN >/dev/null 2>&1; md5sum $MAIN.bbl
say compile
grep mdfive $MAIN.aux
od -c $MAIN.toc | head -2
cp $MAIN.aux aux-incr
eval "$EDIT"
say compile
biber $MAIN >/dev/null 2>&1; md5sum $MAIN.bbl
say compile
grep mdfive $MAIN.aux
echo quit >&3
wait
grep -a -i "mdfive\|\[l5\]\|\[incr\]" err.txt | cut -c1-400 | tail -30
