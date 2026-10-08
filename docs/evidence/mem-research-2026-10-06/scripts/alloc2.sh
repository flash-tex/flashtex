#!/usr/bin/env bash
# memr/alloc2.sh ENGINE DOC: one allocator variant on DOC (memr.py steps), into ~/ib-memr/alloc
export INCR_BENCH_DIR=$HOME/ib-memr PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
E=$1; D=$2
python3 -u ~/memr/memr.py ~/ib-memr/$E ~/ib-memr/docs/$D main.tex ~/ib-memr/alloc/$D-$E \
  --step :0.5:letter:6 --step :0.5:sentence:3 --step :0.98:letter:4 > ~/ib-memr/alloc/$D-$E.out 2>&1
