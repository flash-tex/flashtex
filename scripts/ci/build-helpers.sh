#!/usr/bin/env bash
# Builds the `flashtex` CLI and every Rust helper that
# apps/mac/scripts/make-app.sh bundles, in
# release mode, one crate at a time (most are members of the root Cargo
# workspace and share ./target; render-pipeline and flashtex-cli are still
# standalone -- see Cargo.toml). Idempotent: cargo rebuilds only what changed. Prints one
# `FLASHTEX_<NAME>=<absolute path>` line per built binary on stdout, in the
# shell/`$GITHUB_ENV` format the Mac app and its tests read, so CI can do
#   scripts/ci/build-helpers.sh >> "$GITHUB_ENV"
# and a developer can do
#   set -a; source <(scripts/ci/build-helpers.sh); set +a
# Progress and cargo output go to stderr.
#
# Usage: scripts/ci/build-helpers.sh [--root <repo>] [--debug] [--only a,b,...]
#                                    [--skip a,b,...] [--check]
#   --root   repository checkout holding crates/ (default: this repository)
#   --debug  build with `cargo build` (no --release); paths point at target/debug
#   --only   build only these crate directory names (comma separated)
#   --skip   skip these crate directory names (comma separated)
#   --check  do not build; only print the env lines for binaries that exist
#            (exit 1 if a required one is missing)
#
# Crates that make-app.sh treats as optional (diagnostic-explanations) are
# skipped with a note when their directory does not exist; every other crate
# in the table must build or this script fails.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PROFILE="release"
CARGO_PROFILE_FLAG="--release"
ONLY=""
SKIP=""
CHECK=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --root) ROOT="$(cd "${2:?--root needs a path}" && pwd)"; shift 2 ;;
    --debug) PROFILE="debug"; CARGO_PROFILE_FLAG=""; shift ;;
    --only) ONLY="${2:?--only needs a list}"; shift 2 ;;
    --skip) SKIP="${2:?--skip needs a list}"; shift 2 ;;
    --check) CHECK=1; shift ;;
    -h|--help) sed -n '2,23p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "build-helpers.sh: unknown argument: $1" >&2; exit 2 ;;
  esac
done

# crate dir | extra cargo args | optional(1/0) | ENV_NAME=binary[,ENV_NAME=binary]
# Mirrors HELPER_TABLE in apps/mac/scripts/make-app.sh and the FLASHTEX_*
# variables the Mac app reads (apps/mac/Sources/FlashTeXMac/*).
HELPERS=(
  "flashtex-cli||0|FLASHTEX_CLI=flashtex"
  "compiler||0|FLASHTEX_COMPILER=flashtex-compiler"
  "pdf||0|FLASHTEX_PDF=flashtex-pdf,FLASHTEX_PDF_EXACT=flashtex-pdf-exact"
  "bridge||0|FLASHTEX_BRIDGE=flashtex-bridge"
  "edit-ledger||0|FLASHTEX_EDIT_LEDGER=flashtex-edit-ledger"
  "render-pipeline||0|FLASHTEX_RENDER=flashtex-render"
  "preview-controller||0|FLASHTEX_PREVIEW_CONTROLLER=flashtex-preview-controller"
  "project-files||0|FLASHTEX_PROJECT_FILES=flashtex-project-files"
  "diagnostic-explanations||1|FLASHTEX_EXPLAIN=flashtex-explain"
)

in_list() {  # $1=needle $2=comma list
  [[ -n "$2" ]] && [[ ",$2," == *",$1,"* ]]
}

log() { echo "==> $*" >&2; }

FAILED=()
for row in "${HELPERS[@]}"; do
  IFS='|' read -r crate extra optional bins <<< "$row"
  if [[ -n "$ONLY" ]] && ! in_list "$crate" "$ONLY"; then continue; fi
  if in_list "$crate" "$SKIP"; then log "skipping crates/$crate (--skip)"; continue; fi
  dir="$ROOT/crates/$crate"
  if [[ ! -f "$dir/Cargo.toml" ]]; then
    if [[ "$optional" == "1" ]]; then
      log "crates/$crate not present; skipping (optional helper)"
      continue
    fi
    echo "build-helpers.sh: required crate directory missing: $dir" >&2
    exit 1
  fi
  if [[ "$CHECK" -eq 0 ]]; then
    log "cargo build $CARGO_PROFILE_FLAG $extra  (crates/$crate)"
    # shellcheck disable=SC2086  # $extra is a deliberate word-split flag list
    if ! (cd "$dir" && cargo build $CARGO_PROFILE_FLAG $extra >&2); then
      echo "build-helpers.sh: cargo build failed for crates/$crate" >&2
      FAILED+=("$crate")
      continue
    fi
  fi
  target_dir="$("$SCRIPT_DIR/../crate-target-dir.sh" "$dir")" || {
    echo "build-helpers.sh: cannot resolve the target directory of crates/$crate" >&2
    FAILED+=("$crate"); continue
  }
  IFS=',' read -ra pairs <<< "$bins"
  for pair in "${pairs[@]}"; do
    env_name="${pair%%=*}"
    bin_name="${pair#*=}"
    path="$target_dir/$PROFILE/$bin_name"
    if [[ -x "$path" ]]; then
      echo "$env_name=$path"
    else
      echo "build-helpers.sh: expected binary not found after build: $path" >&2
      FAILED+=("$crate:$bin_name")
    fi
  done
done

if [[ ${#FAILED[@]} -gt 0 ]]; then
  echo "build-helpers.sh: failed: ${FAILED[*]}" >&2
  exit 1
fi
log "all helpers built ($PROFILE)"
