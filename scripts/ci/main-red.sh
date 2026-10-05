#!/usr/bin/env bash
# Open, update or close the `main-red` issue for one workflow run on main.
#
# The merge queue runs only the gate (docs/ci-cd.md, "The merge-queue gate"):
# the Mac app, the iPad companion, the Rust workspace on Linux and macOS,
# render-pipeline, flashtex-cli, flashtex-xetex and every macOS leg run AFTER
# the merge, on the push to main (ci.yml), and nightly (nightly.yml). They are
# fixed forward, and this script is how a failure there is seen:
#
#   a job failed (or timed out)
#     -> the open issue labelled main-red gets a comment naming the run, the
#        commit, the failed jobs and the pull requests the push merged; with
#        no open issue, one is opened. Policy (docs/ci-cd.md, "main-red"):
#        the merged PR's owner fixes forward within 2 hours, or the Commander
#        reverts the merge.
#   every job passed, the run is ci.yml's on main's current tip, and
#   --no-close was not given
#     -> an open main-red issue (and a leftover main-macos-red one, the label
#        this replaced) gets a "green again" comment and is closed
#
# Usage: scripts/ci/main-red.sh <run-id> [--only REGEX] [--no-close]
#   --only REGEX  consider only the jobs whose name matches REGEX (awk ERE,
#                 case-insensitive); default: every job but this reporter
#                 and `CI required`
#   --no-close    report failures only; never close the issue (nightly.yml:
#                 a green night covers a few legs, not everything on main)
# Environment: GH_TOKEN (issues:write, actions:read, contents:read,
# pull-requests:read), GITHUB_REPOSITORY; BEFORE and AFTER, the push's range
# (github.event.before/after), to name the merged pull requests.
set -euo pipefail

run="${1:?usage: main-red.sh <run-id> [--only REGEX] [--no-close]}"
shift
only=''
close=1
while [[ $# -gt 0 ]]; do
  case "$1" in
    --only) only="${2:?--only needs a regex}"; shift 2 ;;
    --no-close) close=0; shift ;;
    *) echo "main-red.sh: unknown argument: $1" >&2; exit 2 ;;
  esac
done
repo="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is not set}"
label=main-red
legacy_label=main-macos-red

jobs="$(gh api --paginate "repos/$repo/actions/runs/$run/jobs?per_page=100&filter=latest" \
          --jq '.jobs[] | select(.name | test("^(main-red|main-macos-red|CI required)") | not)
                 | [.conclusion // "pending", .name, .html_url] | @tsv')"
if [[ -n "$only" ]]; then
  # The name is the second tab-separated field.
  jobs="$(awk -F'\t' -v re="$only" 'tolower($2) ~ tolower(re)' <<<"$jobs")"
fi
if [[ -z "$jobs" ]]; then
  echo "no jobs to report in run $run"
  exit 0
fi
failed="$(awk -F'\t' '$1 == "failure" || $1 == "timed_out" { printf "- [%s](%s): %s\n", $2, $3, $1 }' <<<"$jobs")"
sha="$(gh api "repos/$repo/actions/runs/$run" --jq '.head_sha')"
event="$(gh api "repos/$repo/actions/runs/$run" --jq '.event + " (" + .name + ")"')"
url="https://github.com/$repo/actions/runs/$run"

# The pull requests this push merged: the merge queue lands each one as a
# "Merge pull request #N from ..." commit, so they are the merge commits in
# BEFORE...AFTER. Without a range (nightly), none are named.
merged_prs() {
  [[ -n "${BEFORE:-}" && -n "${AFTER:-}" && ! "$BEFORE" =~ ^0+$ ]] || return 0
  local n
  gh api "repos/$repo/compare/$BEFORE...$AFTER" --jq '.commits[].commit.message | split("\n")[0]' 2>/dev/null |
    sed -nE 's/^Merge pull request #([0-9]+) .*/\1/p' | sort -un |
    while read -r n; do
      # shellcheck disable=SC2016 # a jq string, not a shell expansion
      gh pr view "$n" --repo "$repo" --json number,title,author,headRefName \
        --jq '"- #\(.number) \(.title) (@\(.author.login), `\(.headRefName)`)"' 2>/dev/null ||
        echo "- #$n"
    done
}

gh label create "$label" --repo "$repo" --color B60205 \
  --description "a post-merge job failed on main; the PR's owner fixes forward within 2 h or the Commander reverts (docs/ci-cd.md)" \
  >/dev/null 2>&1 || true
open="$(gh issue list --repo "$repo" --label "$label" --state open --json number --jq '.[0].number // empty')"

if [[ -n "$failed" ]]; then
  prs="$(merged_prs)"
  body="$(printf 'Failed at %s, %s: %s\n\n%s\n' "$sha" "$event" "$url" "$failed")"
  if [[ -n "$prs" ]]; then
    body="$(printf '%s\n\nMerged by this push (their owners fix forward):\n\n%s\n' "$body" "$prs")"
  fi
  if [[ -n "$open" ]]; then
    gh issue comment "$open" --repo "$repo" --body "$body"
    echo "updated #$open"
  else
    gh issue create --repo "$repo" --label "$label" \
      --title "main is red (post-merge CI)" \
      --body "$(printf '%s\n\nThese jobs run after the merge, not in the merge queue (docs/ci-cd.md, "main-red"). Policy: the owner of the pull request that broke them fixes forward within 2 hours (the fix lands through the queue like any change), or the Commander reverts the merge. This issue closes itself on the next fully green run on main.\n' "$body")"
  fi
  printf '%s\n' "$failed"
  exit 0
fi

[[ "$close" == 1 ]] || exit 0
# Every job passed or was skipped -- and nothing is still pending.
if awk -F'\t' '$1 != "success" && $1 != "skipped" { bad = 1 } END { exit !bad }' <<<"$jobs"; then
  exit 0
fi
# Only a run on main's CURRENT tip may declare it green: runs on main overlap
# (each has its own concurrency group), and an older run finishing late must
# not close an issue a newer commit opened.
tip="$(gh api "repos/$repo/commits/main" --jq '.sha')"
if [[ "$tip" != "$sha" ]]; then
  echo "run $run is at $sha, main is at $tip: leaving the issue to the newer run"
  exit 0
fi
legacy="$(gh issue list --repo "$repo" --label "$legacy_label" --state open --json number --jq '.[].number')"
for n in $open $legacy; do
  gh issue comment "$n" --repo "$repo" --body "Green again at $sha, $event: $url"
  gh issue close "$n" --repo "$repo"
  echo "closed #$n"
done
