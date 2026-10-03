#!/usr/bin/env bash
# Open, update or close the `main-macos-red` issue for one workflow run.
#
# The macOS legs of the Rust workspace, the standalone crates, trip, etrip and
# pdftex-regression no longer run in the merge queue (docs/ci-cd.md, "Post-merge macOS legs").
# They run after the merge, on the push to main, and nightly, and are fixed
# forward: this script is how a failure there is seen. The old engine's parity
# fixtures are covered too: the merge queue runs them only when a change
# touches their paths (ci.yml `plan`, `old_engine`), a push to main always does.
#
#   one or more jobs whose name mentions macOS failed
#     -> the open issue labelled main-macos-red gets a comment naming the run,
#        the commit and the failed jobs; with no open issue, one is opened
#   every such job passed
#     -> an open main-macos-red issue gets a "green again" comment and is closed
#
# Usage: scripts/ci/main-macos-red.sh <run-id>
# Needs GH_TOKEN with issues:write and actions:read, and GITHUB_REPOSITORY.
set -euo pipefail

run="${1:?usage: main-macos-red.sh <run-id>}"
repo="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is not set}"
label=main-macos-red

jobs="$(gh api --paginate "repos/$repo/actions/runs/$run/jobs?per_page=100&filter=latest" \
          --jq '.jobs[] | select((.name | test("macos"; "i")) and (.name | startswith("main-macos-red") | not))
                 | [.conclusion // "pending", .name, .html_url] | @tsv')"
if [[ -z "$jobs" ]]; then
  echo "no macOS jobs in run $run; nothing to report"
  exit 0
fi
failed="$(awk -F'\t' '$1 == "failure" || $1 == "timed_out" { printf "- [%s](%s): %s\n", $2, $3, $1 }' <<<"$jobs")"
sha="$(gh api "repos/$repo/actions/runs/$run" --jq '.head_sha')"
event="$(gh api "repos/$repo/actions/runs/$run" --jq '.event + " (" + .name + ")"')"
url="https://github.com/$repo/actions/runs/$run"

gh label create "$label" --repo "$repo" --color B60205 \
  --description "a post-merge macOS leg failed on main; fix forward (docs/ci-cd.md)" >/dev/null 2>&1 || true
open="$(gh issue list --repo "$repo" --label "$label" --state open --json number --jq '.[0].number // empty')"

if [[ -n "$failed" ]]; then
  body="$(printf 'macOS legs failed at %s, %s: %s\n\n%s\n' "$sha" "$event" "$url" "$failed")"
  if [[ -n "$open" ]]; then
    gh issue comment "$open" --repo "$repo" --body "$body"
    echo "updated #$open"
  else
    gh issue create --repo "$repo" --label "$label" \
      --title "main is red on macOS (post-merge legs)" \
      --body "$(printf '%s\n\nThese legs left the merge queue (docs/ci-cd.md, "Post-merge macOS legs"), so main is fixed forward: land the fix through the queue, and this issue closes itself on the next green run.\n' "$body")"
  fi
  printf '%s\n' "$failed"
  exit 0
fi

if [[ -n "$open" ]] && ! awk -F'\t' '$1 != "success" && $1 != "skipped" { bad = 1 } END { exit !bad }' <<<"$jobs"; then
  gh issue comment "$open" --repo "$repo" --body "Green again at $sha, $event: $url"
  gh issue close "$open" --repo "$repo"
  echo "closed #$open"
fi
