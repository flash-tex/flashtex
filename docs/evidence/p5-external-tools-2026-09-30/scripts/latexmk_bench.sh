#!/usr/bin/env bash
# latexmk's time to a resolved \cite after an edit, on the xbench documents (oracle, for comparison).
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
export LD_LIBRARY_PATH=$HOME/p5x-libxcrypt-legacy/lib SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
for doc in natbib-10 biblatex-10 natbib-120 biblatex-120; do
  W=/tmp/p5x-lmk-bench/$doc
  rm -rf $W; mkdir -p $W
  cp /tmp/p5x-gates/xwork/bench/$doc/doc/main.tex /tmp/p5x-gates/xwork/bench/$doc/doc/refs.bib $W/
  cd $W
  # the document as the bench opened it (its edits undone: take the generated one)
  python3 - <<EOF
import re
t=open("main.tex").read()
t=re.sub(r"See \\\\cite\{k\d+\}\. ","",t)
open("main.tex","w").write(t)
EOF
  s=$(date +%s.%N); latexmk -pdf -interaction=nonstopmode main.tex >/dev/null 2>&1; e=$(date +%s.%N)
  open_s=$(echo "$e - $s" | bc)
  n=$(grep -o 'cite{k[0-9]*}' main.tex | sed 's/[^0-9]//g' | sort -n | tail -1)
  times=""
  for r in 1 2 3; do
    k=$((n + 10 + r))
    python3 - <<EOF
t=open("main.tex","rb").read()
mid=t.find(b". ", len(t)//2)+2
open("main.tex","wb").write(t[:mid]+b"See \\\\cite{k$k}. "+t[mid:])
EOF
    s=$(date +%s.%N); latexmk -pdf -interaction=nonstopmode main.tex > lmk-$r.txt 2>&1; e=$(date +%s.%N)
    runs=$(grep -c "Run number" lmk-$r.txt)
    times="$times $(echo "$e - $s" | bc)s(${runs} runs)"
  done
  echo "$doc: open ${open_s}s; after a \\cite edit:$times"
done
