#!/usr/bin/env bash
# Tests for scripts/ci/p5-pick-t4.sh with a fake `gh` (no network): the P5
# board's T4 pick accepts only this repository's own schedule/dispatch run of
# the route's workflow on main, at its own commit or a commit of main.
#   scripts/tests/p5-pick-t4.test.sh
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PICK="$ROOT/scripts/ci/p5-pick-t4.sh"
command -v jq >/dev/null || { echo "SKIP: no jq"; exit 0; }
T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT
mkdir -p "$T/bin"
# fake gh: answers from $FAKE (artifacts.json, runs/ID.json, summaries/ID.json, compare/SHA)
cat >"$T/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "$1 $2" in
  "api repos/o/r") echo '{"id": 7}' ;;
  "api repos/o/r/actions/artifacts?"*) cat "$FAKE/artifacts.json" ;;
  "api repos/o/r/actions/runs/"*) cat "$FAKE/runs/${2##*/}.json" ;;
  "api repos/o/r/compare/main..."*) s="${2##*...}"; printf '{"status": "%s"}\n' "$(cat "$FAKE/compare/$s" 2>/dev/null || echo diverged)" ;;
  "run download")
    id="$3"; dir=""; while [[ $# -gt 0 ]]; do [[ $1 == -D ]] && dir="$2"; shift; done
    [[ -f "$FAKE/summaries/$id.json" ]] || exit 1
    mkdir -p "$dir"; cp "$FAKE/summaries/$id.json" "$dir/summary.json" ;;
  *) echo "fake gh: unexpected: $*" >&2; exit 9 ;;
esac
EOF
chmod +x "$T/bin/gh"
export PATH="$T/bin:$PATH" GITHUB_REPOSITORY=o/r PICK_NOW=1759500000  # 2025-10-03T14:00:00Z

A=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa B=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
C=cccccccccccccccccccccccccccccccccccccccc
fail=0
setup() {  # setup NAME: an empty fake world
  export FAKE="$T/$1"; mkdir -p "$FAKE/runs" "$FAKE/summaries" "$FAKE/compare"
  echo '{"artifacts": []}' >"$FAKE/artifacts.json"
}
artifact() {  # artifact ID CREATED HEAD_REPO_ID NAME
  jq --argjson id "$1" --arg c "$2" --argjson hr "$3" --arg n "$4" \
    '.artifacts += [{"name": $n, "expired": false, "created_at": $c,
      "workflow_run": {"id": $id, "repository_id": 7, "head_repository_id": $hr, "head_branch": "main"}}]' \
    "$FAKE/artifacts.json" >"$FAKE/a.tmp" && mv "$FAKE/a.tmp" "$FAKE/artifacts.json"
}
run() {  # run ID PATH EVENT HEAD_SHA [HEAD_REPO_ID]
  printf '{"id": %s, "path": "%s", "event": "%s", "head_sha": "%s", "head_branch": "main", "repository": {"id": 7}, "head_repository": {"id": %s}}\n' \
    "$1" "$2" "$3" "$4" "${5:-7}" >"$FAKE/runs/$1.json"
}
summary() { printf '{"git_sha": "%s", "shards": {"done": 50, "total": 50}}\n' "$2" >"$FAKE/summaries/$1.json"; }
expect() {  # expect LABEL ROUTE WANT ("id=N sha=S" or "none")
  local out="$FAKE/out" got
  : >"$out"
  bash "$PICK" "$2" "$out" >"$FAKE/log" 2>&1 || { echo "FAIL $1: exit $?"; cat "$FAKE/log"; fail=1; return; }
  got="$(tr '\n' ' ' <"$out" | sed 's/ $//')"; [[ -n $got ]] || got=none
  if [[ "$got" == "$3" ]]; then echo "ok   $1"; else echo "FAIL $1: got '$got', want '$3'"; cat "$FAKE/log"; fail=1; fi
}

setup good
artifact 11 2025-10-03T06:00:00Z 7 corpus-t4; run 11 .github/workflows/nightly.yml schedule "$A"; summary 11 "$A"
expect "pc: own scheduled nightly run" pc "id=11 sha=$A"

setup fork
artifact 21 2025-10-03T06:00:00Z 7 corpus-t4; run 21 .github/workflows/nightly.yml schedule "$A"; summary 21 "$A"
artifact 22 2025-10-03T12:00:00Z 99 corpus-t4; run 22 .github/workflows/nightly.yml pull_request "$B" 99; summary 22 "$B"
expect "a fork's newer artifact is never a candidate" pc "id=21 sha=$A"

setup forkrun
artifact 31 2025-10-03T12:00:00Z 7 corpus-t4; run 31 .github/workflows/nightly.yml schedule "$B" 99; summary 31 "$B"
expect "a run whose head is a fork is skipped" pc none

setup event
artifact 41 2025-10-03T12:00:00Z 7 corpus-t4; run 41 .github/workflows/nightly.yml pull_request "$B"; summary 41 "$B"
artifact 42 2025-10-03T06:00:00Z 7 corpus-t4; run 42 .github/workflows/nightly.yml workflow_dispatch "$A"; summary 42 "$A"
expect "a pull_request run is skipped, the dispatch before it is used" pc "id=42 sha=$A"

setup path
artifact 51 2025-10-03T12:00:00Z 7 corpus-t4; run 51 .github/workflows/evil.yml workflow_dispatch "$B"; summary 51 "$B"
expect "another workflow's artifact of the same name is skipped" pc none
setup pathmac
artifact 52 2025-10-03T12:00:00Z 7 corpus-t4-mac; run 52 .github/workflows/nightly.yml workflow_dispatch "$B"; summary 52 "$B"
expect "mac: only corpus-t4-mac.yml's artifact" mac none

setup pinned
artifact 61 2025-10-02T12:00:00Z 7 corpus-t4-mac; run 61 .github/workflows/corpus-t4-mac.yml workflow_dispatch "$A"; summary 61 "$C"
echo behind >"$FAKE/compare/$C"
expect "mac: a pinned commit of main is accepted" mac "id=61 sha=$C"

setup foreign
artifact 71 2025-10-02T12:00:00Z 7 corpus-t4-mac; run 71 .github/workflows/corpus-t4-mac.yml workflow_dispatch "$A"; summary 71 "$C"
echo diverged >"$FAKE/compare/$C"
expect "mac: a commit outside main is refused" mac none

setup stale
artifact 81 2025-10-01T12:00:00Z 7 corpus-t4; run 81 .github/workflows/nightly.yml schedule "$A"; summary 81 "$A"
expect "pc: older than 36 h" pc none
setup stalemac
artifact 82 2025-09-25T12:00:00Z 7 corpus-t4-mac; run 82 .github/workflows/corpus-t4-mac.yml workflow_dispatch "$A"; summary 82 "$A"
expect "mac: older than 7 days" mac none

setup nosha
artifact 91 2025-10-03T12:00:00Z 7 corpus-t4; run 91 .github/workflows/nightly.yml schedule "$A"
printf '{"shards": {"done": 1, "total": 50}}\n' >"$FAKE/summaries/91.json"
expect "a summary without a commit is skipped" pc none

setup empty
expect "no artifact at all" pc none

[[ $fail == 0 ]] && echo "p5-pick-t4: all passed"
exit $fail
