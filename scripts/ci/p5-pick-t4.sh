#!/usr/bin/env bash
# p5-pick-t4.sh ROUTE OUT: the T4 run the P5 board uses on ROUTE (pc or mac)
# (.github/workflows/p5-scoreboard.yml, job `pick`). Appends `id=RUN` and
# `sha=COMMIT` to OUT ($GITHUB_OUTPUT) when a run qualifies; appends nothing
# when none does, and the board's T4 rows then read missing. Messages, with
# GitHub workflow commands, go to stdout.
#
# The board checks out the chosen commit and builds it on a self-hosted
# runner, so the run must be one this repository made on main by schedule or
# by hand, never a pull request's (DESIGN §9.3). A candidate qualifies only
# when ALL of these hold; the newest of the 10 newest artifacts that does is
# used:
#   - the artifact is named for the route (pc: corpus-t4; mac: corpus-t4-mac),
#     not expired, uploaded within the route's window (pc 36 h, mac 7 days);
#   - its run is this repository's own (repository_id and head_repository_id
#     both this repository: a fork's pull_request run has the fork as head);
#   - the run's head_branch is main, its event schedule or workflow_dispatch,
#     and its workflow file exactly the route's (nightly.yml, corpus-t4-mac.yml);
#   - the summary's git_sha is a full SHA and a commit of main (GitHub's
#     compare main...SHA says behind or identical), checked even when it is the
#     run's own head_sha: a tag named main is not the branch, and a
#     corpus-t4-mac cycle is pinned to an older commit of main.
#
# Needs gh (authenticated, actions: read), jq and GITHUB_REPOSITORY.
# PICK_NOW (seconds since the epoch) fixes "now" for tests.
set -euo pipefail

route="${1:?usage: p5-pick-t4.sh pc|mac OUT}" out="${2:?usage: p5-pick-t4.sh pc|mac OUT}"
repo="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is not set}"
case "$route" in
  pc) name=corpus-t4 workflow=.github/workflows/nightly.yml hours=36 ;;
  mac) name=corpus-t4-mac workflow=.github/workflows/corpus-t4-mac.yml hours=168 ;;
  *) echo "::error::p5-pick-t4.sh: route $route: expected pc or mac"; exit 2 ;;
esac
now="${PICK_NOW:-$(date -u +%s)}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

repo_id="$(gh api "repos/$repo" | jq -r .id)"
[[ "$repo_id" =~ ^[0-9]+$ ]] || { echo "::error::p5-pick-t4.sh: no id for $repo"; exit 1; }

# "RUN_ID CREATED_AT" of the 10 newest candidates, newest first
gh api "repos/$repo/actions/artifacts?name=$name&per_page=50" >"$tmp/artifacts.json"
jq -r --argjson rid "$repo_id" --argjson now "$now" --argjson max $((hours * 3600)) '
  [.artifacts[]
   | select((.expired | not)
            and .workflow_run.head_branch == "main"
            and .workflow_run.repository_id == $rid
            and .workflow_run.head_repository_id == $rid
            and ($now - (.created_at | fromdateiso8601)) <= $max)]
  | sort_by(.created_at) | reverse | .[:10][]
  | "\(.workflow_run.id) \(.created_at)"' "$tmp/artifacts.json" >"$tmp/candidates"

while read -r id created; do
  [[ "$id" =~ ^[0-9]+$ ]] || continue
  if ! gh api "repos/$repo/actions/runs/$id" >"$tmp/run.json"; then
    echo "::warning::route $route: run $id of $name cannot be read; skipped"; continue
  fi
  why="$(jq -r --argjson rid "$repo_id" --arg wf "$workflow" '
    if .repository.id != $rid or .head_repository.id != $rid then "not this repository'"'"'s own run"
    elif .head_branch != "main" then "not a run of main (\(.head_branch))"
    elif (.event | IN("schedule", "workflow_dispatch") | not) then "event \(.event), not schedule or workflow_dispatch"
    elif .path != $wf then "workflow \(.path), not \($wf)"
    else "" end' "$tmp/run.json")"
  if [[ -n "$why" ]]; then
    echo "::warning::route $route: $name of run $id skipped: $why"; continue
  fi
  head="$(jq -r .head_sha "$tmp/run.json")"
  rm -rf "$tmp/t4"
  if ! gh run download "$id" --repo "$repo" -n "$name" -D "$tmp/t4" >/dev/null ||
     ! sha="$(jq -r '.git_sha // empty' "$tmp/t4/summary.json")" ||
     ! [[ "$sha" =~ ^[0-9a-f]{40}$ ]]; then
    echo "::warning::route $route: $name of run $id is unreadable or records no commit; skipped"; continue
  fi
  # Always a commit of main, even when it is the run's own head_sha: head_branch
  # "main" can also be a tag named main, whose commit is not on the branch.
  status="$(gh api "repos/$repo/compare/main...$sha" | jq -r .status)" || status="unreadable"
  case "$status" in
    behind|identical) ;;
    *) echo "::warning::route $route: $name of run $id measured $sha (run head $head), not a commit of main (compare: $status); skipped"
       continue ;;
  esac
  echo "route $route: $name of run $id ($workflow, uploaded $created) measured $sha;" \
       "$(jq -r '"\(.shards.done) of \(.shards.total) shards"' "$tmp/t4/summary.json")"
  { echo "id=$id"; echo "sha=$sha"; } >>"$out"
  exit 0
done <"$tmp/candidates"
echo "::notice::route $route: no qualifying $name artifact from this repository's own run of $workflow on main in the last $hours h: T4 rows read missing"
