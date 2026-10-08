#!/usr/bin/env bash
export INCR_BENCH_DIR=$HOME/ib-memr PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
python3 -u ~/memr/memr.py ~/ib-memr/memr ~/bench-docs/infdesc infdesc-x2.tex ~/ib-memr/inf/x2-base --interval-ms 50 \
  --step book/number-theory/divisibility.tex:0.5:letter:4 \
  --step book/number-theory/divisibility.tex:0.5:sentence:2 \
  --step book/relations/equivalence-relations.tex:0.5:typing:20 > ~/ib-memr/inf/x2-base.out 2>&1
