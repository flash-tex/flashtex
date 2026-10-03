#!/usr/bin/env bash
# capped.sh SECONDS CMD...: CMD under the NixOS PC's project memory cap (Commander rule, 2026-10-01):
# the user slice flashtex.slice (20 GB shared with the CI runners), 10 GB for this command, no
# swap, nice 10, a time limit. Every lane command on the PC goes through it; at most 4 engine hosts.
T=$1; shift
exec systemd-run --user --scope --quiet --slice=flashtex.slice -p MemoryMax=10G -p MemorySwapMax=0 \
  nice -n 10 timeout "$T" "$@"
