#!/usr/bin/env bash
# memr/inf.sh ENGINE TAG MAIN [HOSTARGS]: infdesc memory run
export INCR_BENCH_DIR=$HOME/ib-memr PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
E=$1; TAG=$2; M=${3:-infdesc.tex}
mkdir -p ~/ib-memr/inf
python3 -u ~/memr/memr.py ~/ib-memr/$E ~/bench-docs/infdesc $M ~/ib-memr/inf/$TAG --interval-ms 50 --host-args "${4:-}" \
  --step book/logical-structure/propositional-logic.tex:0.5:letter:8 \
  --step book/number-theory/divisibility.tex:0.5:letter:8 \
  --step book/real-numbers/series-sums.tex:0.5:letter:8 \
  --step book/number-theory/divisibility.tex:0.5:sentence:6 \
  --step book/relations/equivalence-relations.tex:0.5:typing:40 > ~/ib-memr/inf/$TAG.out 2>&1
