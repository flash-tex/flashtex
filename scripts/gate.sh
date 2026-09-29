#!/usr/bin/env bash
# FlashTeX local gates -- the local half of the tiered CI (DESIGN.md §9.1).
#
#   scripts/gate.sh quick     fmt + clippy + tests, scoped to what you changed
#   scripts/gate.sh pr        quick + parity fixtures + the licence boundary
#   scripts/gate.sh full      the whole workspace in both profiles + the Mac app
#
# Each tier runs exactly the steps .github/workflows/ci.yml runs for the same
# tier, so `scripts/gate.sh pr` before pushing means CI only confirms. See
# docs/ci-cd.md ("Tiers") for what each one is for and how long it takes.
#
# Scoping: `quick` derives the crates it touches from
#   git diff --name-only <base>...HEAD
# (base defaults to origin/main) plus, unless --committed-only, your uncommitted
# edits -- because the point of a local gate is to catch a problem before the
# commit, not after. A change to the root Cargo.toml or Cargo.lock adds one
# workspace-wide `cargo check` and one workspace-wide clippy run, not a test
# run per crate: the workspace's tests are the merge queue's job.
#
# Exit status is 1 if any step FAILED. WARN and SKIP do not fail the run; the
# summary table at the end says which is which and why.
set -euo pipefail
set -o pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BASE="${FLASHTEX_GATE_BASE:-origin/main}"
COMMITTED_ONLY=0
LIST_ONLY=0
JOBS="${FLASHTEX_GATE_JOBS:-}"
TIER=""

# Print this file's leading comment block (everything after the shebang, up to
# the first line that is not a comment). A line range would go stale the moment
# a paragraph is added -- which it did.
header() {
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
}

usage() {
  header
  cat <<'EOF'

Options:
  --base <ref>        compare against <ref> instead of origin/main
  --committed-only    ignore uncommitted edits when deciding what changed
  --jobs <n>          parallelism for the parity scoreboard (default: cores/2)
  --list              print the steps this tier would run, then exit
  -h, --help          this text

Environment:
  FLASHTEX_GATE_BASE  same as --base
  FLASHTEX_GATE_JOBS  same as --jobs
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    quick|pr|full) TIER="$1"; shift ;;
    --base) BASE="${2:?--base needs a ref}"; shift 2 ;;
    --committed-only) COMMITTED_ONLY=1; shift ;;
    --jobs) JOBS="${2:?--jobs needs a number}"; shift 2 ;;
    --list) LIST_ONLY=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "gate.sh: unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ -z "$TIER" ]]; then
  echo "gate.sh: name a tier: quick | pr | full" >&2
  usage >&2
  exit 2
fi

# ---------------------------------------------------------------------------
# Step bookkeeping
# ---------------------------------------------------------------------------
STEP_NAMES=()
STEP_RESULTS=()
STEP_SECONDS=()
STEP_NOTES=()
FAILED=0
GATE_T0=$SECONDS

bold() { printf '\033[1m%s\033[0m\n' "$*"; }
note() { printf '  %s\n' "$*"; }

record() { # record <name> <result> <seconds> [note]
  STEP_NAMES+=("$1"); STEP_RESULTS+=("$2"); STEP_SECONDS+=("$3"); STEP_NOTES+=("${4:-}")
  [[ "$2" == FAIL ]] && FAILED=1
  return 0
}

# step <name> -- <command...>
# Runs the command, times it, and records PASS/FAIL. Output is not swallowed:
# a local gate that hides the compiler error it found is useless.
step() {
  local name="$1"; shift
  [[ "${1:-}" == "--" ]] && shift
  if (( LIST_ONLY )); then record "$name" PLAN 0 "$*"; return 0; fi
  bold "==> $name"
  local t0=$SECONDS rc=0
  set +e
  ( "$@" )
  rc=$?
  set -e
  local el=$(( SECONDS - t0 ))
  if (( rc == 0 )); then
    record "$name" PASS "$el"
    printf '    ok (%ds)\n' "$el"
  else
    record "$name" FAIL "$el" "exit $rc"
    printf '    FAILED (exit %d, %ds)\n' "$rc" "$el"
  fi
  return 0
}

skip() { # skip <name> <why>
  if (( LIST_ONLY )); then record "$1" PLAN 0 "$2"; return 0; fi
  bold "==> $1"; note "skipped: $2"; record "$1" SKIP 0 "$2"
}

have() { command -v "$1" >/dev/null 2>&1; }

# ---------------------------------------------------------------------------
# Shared environment: the same variables ci.yml gives the Rust jobs, so a test
# that needs the vendored Latin Modern / EC / AMS metrics runs here exactly as
# it runs in CI instead of XCTSkip-style self-skipping.
# ---------------------------------------------------------------------------
export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"
export FLASHTEX_FONT_DIRS="${FLASHTEX_FONT_DIRS:-$ROOT/apps/mac/Fonts}"
export FLASHTEX_TFM_DIRS="${FLASHTEX_TFM_DIRS:-$ROOT/apps/mac/Fonts/texmf/fonts/tfm/public/lm:$ROOT/apps/mac/Fonts/texmf/fonts/tfm/jknappen/ec:$ROOT/apps/mac/Fonts/texmf/fonts/tfm/public/amsfonts/symbols}"
export FLASHTEX_LM_DIR="${FLASHTEX_LM_DIR:-$ROOT/apps/mac/Fonts}"

# Crates whose tests are temporarily not gating. One source of truth for
# ci.yml and this script: scripts/rust-test-exclude.txt.
read_list() { # read_list <file> -> one entry per line, comments stripped
  [[ -f "$1" ]] || return 0
  sed -e 's/#.*//' "$1" | tr '[:space:]' '\n' | grep -v '^$' || true
}
TEST_EXCLUDE="$(read_list scripts/rust-test-exclude.txt | tr '\n' ' ')"
CLIPPY_DEBT="$(read_list scripts/clippy-debt.txt | tr '\n' ' ')"

in_list() { # in_list <needle> <space separated haystack>
  local n="$1" h=" $2 "
  [[ "$h" == *" $n "* ]]
}

# ---------------------------------------------------------------------------
# What changed
# ---------------------------------------------------------------------------
BASE_OK=1
if ! git rev-parse --verify --quiet "$BASE" >/dev/null; then
  BASE_OK=0
fi

CHANGED_FILES=""
if (( BASE_OK )); then
  # `A...B` is the merge-base diff: what this branch added, not what main added.
  CHANGED_FILES="$(git diff --name-only "$BASE...HEAD")"
  if (( ! COMMITTED_ONLY )); then
    CHANGED_FILES="$CHANGED_FILES
$(git diff --name-only HEAD)
$(git ls-files --others --exclude-standard)"
  fi
  CHANGED_FILES="$(printf '%s\n' "$CHANGED_FILES" | grep -v '^$' | sort -u || true)"
fi

# Map changed paths onto cargo packages.
#   crates/<dir>/...        -> that crate's [package] name
#   Cargo.toml / Cargo.lock -> ROOT_MANIFEST=1 below, NOT a per-crate fan-out
# A crate is standalone (its own workspace) iff it has its own Cargo.lock. Since
# lane P0-RETIRE-VENDOR deleted crates/render-pipeline/vendor/ and folded the
# three standalone lockfiles into the root one, NO crate is standalone any more;
# the rule stays because it keeps this correct without a list to maintain.
# Output: one "<package>\t<workspace dir>\t<edition>" line per crate.
# Takes the changed-path list as its first argument (a file). NOT on stdin:
# `python3 -` reads its program from stdin, so the heredoc below already owns it,
# and a silent empty read here would scope every gate to zero crates.
crate_table() { # crate_table <file of changed paths>
  python3 - "$ROOT" "$1" <<'PY'
import os, re, sys
root = sys.argv[1]
with open(sys.argv[2], encoding="utf-8") as fh:
    changed = [l.strip() for l in fh if l.strip()]

def crate_info(d):
    m = os.path.join(root, "crates", d, "Cargo.toml")
    if not os.path.exists(m):
        return None
    txt = open(m, encoding="utf-8", errors="replace").read()
    pkg = re.search(r'^\[package\]', txt, re.M)
    if not pkg:
        return None
    tail = txt[pkg.end():]
    tail = tail.split("\n[", 1)[0]
    name = re.search(r'^\s*name\s*=\s*"([^"]+)"', tail, re.M)
    ed = re.search(r'^\s*edition\s*=\s*"([^"]+)"', tail, re.M)
    if not name:
        return None
    standalone = os.path.exists(os.path.join(root, "crates", d, "Cargo.lock"))
    ws = os.path.join("crates", d) if standalone else "."
    return (name.group(1), ws, ed.group(1) if ed else "2015")

dirs = set()
for p in changed:
    parts = p.split("/")
    if parts[0] == "crates" and len(parts) > 1:
        dirs.add(parts[1])

out = []
for d in sorted(dirs):
    info = crate_info(d)
    if info:
        out.append(info)
for name, ws, ed in out:
    print(f"{name}\t{ws}\t{ed}")
PY
}

CRATE_TABLE=""
if (( BASE_OK )); then
  CHANGED_LIST="$(mktemp "${TMPDIR:-/tmp}/flashtex-gate-changed-XXXXXX")"
  trap 'rm -f "$CHANGED_LIST"' EXIT
  printf '%s\n' "$CHANGED_FILES" > "$CHANGED_LIST"
  CRATE_TABLE="$(crate_table "$CHANGED_LIST")"
fi
CHANGED_CRATES="$(printf '%s\n' "$CRATE_TABLE" | awk 'NF{print $1}' | sort -u | tr '\n' ' ')"

# A change to the root Cargo.toml or Cargo.lock can break any root-workspace
# member, but it must not fan out into per-crate clippy + tests for all ~35 of
# them: that took 41-45+ min on a hosted runner and timed out the 45-min
# `quick` job (PR #1183, run 36546401263). Instead the whole workspace is
# checked, then clippied, in one cargo invocation each (one shared build);
# tests run only for crates whose own files changed; and the workspace's tests
# are the merge queue's job (`rust workspace` in ci.yml, on merge_group and on
# push to main). The standalone crates have their own lockfiles: unaffected.
ROOT_MANIFEST=0
if printf '%s\n' "$CHANGED_FILES" | grep -qxE 'Cargo\.(toml|lock)'; then
  ROOT_MANIFEST=1
fi
CHANGED_RS="$(printf '%s\n' "$CHANGED_FILES" | grep -E '\.rs$' || true)"

# ---------------------------------------------------------------------------
# quick: fmt, clippy on changed crates, tests of changed crates
# ---------------------------------------------------------------------------

# rustfmt, scoped to the .rs files this branch changed.
#
# A repo-wide `cargo fmt --all --check` cannot be a gate yet: at 7a1ed08aa it
# reports 288 unformatted files across 22 crates (measured 2026-09-29), and
# reformatting them belongs to its own lane, not to every PR. So this gate is a
# REGRESSION gate. A changed file fails only if it is unformatted now AND was
# formatted (or did not exist) at the base -- i.e. only if this branch made it
# worse. Pre-existing debt is reported as a warning with the fix command.
#
# The check is `rustfmt --edition <crate edition> --emit stdout` over stdin,
# compared with the file. Verified against `cargo fmt --all --check` on a
# 60-file sample: identical verdicts for every root-workspace file (the only
# divergences were render-pipeline files, which `cargo fmt --all` never visits
# because that crate is outside the root workspace).
gate_fmt() {
  local dirty_new=() dirty_old=() f ed crate rc=0
  local list
  list="$(printf '%s\n' "$CHANGED_RS" | grep -v '^$' || true)"
  if [[ -z "$list" ]]; then
    echo "no .rs files changed against $BASE"
    return 0
  fi
  while IFS= read -r f; do
    [[ -f "$f" ]] || continue
    ed=2021
    crate="$(printf '%s\n' "$f" | awk -F/ '$1=="crates" {print $2}')"
    if [[ -n "$crate" && -f "crates/$crate/Cargo.toml" ]]; then
      ed="$(sed -n '/^\[package\]/,/^\[/p' "crates/$crate/Cargo.toml" \
            | sed -n 's/^edition *= *"\([0-9]*\)".*/\1/p' | head -1)"
      [[ -n "$ed" ]] || ed=2021
    fi
    # Both sides only READ $f; SC2094 is about read-and-write in one pipeline.
    # shellcheck disable=SC2094
    if rustfmt --edition "$ed" --emit stdout --quiet < "$f" 2>/dev/null | diff -q - "$f" >/dev/null 2>&1; then
      continue
    fi
    # Unformatted now. Was it unformatted at the base too?
    if git cat-file -e "$BASE:$f" 2>/dev/null &&
       ! git show "$BASE:$f" | rustfmt --edition "$ed" --emit stdout --quiet 2>/dev/null \
         | diff -q - <(git show "$BASE:$f") >/dev/null 2>&1; then
      dirty_old+=("$f")
    else
      dirty_new+=("$f")
    fi
  done <<< "$list"

  if (( ${#dirty_old[@]} )); then
    echo "pre-existing rustfmt debt in files you touched (not gating):"
    printf '  %s\n' "${dirty_old[@]}"
  fi
  if (( ${#dirty_new[@]} )); then
    echo "rustfmt: these are unformatted and were clean at $BASE:"
    printf '  %s\n' "${dirty_new[@]}"
    echo "fix: rustfmt --edition <crate edition> ${dirty_new[0]}"
    rc=1
  fi
  if (( ${#dirty_old[@]} + ${#dirty_new[@]} == 0 )); then
    echo "all $(printf '%s\n' "$list" | grep -c .) changed .rs files are formatted"
  fi
  return $rc
}

# Root Cargo.toml/Cargo.lock changed (ROOT_MANIFEST): `cargo check --workspace
# --all-targets`, then clippy over the workspace in ONE invocation; the two
# share one dependency build. Root-workspace crates in scripts/clippy-debt.txt
# are --exclude'd from the clippy run (they are not gating anyway, and
# --no-deps keeps every other crate answerable only for itself); the check
# step still compiles every target of theirs.
gate_workspace_check() {
  cargo check --workspace --all-targets --locked
}

gate_workspace_clippy() {
  local excludes=() d name
  for d in crates/*/; do
    d="${d%/}"
    [[ -f "$d/Cargo.toml" && ! -f "$d/Cargo.lock" ]] || continue  # root members only
    name="$(sed -n '/^\[package\]/,/^\[/p' "$d/Cargo.toml" \
            | sed -n 's/^name *= *"\([^"]*\)".*/\1/p' | head -1)"
    [[ -n "$name" ]] && in_list "$name" "$CLIPPY_DEBT" && excludes+=(--exclude "$name")
  done
  echo "not gating (scripts/clippy-debt.txt), excluded: ${excludes[*]:-none}"
  cargo clippy --workspace --all-targets --no-deps --locked \
    ${excludes[@]+"${excludes[@]}"} -- -D warnings
}

# `cargo clippy -p <crate> --all-targets --no-deps -- -D warnings`.
#
# --no-deps matters: without it clippy lints every path dependency too, so one
# crate's debt fails every crate downstream of it. With it, a crate is
# answerable only for itself.
#
# scripts/clippy-debt.txt lists the crates that do not pass today (measured
# per-crate at 7a1ed08aa: 18 of what are now 38 workspace members -- 15 plus the
# three that were standalone until vendor/ was retired). Those run non-gating and
# warn once they pass, exactly like scripts/rust-test-exclude.txt. The list may
# only shrink.
gate_clippy() {
  if [[ -z "${CHANGED_CRATES// /}" ]]; then
    echo "no crates changed against $BASE"
    return 0
  fi
  local rc=0 pkg ws ok el t0
  while IFS=$'\t' read -r pkg ws _ed; do
    [[ -n "$pkg" ]] || continue
    if (( ROOT_MANIFEST )) && [[ "$ws" == "." ]] && ! in_list "$pkg" "$CLIPPY_DEBT"; then
      echo "  ok    clippy $pkg: covered by the workspace clippy step"
      continue
    fi
    t0=$SECONDS; ok=0
    if [[ "$ws" == "." ]]; then
      cargo clippy -p "$pkg" --all-targets --no-deps --locked -- -D warnings && ok=1 || ok=0
    else
      ( cd "$ws" && cargo clippy --all-targets --no-deps --locked -- -D warnings ) && ok=1 || ok=0
    fi
    el=$(( SECONDS - t0 ))
    if in_list "$pkg" "$CLIPPY_DEBT"; then
      if (( ok )); then
        echo "  NOTE  clippy now passes for $pkg (${el}s): delete its line from scripts/clippy-debt.txt"
        echo "::warning::clippy now passes for $pkg; delete its line from scripts/clippy-debt.txt"
      else
        echo "  WARN  clippy debt in $pkg (${el}s), listed in scripts/clippy-debt.txt: not gating"
      fi
    elif (( ok )); then
      echo "  ok    clippy $pkg (${el}s)"
    else
      echo "  FAIL  clippy $pkg (${el}s): -D warnings"
      rc=1
    fi
  done <<< "$CRATE_TABLE"
  return $rc
}

gate_tests_touched() {
  if [[ -z "${CHANGED_CRATES// /}" ]]; then
    echo "no crates changed against $BASE"
    return 0
  fi
  local rc=0 pkg ws ok el t0 what
  while IFS=$'\t' read -r pkg ws _ed; do
    [[ -n "$pkg" ]] || continue
    t0=$SECONDS; ok=0
    if in_list "$pkg" "$TEST_EXCLUDE"; then
      # Same rule as ci.yml: an excluded crate still has to BUILD; only its
      # tests are not gating (scripts/rust-test-exclude.txt names the issue).
      what="build $pkg (tests excluded)"
      if [[ "$ws" == "." ]]; then
        cargo build -p "$pkg" --all-targets --locked && ok=1 || ok=0
      else
        ( cd "$ws" && cargo build --all-targets --locked ) && ok=1 || ok=0
      fi
    else
      what="test $pkg"
      if [[ "$ws" == "." ]]; then
        cargo test -p "$pkg" --locked --no-fail-fast && ok=1 || ok=0
      else
        ( cd "$ws" && cargo test --locked --no-fail-fast ) && ok=1 || ok=0
      fi
    fi
    el=$(( SECONDS - t0 ))
    if (( ok )); then
      echo "  ok    $what (${el}s)"
    else
      echo "  FAIL  $what (${el}s)"
      rc=1
    fi
  done <<< "$CRATE_TABLE"
  return $rc
}

# ---------------------------------------------------------------------------
# pr: + parity fixtures, + the licence boundary
# ---------------------------------------------------------------------------

# The fixtures tier of the parity scoreboard (tools/parity/README.md): the
# committed fixtures against their committed pdflatex references, through the
# CLI just built. No TeX needed, ~3 s, and no document may drop below the level
# recorded in tools/parity/baseline-fixtures.json.
gate_parity_fixtures() {
  cargo build --release --locked --manifest-path crates/flashtex-cli/Cargo.toml --bin flashtex
  local work out
  work="${TMPDIR:-/tmp}/flashtex-gate-parity-work"
  out="${TMPDIR:-/tmp}/flashtex-gate-parity"
  rm -rf "$work" "$out"
  # ${a[@]+"${a[@]}"}: expanding an EMPTY array under `set -u` is an error on
  # bash 3.2, which is the /bin/bash on macOS runners.
  local jflag=()
  [[ -n "$JOBS" ]] && jflag=(-j "$JOBS")
  python3 tools/parity/parity.py --tier fixtures --raster none ${jflag[@]+"${jflag[@]}"} \
    --out "$out" --work "$work" \
    --check-baseline tools/parity/baseline-fixtures.json
}

gate_parity_selftest() {
  python3 -m unittest discover -s tools/parity -p 'test_*.py'
}

# ---------------------------------------------------------------------------
# full: the whole workspace in both profiles, the standalone crates, the Mac app
# ---------------------------------------------------------------------------
workspace_profile() { # workspace_profile <debug|release>
  local profile="$1" flag=()
  [[ "$profile" == release ]] && flag=(--release)
  cargo build --workspace --all-targets --locked "${flag[@]}"
  local excludes=()
  local p
  for p in $TEST_EXCLUDE; do excludes+=(--exclude "$p"); done
  cargo test --workspace --locked --no-fail-fast "${flag[@]}" ${excludes[@]+"${excludes[@]}"}
}

# render-pipeline and flashtex-cli, built and tested one package at a time in
# their own directories. They are ordinary workspace members since vendor/ was
# retired -- so this uses the root Cargo.lock and target/, and workspace_profile
# already covers them -- but a per-crate failure reads far more clearly than one
# inside the 38-crate run, and ci.yml's `rust-standalone` does exactly this.
standalone_profile() { # standalone_profile <debug|release>
  local profile="$1" flag=() c rc=0
  [[ "$profile" == release ]] && flag=(--release)
  for c in render-pipeline flashtex-cli; do
    [[ -f "crates/$c/Cargo.toml" ]] || continue
    ( cd "crates/$c" && cargo build --locked "${flag[@]}" && cargo test --locked --no-fail-fast "${flag[@]}" ) || rc=1
  done
  return $rc
}

gate_mac_app() {
  local helpers
  helpers="${TMPDIR:-/tmp}/flashtex-gate-helpers.env"
  scripts/ci/build-helpers.sh > "$helpers"
  set -a; # shellcheck disable=SC1090
  source "$helpers"; set +a
  CI=1 FLASHTEX_NO_ACTIVATE=1 FLASHTEX_KEYCHAIN_OFF=1 FLASHTEX_REVIEW_HISTORY_DIR=off \
    bash -c 'cd apps/mac && swift build && swift test'
}

# ---------------------------------------------------------------------------
# Run the tier
# ---------------------------------------------------------------------------
bold "FlashTeX gate: $TIER"
note "repository: $ROOT"
note "base:       $BASE $( (( BASE_OK )) || echo '(MISSING -- see below)')"
if (( BASE_OK )); then
  note "changed:    $(printf '%s\n' "$CHANGED_FILES" | grep -c . || true) files"
  note "crates:     ${CHANGED_CRATES:-(none)}"
  if (( ROOT_MANIFEST )); then
    note "root:       Cargo.toml/Cargo.lock changed -> workspace check + clippy;"
    note "            the workspace's tests run in the merge queue (rust workspace)"
  fi
fi
echo

if (( ! BASE_OK )); then
  echo "gate.sh: '$BASE' does not exist in this checkout, so nothing can be scoped" >&2
  echo "         to your change. Run: git fetch origin main   (or pass --base <ref>)" >&2
  exit 2
fi

# A stale build directory for a deleted crate makes every `cargo` command in the
# repository fail with "failed to load manifest for workspace member", because
# the root workspace globs crates/*. Say so once, up front, instead of letting
# each step die with the same opaque error. (crates/assistant-context was
# removed in 39118b9f0; a local target/ can outlive it.)
STRAY=()
for d in crates/*/; do
  [[ -f "$d/Cargo.toml" ]] && continue
  [[ -f "$d/Cargo.lock" ]] && continue
  STRAY+=("${d%/}")
done
if (( ${#STRAY[@]} )); then
  echo "gate.sh: these directories are inside the root workspace glob (crates/*) but" >&2
  echo "         have no Cargo.toml, so every cargo command here will fail:" >&2
  printf '           %s\n' "${STRAY[@]}" >&2
  echo "         They are leftover build output for crates that were deleted. Remove them:" >&2
  printf '           rm -rf %s\n' "${STRAY[@]}" >&2
  (( LIST_ONLY )) || exit 2
fi

case "$TIER" in
  quick|pr|full)
    if have rustfmt; then step "rustfmt (changed files)" -- gate_fmt
    else skip "rustfmt (changed files)" "rustfmt is not installed (rustup component add rustfmt)"; fi
    if (( ROOT_MANIFEST )); then
      step "workspace check (root manifest changed)" -- gate_workspace_check
      if cargo clippy --version >/dev/null 2>&1; then
        step "clippy (workspace, root manifest changed)" -- gate_workspace_clippy
      else
        skip "clippy (workspace, root manifest changed)" "clippy is not installed (rustup component add clippy)"
      fi
    fi
    if cargo clippy --version >/dev/null 2>&1; then step "clippy (changed crates)" -- gate_clippy
    else skip "clippy (changed crates)" "clippy is not installed (rustup component add clippy)"; fi
    step "tests (changed crates)" -- gate_tests_touched
    ;;
esac

case "$TIER" in
  pr|full)
    if [[ -x scripts/check-license-boundary.sh ]]; then
      step "licence boundary (DESIGN §3)" -- scripts/check-license-boundary.sh
    else
      skip "licence boundary (DESIGN §3)" "scripts/check-license-boundary.sh is not in this checkout"
    fi
    step "parity scoreboard self-tests" -- gate_parity_selftest
    if [[ "$(uname -s)" == Darwin ]]; then
      step "parity fixtures hold their baseline" -- gate_parity_fixtures
    else
      skip "parity fixtures hold their baseline" "baseline-fixtures.json was recorded on macOS (tools/parity/README.md); it is not comparable here"
    fi
    step "bundled inventory matches the compiler" -- apps/mac/scripts/sync-supported-latex.sh --check
    ;;
esac

case "$TIER" in
  full)
    step "generated tables match the manifest" -- python3 scripts/check-generated.py
    step "workspace, debug profile" -- workspace_profile debug
    step "workspace, release profile" -- workspace_profile release
    step "standalone crates, debug profile" -- standalone_profile debug
    step "standalone crates, release profile" -- standalone_profile release
    if [[ "$(uname -s)" == Darwin ]] && have swift; then
      step "mac app (swift build + swift test)" -- gate_mac_app
    else
      skip "mac app (swift build + swift test)" "needs macOS and a Swift toolchain"
    fi
    ;;
esac

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo
bold "gate.sh $TIER -- summary"
for i in "${!STEP_NAMES[@]}"; do
  printf '  %-6s %5ss  %s%s\n' "${STEP_RESULTS[$i]}" "${STEP_SECONDS[$i]}" \
    "${STEP_NAMES[$i]}" "$([[ -n "${STEP_NOTES[$i]}" ]] && printf '  [%s]' "${STEP_NOTES[$i]}")"
done
printf '  total %5ss\n' "$(( SECONDS - GATE_T0 ))"

if (( LIST_ONLY )); then exit 0; fi
if (( FAILED )); then
  bold "gate.sh $TIER: FAILED"
  exit 1
fi
bold "gate.sh $TIER: passed"
