#!/usr/bin/env bash
# Decides whether a Linux CLI release build is allowed to ship.
#
# The Linux release job builds each CLI crate independently so that one crate
# failing to build does not lose the crates that did build. That is deliberate,
# but it used to be *silent*: the job asserted only that at least one crate had
# built, so a regression breaking `compiler` or `pdf` left the job green and
# shipped a tarball whose README still advertised the missing tools, while
# `publish` never looked at the Linux job's result at all.
#
# This script replaces that "at least one" assertion. A crate that does not
# build fails the release unless the repository explicitly declares it as an
# allowed failure in `.github/linux-release-allowed-failures`. Declaring one is
# a reviewable commit, not an accident, and the declared-degraded case is loud:
# an annotation, a job-summary block, and a banner in the release notes.
#
# Usage:
#   linux-release-gate.sh --required "a b c" --built "a b" [--allow-file PATH]
#   linux-release-gate.sh --self-test
#
#   --required    every crate the release is expected to produce (space separated)
#   --built       the crates that actually built (space separated, may be empty)
#   --mandatory   crates a declaration may never excuse (space separated): a
#                 tarball without these is not a CLI release at all
#   --allow-file  declared allowed failures
#                 (default: <repo>/.github/linux-release-allowed-failures)
#
# Exit 0 = the release may ship (complete, or degraded exactly as declared).
# Exit 1 = a crate failed to build that nobody declared. The release stops.
#
# When $GITHUB_OUTPUT is set it writes `degraded`, `missing` and `allowed`; when
# $GITHUB_STEP_SUMMARY is set it appends a human-readable summary.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ALLOW_FILE="$REPO_ROOT/.github/linux-release-allowed-failures"
REQUIRED=""
BUILT=""
MANDATORY="flashtex-cli render-pipeline"
SELF_TEST=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --required) REQUIRED="${2?--required needs a list}"; shift 2 ;;
    --built) BUILT="${2-}"; shift 2 ;;
    --mandatory) MANDATORY="${2-}"; shift 2 ;;
    --allow-file) ALLOW_FILE="${2:?--allow-file needs a path}"; shift 2 ;;
    --self-test) SELF_TEST=1; shift ;;
    -h|--help) sed -n '2,31p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "linux-release-gate.sh: unknown argument: $1" >&2; exit 2 ;;
  esac
done

# Declared allowed failures: one crate name per line, `#` comments and blank
# lines ignored. A missing file means nothing is allowed to fail.
read_allowed() {
  [[ -f "$1" ]] || return 0
  sed -e 's/#.*//' -e 's/[[:space:]]//g' "$1" | grep -v '^$' || true
}

contains() { # contains <needle> <space-separated haystack>
  local n="$1" h=" $2 "
  [[ "$h" == *" $n "* ]]
}

gate() { # gate <required> <built> <allow-file> [mandatory]; echoes report, returns 0/1
  local required="$1" built="$2" allow_file="$3" mandatory="${4-}"
  local allowed missing=() undeclared=() declared=() stale=()
  allowed="$(read_allowed "$allow_file" | tr '\n' ' ')"

  local c
  for c in $required; do
    contains "$c" "$built" || missing+=("$c")
  done
  for c in ${missing[@]+"${missing[@]}"}; do
    # A mandatory crate can never be declared away: without `flashtex` or
    # `flashtex-render` the tarball is not a CLI release, however it is labelled.
    if contains "$c" "$mandatory"; then
      undeclared+=("$c")
    elif contains "$c" "$allowed"; then
      declared+=("$c")
    else
      undeclared+=("$c")
    fi
  done
  # A declared allowance for a crate that builds fine is stale: report it so the
  # file does not silently accumulate permanent permission to ship degraded.
  for c in $allowed; do
    contains "$c" "${missing[*]-}" || stale+=("$c")
  done

  GATE_MISSING="${missing[*]-}"
  GATE_DECLARED="${declared[*]-}"
  GATE_UNDECLARED="${undeclared[*]-}"
  GATE_STALE="${stale[*]-}"

  if [[ ${#undeclared[@]} -gt 0 ]]; then
    GATE_DEGRADED=false
    echo "linux-release-gate: FAIL — these crates did not build and are not declared in ${allow_file#"$REPO_ROOT/"}: ${undeclared[*]}" >&2
    echo "linux-release-gate: a release tarball advertising these tools must not ship. Fix the build, or declare the failure in that file with a reason." >&2
    return 1
  fi
  if [[ ${#missing[@]} -gt 0 ]]; then
    GATE_DEGRADED=true
    echo "linux-release-gate: DEGRADED — shipping without: ${missing[*]} (declared in ${allow_file#"$REPO_ROOT/"})" >&2
    return 0
  fi
  GATE_DEGRADED=false
  echo "linux-release-gate: complete — every required crate built: $required"
  return 0
}

self_test() {
  local tmp pass=0 fail=0
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  : > "$tmp/none"
  printf '# pdf needs a newer glibc on this runner\npdf\n' > "$tmp/pdf-allowed"
  printf '\n  # only comments\n\n' > "$tmp/comments-only"
  printf 'flashtex-cli\n' > "$tmp/cli-allowed"

  check() { # check <label> <rc> <degraded> <required> <built> <allow-file> [mandatory]
    local label="$1" want_rc="$2" want_deg="$3" rc=0
    GATE_DEGRADED=unset
    gate "$4" "$5" "$6" "${7-flashtex-cli render-pipeline}" >/dev/null 2>&1 || rc=$?
    if [[ "$rc" == "$want_rc" && "$GATE_DEGRADED" == "$want_deg" ]]; then
      pass=$((pass + 1)); echo "  ok   $label (rc=$rc degraded=$GATE_DEGRADED)"
    else
      fail=$((fail + 1))
      echo "  FAIL $label: want rc=$want_rc degraded=$want_deg, got rc=$rc degraded=$GATE_DEGRADED" >&2
    fi
  }

  echo "==> linux-release-gate.sh self-test"
  check "complete build passes" 0 false \
    "flashtex-cli render-pipeline compiler pdf" "flashtex-cli render-pipeline compiler pdf" "$tmp/none"
  # The bug this script exists to close: one crate missing used to be green.
  check "undeclared missing crate fails" 1 false \
    "flashtex-cli render-pipeline compiler pdf" "flashtex-cli render-pipeline compiler" "$tmp/none"
  check "the old 'at least one built' case fails" 1 false \
    "flashtex-cli render-pipeline compiler pdf" "render-pipeline" "$tmp/none"
  check "nothing built fails" 1 false \
    "flashtex-cli render-pipeline compiler pdf" "" "$tmp/none"
  check "declared missing crate passes as degraded" 0 true \
    "flashtex-cli render-pipeline compiler pdf" "flashtex-cli render-pipeline compiler" "$tmp/pdf-allowed"
  # A declaration for `pdf` must not excuse `compiler` going missing too.
  check "a declaration excuses only the crate it names" 1 false \
    "flashtex-cli render-pipeline compiler pdf" "flashtex-cli render-pipeline" "$tmp/pdf-allowed"
  check "comments and blanks are not crate names" 1 false \
    "flashtex-cli render-pipeline compiler pdf" "flashtex-cli render-pipeline compiler" "$tmp/comments-only"
  check "a missing allow-file allows nothing" 1 false \
    "flashtex-cli render-pipeline compiler pdf" "flashtex-cli render-pipeline compiler" "$tmp/does-not-exist"
  # The allow file must not be able to sign off an empty release.
  check "a mandatory crate cannot be declared away" 1 false \
    "flashtex-cli render-pipeline compiler pdf" "render-pipeline compiler pdf" "$tmp/cli-allowed"

  # A stale declaration is reported but does not fail a complete build.
  GATE_STALE=""
  gate "flashtex-cli pdf" "flashtex-cli pdf" "$tmp/pdf-allowed" "flashtex-cli" >/dev/null 2>&1
  if [[ "$GATE_STALE" == "pdf" ]]; then
    pass=$((pass + 1)); echo "  ok   stale declaration reported"
  else
    fail=$((fail + 1)); echo "  FAIL stale declaration reported: got '$GATE_STALE'" >&2
  fi

  echo "==> linux-release-gate self-test: $pass ok, $fail failed"
  [[ "$fail" -eq 0 ]]
}

if [[ "$SELF_TEST" -eq 1 ]]; then
  self_test
  exit $?
fi

[[ -n "$REQUIRED" ]] || { echo "linux-release-gate.sh: --required is mandatory" >&2; exit 2; }

rc=0
gate "$REQUIRED" "$BUILT" "$ALLOW_FILE" "$MANDATORY" || rc=$?

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  {
    echo "degraded=$GATE_DEGRADED"
    echo "missing=$GATE_MISSING"
    echo "allowed=$GATE_DECLARED"
  } >> "$GITHUB_OUTPUT"
fi

if [[ "$rc" -ne 0 ]]; then
  echo "::error title=Linux release build incomplete::did not build: $GATE_UNDECLARED (not declared in .github/linux-release-allowed-failures)"
elif [[ "$GATE_DEGRADED" == true ]]; then
  echo "::warning title=Degraded Linux release::shipping without $GATE_MISSING (declared allowed failures)"
fi
if [[ -n "$GATE_STALE" ]]; then
  echo "::warning title=Stale allowed-failure declaration::$GATE_STALE builds fine; remove it from .github/linux-release-allowed-failures"
fi

if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
  {
    echo "### Linux CLI release gate"
    echo
    echo "- required: \`$REQUIRED\`"
    echo "- built: \`${BUILT:-none}\`"
    if [[ "$rc" -ne 0 ]]; then
      echo "- **failed**: \`$GATE_UNDECLARED\` did not build and is not a declared allowed failure."
    elif [[ "$GATE_DEGRADED" == true ]]; then
      echo "- **degraded**: shipping without \`$GATE_MISSING\`, declared in \`.github/linux-release-allowed-failures\`."
    else
      echo "- complete: every required crate built."
    fi
    [[ -n "$GATE_STALE" ]] && echo "- stale declaration(s): \`$GATE_STALE\` build fine and should be removed from the allow file."
  } >> "$GITHUB_STEP_SUMMARY"
fi

exit "$rc"
