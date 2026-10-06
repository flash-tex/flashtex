#!/usr/bin/env bash
# The P5 scoreboard, end to end (DESIGN §12 P5, §8): build both engines from
# this checkout, run every tier's existing harness for each engine against the
# oracle, then aggregate with tools/parity/scoreboard.py. The nightly workflow
# runs exactly this; so can anyone with TeX Live 2026.
#
#   engines   new = crates/flashtex-engine's flashtex-initex (with its own
#             pdflatex.fmt / pdftex.fmt, built as fmtutil does, like
#             scripts/engine-parity.sh); old = v1, the flashtex CLI
#   parity    tools/parity/parity.py on --tiers (default: fixtures arxiv
#             templates, plus packages when its manifest exists), both engines
#   t2        tools/latex-suites/run.py through this host's pdfTeX (the
#             baseline) and through the new engine
#   smoke     tools/package-smoke/run.py, new engine
#   fonts     tools/font-census/census.py, new engine
#   t4        not run here: pass --t4-new/--t4-old with nightly.py output
#             directories (nightly.yml's corpus-t4 or corpus-t4-mac.yml's
#             artifacts). They must be at this checkout's commit, with the same
#             engine binaries and this host's oracle (texlive.tlpdb and pdftex
#             sha256, OUT/oracle.json): any other run reads INVALID
#
# v1 cannot run t2, smoke, fonts or P-T1 (they need a pdfTeX-compatible
# binary); the board says "old n/a" for those, with the reason.
#
# Usage: tools/parity/scoreboard-run.sh --out DIR [--work DIR] [--keep-work] [--jobs N]
#          [--tiers "fixtures arxiv ..."] [--limit N] [--skip "t2 fonts ..."]
#          [--t4-new DIR] [--t4-old DIR] [--sample-note TEXT] [--] [scoreboard.py args...]
# --limit N runs the first N documents per parity tier and 20 T2 tests (a
# sample; the board is then never all-green). Needs TeX Live 2026 first on
# PATH, python3 and qpdf. Parity runs use -j N (default 2).
#
# Disk (P5-BOARD-DISK: the first Mac board, run 37113092020, died after 5 h
# with "No space left on device"). Every harness writes under --work (default
# OUT/work), its TMPDIR too, and what the script made there is removed on exit
# unless --keep-work; parity.py removes a document's work directory as soon as
# it is scored, and no traced log reaches the disk (its pt1_plan). A guard
# reads the free space of OUT, WORK and the parity cache every
# FLASHTEX_BOARD_DISK_POLL_S seconds (default 30) while a stage runs. Under
# FLASHTEX_BOARD_MIN_FREE_GB (default 10) it stops that stage (SIGTERM to its
# process tree, which the harnesses turn into their own cleanup; SIGKILL to
# what is left after 60 s), starts no other, and still writes the board, whose
# unfinished rows read missing or invalid: a ::error:: names the directory and
# its free space, and the exit status is 1. It stops rather than pauses: a
# pause would run into the harnesses' wall-clock time limits and turn into
# harness errors.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

OUT="" WORK="" KEEP_WORK=0 JOBS=2 TIERS="" LIMIT=0 SKIP="" T4NEW="" T4OLD="" NOTE=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --out) OUT="${2:?}"; shift 2 ;;
    --work) WORK="${2:?}"; shift 2 ;;
    --keep-work) KEEP_WORK=1; shift ;;
    --jobs) JOBS="${2:?}"; shift 2 ;;
    --tiers) TIERS="${2:?}"; shift 2 ;;
    --limit) LIMIT="${2:?}"; shift 2 ;;
    --skip) SKIP="${2:?}"; shift 2 ;;
    --t4-new) T4NEW="${2:?}"; shift 2 ;;
    --t4-old) T4OLD="${2:?}"; shift 2 ;;
    --sample-note) NOTE="${2:?}"; shift 2 ;;
    -h|--help) awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"; exit 0 ;;
    --) shift; break ;;
    *) echo "scoreboard-run.sh: unknown argument: $1" >&2; exit 2 ;;
  esac
done
[[ -n "$OUT" ]] || { echo "scoreboard-run.sh: --out DIR is required" >&2; exit 2; }
WORK="${WORK:-$OUT/work}"
mkdir -p "$OUT" "$WORK"
OUT="$(cd "$OUT" && pwd)"; WORK="$(cd "$WORK" && pwd)"
skip() { [[ " $SKIP " == *" $1 "* ]]; }

# ---- the disk guard (see the header) ---------------------------------------------
MIN_FREE_GB="${FLASHTEX_BOARD_MIN_FREE_GB:-10}" DISK_POLL="${FLASHTEX_BOARD_DISK_POLL_S:-30}"
GUARDED=("$OUT" "$WORK" "${FLASHTEX_PARITY_CACHE:-$HOME/.cache/flashtex-parity}")
DISK_LOW=""
exec 3>&2  # the guard's messages reach the job log while a stage writes to its own log
low_disk() {  # "DIR FREE_GB" for the first guarded directory whose disk is under the floor
  local d kb
  for d in "${GUARDED[@]}"; do
    [[ -d $d ]] || continue
    kb=$(df -Pk "$d" | awk 'NR == 2 { print $4 }')  # POSIX columns on macOS and Linux
    if ((kb < MIN_FREE_GB * 1048576)); then echo "$d $(awk -v k="$kb" 'BEGIN { printf "%.1f", k / 1048576 }')"; return 0; fi
  done
  return 1
}
disk_low() {  # disk_low STAGE "DIR FREE_GB"
  DISK_LOW="only ${2##* } GB free on the disk of ${2% *} (the floor is $MIN_FREE_GB GB: FLASHTEX_BOARD_MIN_FREE_GB)"
  echo "::error::scoreboard-run: $DISK_LOW; stopping $1 and starting no other stage: their rows read missing" >&3
}
tree_of() {  # PID and its descendants, as `ps` sees them now
  ps -A -o pid= -o ppid= | awk -v root="$1" '
    function walk(p,   n, a, i) { n = split(kids[p], a, " "); for (i = 1; i <= n; i++) walk(a[i]); print p }
    { kids[$2] = kids[$2] " " $1 }
    END { walk(root) }'
}
stop_tree() {  # SIGTERM to PID's whole tree (T2's l3build runs in a session of its own), then SIGKILL
  local pids p pp i
  pids=" $(tree_of "$1" | tr "\n" " ")"
  # shellcheck disable=SC2086 # a list of pids
  kill -TERM $pids 2>/dev/null || true
  for ((i = 0; i < 60; i++)); do kill -0 "$1" 2>/dev/null || break; sleep 1; done
  for p in $pids; do  # a survivor: still in the tree, or orphaned (ppid 1); never a reused pid
    pp=$(ps -o ppid= -p "$p" 2>/dev/null | tr -d ' ') || continue
    if [[ $pp == 1 || "$pids" == *" $pp "* ]]; then kill -KILL "$p" 2>/dev/null || true; fi
  done
}
guard() {  # guard STAGE CMD...: CMD in the background, stopped when the disk runs low; 125 = stopped
  local name=$1 low pid rc i; shift
  [[ -z $DISK_LOW ]] || return 125  # not started: note_fail says so
  if low=$(low_disk); then disk_low "$name" "$low"; return 125; fi
  "$@" 3>&- &
  pid=$!
  while :; do
    for ((i = 0; i < DISK_POLL; i++)); do kill -0 "$pid" 2>/dev/null || break 2; sleep 1; done
    if low=$(low_disk); then disk_low "$name" "$low"; stop_tree "$pid"; break; fi
  done
  rc=0; wait "$pid" || rc=$?
  [[ -z $DISK_LOW ]] || return 125
  return $rc
}
# What this script makes under WORK; removed on exit (--keep-work keeps it).
cleanup() {
  [[ $KEEP_WORK == 1 ]] && return
  rm -rf "$WORK"/{eng,fmt,texmfvar,tmp,parity-new,parity-old}
  rmdir "$WORK" 2>/dev/null || true
}
trap cleanup EXIT
if low=$(low_disk); then
  echo "::error::scoreboard-run: only ${low##* } GB free on the disk of ${low% *} (the floor is $MIN_FREE_GB GB): not starting" >&2
  exit 1
fi

PDFTEX="$(command -v pdftex)" || { echo "::error::no pdftex on PATH (TeX Live 2026 is the oracle)" >&2; exit 1; }
TEXBIN="$(dirname "$PDFTEX")"
TEXMF="$(kpsewhich -var-value TEXMFDIST)"
"$PDFTEX" --version | sed -n 1p
if [[ -z "$TIERS" ]]; then
  TIERS="fixtures arxiv templates"
  compgen -G "tools/parity/corpus/packages-*.json" >/dev/null && TIERS="$TIERS packages"
fi
# A scratch TEXMFVAR, so no run writes into the user's or the runner's own.
export TEXMFVAR="$WORK/texmfvar"
mkdir -p "$TEXMFVAR"

# ---- engines -----------------------------------------------------------------
TARGET="${CARGO_TARGET_DIR:-target}"
ENG="$WORK/eng" FMT="$WORK/fmt"
rm -rf "$ENG" "$FMT"; mkdir -p "$ENG" "$FMT"
# The engine alone, exactly as nightly.yml's corpus-t4 job builds it, so its
# sha256 can match the T4 run's (scoreboard.py compares them). Built together
# with flashtex-cli in one cargo invocation, shared dependencies get unified
# features and the binary differs: measured on mac-m5pro-dq222 at 296c90197,
# combined 4e2588473a87..., alone 3a9ff3dcca4e... (alone is the same in two
# different target directories). Copied out before the v1 build runs.
cargo build --release --locked -p flashtex-engine --bin flashtex-initex
cp "$TARGET/release/flashtex-initex" "$ENG/"
cargo build --release --locked -p flashtex-cli --bin flashtex
cp "$TARGET/release/flashtex" "$ENG/"
cp crates/flashtex-engine/pdftex.pool "$ENG/pdftex.pool"
POOL="$ENG/pdftex.pool" INITEX="$ENG/flashtex-initex" V1="$ENG/flashtex"
for f in pdflatex pdftex; do
  ini=$f.ini; [[ $f == pdftex ]] && ini=pdfetex.ini
  (cd "$FMT" && SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL="$POOL" "$INITEX" -ini -jobname=$f \
     -progname=$f -etex -translate-file=cp227.tcx "$ini" </dev/null >"$FMT/$f.out" 2>&1) && [[ -s "$FMT/$f.fmt" ]] ||
    { tail -n 40 "$FMT/$f.out" >&2; echo "::error::the engine did not build $f.fmt" >&2; exit 1; }
done
# The full commit: nightly.py records it too, and scoreboard.py marks a T4 run at any
# other commit INVALID (a stale or foreign run).
SHA="$(git rev-parse HEAD)"

ARGS=(--sha "new=$SHA" --sha "old=$SHA" --out "$OUT/board")
# The board's oracle, down to its texlive.tlpdb and pdftex binary (DESIGN §8): a T4
# summary measured against any other reads INVALID (the version string alone is the
# same on the PC and the Macs). The workflow writes OUT/oracle.json first.
[[ -f "$OUT/oracle.json" ]] || python3 tools/parity/oracle_provenance.py --texbin "$TEXBIN" --json "$OUT/oracle.json"
ARGS+=(--oracle "$OUT/oracle.json")
[[ -n "$NOTE" ]] && ARGS+=(--sample-note "$NOTE")
rc_all=0
note_fail() {
  if [[ $2 -eq 125 && -n $DISK_LOW ]]; then echo "::warning::$1 stopped or not started by the disk guard; its row reads missing or invalid" >&2
  else echo "::warning::$1 did not complete (exit $2); its row reads missing or invalid" >&2; fi
  rc_all=1
}
# Every harness's temporary files (T2's shims, package-smoke's runs) under WORK too.
export TMPDIR="$WORK/tmp"
mkdir -p "$TMPDIR"

# ---- parity tiers, both engines ------------------------------------------------
if ! skip parity; then
  margs=()
  for t in $TIERS; do
    [[ $t == fixtures ]] && continue
    for m in tools/parity/corpus/"$t"-*.json; do [[ -f $m ]] && margs+=(--manifest "$m"); done
  done
  if [[ ${#margs[@]} -gt 0 ]]; then
    rc=0; guard "corpus fetch" python3 tools/parity/corpus.py --texmf "$TEXMF" fetch "${margs[@]}" || rc=$?
    [[ $rc -eq 125 ]] && note_fail "corpus fetch" $rc
    [[ $rc -eq 0 || $rc -eq 125 ]] || exit $rc
  fi
  targs=(); for t in $TIERS; do targs+=(--tier "$t"); done
  [[ $LIMIT -gt 0 ]] && targs+=(--limit "$LIMIT")
  common=(--texbin "$TEXBIN" --oracle-pdftex "$PDFTEX" --texmf "$TEXMF" --raster none -j "$JOBS")
  rc=0; guard "parity (new)" python3 tools/parity/parity.py "${targs[@]}" "${common[@]}" --engine "$INITEX" \
    --engine-env "FLASHTEX_FORMATS=$FMT" --engine-env "FLASHTEX_POOL=$POOL" \
    --out "$OUT/parity-new" --work "$WORK/parity-new" >"$OUT/parity-new.log" 2>&1 || rc=$?
  [[ $rc -eq 0 ]] || note_fail "parity (new)" $rc
  [[ -f "$OUT/parity-new/scoreboard.json" ]] && ARGS+=(--parity "new=$OUT/parity-new")
  rc=0; guard "parity (old)" python3 tools/parity/parity.py "${targs[@]}" "${common[@]}" --engine "$V1" \
    --out "$OUT/parity-old" --work "$WORK/parity-old" >"$OUT/parity-old.log" 2>&1 || rc=$?
  [[ $rc -eq 0 ]] || note_fail "parity (old)" $rc
  [[ -f "$OUT/parity-old/scoreboard.json" ]] && ARGS+=(--parity "old=$OUT/parity-old")
fi

# ---- T2: this host's pdfTeX is the baseline ------------------------------------
if ! skip t2; then
  rc=0; guard "T2 fetch" sh tools/latex-suites/fetch.sh >"$OUT/t2-fetch.log" 2>&1 || rc=$?
  [[ $rc -eq 0 || $rc -eq 125 ]] || { tail -n 20 "$OUT/t2-fetch.log" >&2; exit $rc; }
  if [[ $rc -eq 125 ]]; then
    note_fail "T2" $rc  # no checkouts: nothing below can run
  else
    sargs=(--suite all)
    if [[ $LIMIT -gt 0 ]]; then
      # the first 20 latex2e/base tests (a glob, not `ls | head`: head's SIGPIPE
      # would end the script under pipefail)
      lvts=(tools/latex-suites/.cache/latex2e/base/testfiles/*.lvt)
      names=(); for f in "${lvts[@]:0:20}"; do f=${f##*/}; names+=("${f%.lvt}"); done
      tests=$(IFS=,; echo "${names[*]}")
      sargs=(--suite base --tests "$tests")
    fi
    # The full-suite denominator, from the same checkouts: a run of fewer tests is partial.
    python3 tools/latex-suites/run.py --engine "$PDFTEX" --suite all --list >"$OUT/t2-list.txt"
    rc=0; guard "T2 (pdfTeX reference)" python3 tools/latex-suites/run.py --engine "$PDFTEX" "${sargs[@]}" --allow-stale \
      >"$OUT/t2-reference.txt" 2>&1 || rc=$?
    [[ $rc -le 1 ]] || note_fail "T2 (pdfTeX reference)" $rc
    rc=0; guard "T2 (new)" python3 tools/latex-suites/run.py --engine "$INITEX" "${sargs[@]}" --allow-stale --allow-any-engine \
      --engine-env "FLASHTEX_FORMATS=$FMT" --engine-env "FLASHTEX_POOL=$POOL" >"$OUT/t2-new.txt" 2>&1 || rc=$?
    [[ $rc -le 1 ]] || note_fail "T2 (new)" $rc
    ARGS+=(--latex-suites "new=$OUT/t2-new.txt" --latex-suites-reference "$OUT/t2-reference.txt"
           --latex-suites-list "$OUT/t2-list.txt")
  fi
fi

# ---- package-smoke --------------------------------------------------------------
if ! skip smoke; then
  rc=0; guard "package-smoke (new)" env FLASHTEX_FORMATS="$FMT" FLASHTEX_POOL="$POOL" \
    python3 tools/package-smoke/run.py --candidate "$INITEX" --reference "$PDFTEX" >"$OUT/smoke-new.txt" 2>&1 || rc=$?
  [[ $rc -le 1 ]] || note_fail "package-smoke (new)" $rc
  ARGS+=(--package-smoke "new=$OUT/smoke-new.txt")
fi

# ---- fonts ------------------------------------------------------------------------
if ! skip fonts; then
  rc=0; guard "font census (new)" python3 tools/font-census/census.py --engine "$INITEX" --pool "$POOL" --texbin "$TEXBIN" \
    -j "$JOBS" --out "$OUT/fonts-new" >"$OUT/fonts-new.log" 2>&1 || rc=$?
  [[ $rc -le 1 ]] || note_fail "font census (new)" $rc
  [[ -f "$OUT/fonts-new/census.json" ]] && ARGS+=(--fonts "new=$OUT/fonts-new")
fi

# ---- T4 (from the nightly corpus job) ------------------------------------------
[[ -n "$T4NEW" && -f "$T4NEW/summary.json" ]] && ARGS+=(--nightly "new=$T4NEW")
[[ -n "$T4OLD" && -f "$T4OLD/summary.json" ]] && ARGS+=(--nightly "old=$T4OLD")

if [[ -n $DISK_LOW ]]; then
  cleanup  # room for the board, which is small
  echo "::error::scoreboard-run: the board is partial: $DISK_LOW" >&2
fi
python3 tools/parity/scoreboard.py "${ARGS[@]}" "$@"
exit $rc_all
