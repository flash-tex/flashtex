#!/usr/bin/env bash
# Install GitHub's official Actions runner on one of the team's Apple Silicon
# Macs, with the rules a PUBLIC repository requires
# (docs/design/engine-v2/DESIGN.md §9.3). See docs/ci-cd.md, "Self-hosted Macs".
#
# WHY. On 2026-09-27 a CI run on main took 225-275 minutes, almost all of it
# macOS jobs queueing for GitHub-hosted runners. The team's own Macs are idle
# most of the day and are 3-5x faster at the Swift and Rust suites.
#
# WHAT IT DOES
#   1. Resolves an exact runner release (default: the newest) and downloads
#      actions-runner-osx-arm64-<version>.tar.gz from github.com/actions/runner.
#   2. Verifies its SHA-256 against the hash published in that release's own
#      notes. A mismatch aborts before anything is unpacked.
#   3. Registers it against this repository with the labels
#      self-hosted,macOS,ARM64,flashtex and `--disableupdate`, so the binary
#      stays the version whose hash was verified.
#   4. Gives every job a fresh working directory, by wiping the workspace and
#      the job temp directory in the runner's JOB_COMPLETED hook.
#   5. Keeps cargo's and rustup's caches OUTSIDE that working directory, so the
#      expensive part (the crates.io registry, the git checkouts, the toolchain)
#      survives while the source tree does not.
#   6. Installs it as a per-user launchd service with GitHub's own `svc.sh`,
#      which writes ~/Library/LaunchAgents/actions.runner.<repo>.<name>.plist.
#      No root, no system-wide daemon, no login shell left signed in.
#
# SECURITY, AND WHY THIS IS SAFE ON A PUBLIC REPOSITORY
#   * Fork pull requests never reach this runner. ci.yml guards every
#     `runs-on: [self-hosted, ...]` job with
#       github.event_name != 'pull_request' ||
#       github.event.pull_request.head.repo.full_name == github.repository
#     and the repository's Actions setting requires approval before any external
#     contributor's workflow runs at all
#     (`actions/permissions/fork-pr-contributor-approval`,
#     `approval_policy=all_external_contributors`).
#   * The runner is repository-scoped, not organisation-scoped, so a different
#     repository cannot schedule work on this Mac.
#   * It runs as an ordinary user agent. Do not run this script with sudo; it
#     refuses.
#   * It is still a machine on your network running code from the repository.
#     Anyone who can push a branch here can run commands on it. That is the
#     accepted trade-off, and it is why fork PRs are excluded rather than
#     sandboxed.
#
# USAGE
#   scripts/ci/install-selfhosted-runner.sh --dry-run      # resolve + verify only
#   scripts/ci/install-selfhosted-runner.sh                # install and start
#   scripts/ci/install-selfhosted-runner.sh --version v2.337.0
#   scripts/ci/install-selfhosted-runner.sh --uninstall    # stop, remove, deregister
#
# REQUIREMENTS
#   macOS on Apple Silicon; `gh` authenticated as a user with admin on the
#   repository (the registration token comes from `gh api`); curl; shasum.
set -euo pipefail
set -o pipefail

REPO="flash-tex/flashtex"
VERSION=""                 # empty => the newest release
LABELS="self-hosted,macOS,ARM64,flashtex"
RUNNER_NAME="$(scutil --get ComputerName 2>/dev/null || hostname -s)"
BASE="$HOME/Library/Application Support/flashtex-actions-runner"
CACHE="$HOME/Library/Caches/flashtex-actions-runner"
DRY_RUN=0
UNINSTALL=0
GROUP=""

die() { echo "install-selfhosted-runner.sh: $*" >&2; exit 1; }
say() { printf '\033[1m==> %s\033[0m\n' "$*"; }

usage() { sed -n '2,60p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo) REPO="${2:?--repo needs owner/name}"; shift 2 ;;
    --version) VERSION="${2:?--version needs vX.Y.Z}"; shift 2 ;;
    --name) RUNNER_NAME="${2:?--name needs a runner name}"; shift 2 ;;
    --labels) LABELS="${2:?--labels needs a comma-separated list}"; shift 2 ;;
    --group) GROUP="${2:?--group needs a runner group}"; shift 2 ;;
    --dir) BASE="${2:?--dir needs a path}"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --uninstall) UNINSTALL=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
done

# ---------------------------------------------------------------------------
# Preconditions
# ---------------------------------------------------------------------------
[[ "$(uname -s)" == Darwin ]] || die "this installs the macOS runner; this is $(uname -s)"
[[ "$(uname -m)" == arm64 ]] || die "this installs the osx-arm64 runner; this machine is $(uname -m)"
[[ "$(id -u)" != "0" ]] || die "do not run this as root. It installs a PER-USER launchd agent."
[[ -z "${SUDO_USER:-}" ]] || die "do not run this under sudo (SUDO_USER is set)."
command -v gh >/dev/null || die "gh is required (brew install gh), authenticated with admin on $REPO"
command -v curl >/dev/null || die "curl is required"
command -v shasum >/dev/null || die "shasum is required"

RUNNER_DIR="$BASE/runner"
WORK_DIR="$BASE/_work"
HOOKS_DIR="$BASE/hooks"

# ---------------------------------------------------------------------------
# --uninstall
# ---------------------------------------------------------------------------
if (( UNINSTALL )); then
  [[ -d "$RUNNER_DIR" ]] || die "no runner at $RUNNER_DIR"
  say "Stopping and removing the launchd service"
  ( cd "$RUNNER_DIR" && ./svc.sh stop || true )
  ( cd "$RUNNER_DIR" && ./svc.sh uninstall || true )
  say "Deregistering from $REPO"
  token="$(gh api -X POST "repos/$REPO/actions/runners/remove-token" --jq .token)" \
    || die "could not get a removal token; gh needs admin on $REPO"
  ( cd "$RUNNER_DIR" && ./config.sh remove --token "$token" ) \
    || echo "config.sh remove failed; remove the runner in the repository's Settings > Actions > Runners" >&2
  say "Deleting $BASE (the cargo cache under $CACHE is left in place)"
  rm -rf "$BASE"
  echo "done. The cargo/rustup caches are still at $CACHE; delete them by hand if you want the disk back."
  exit 0
fi

# ---------------------------------------------------------------------------
# 1. Resolve the release and its published SHA-256
# ---------------------------------------------------------------------------
say "Resolving the actions/runner release"
if [[ -n "$VERSION" ]]; then
  rel_json="$(gh api "repos/actions/runner/releases/tags/$VERSION")" \
    || die "no actions/runner release tagged $VERSION"
else
  rel_json="$(gh api repos/actions/runner/releases/latest)"
fi

release_file="$(mktemp "${TMPDIR:-/tmp}/ftrunner-release-XXXXXX")"
printf '%s' "$rel_json" > "$release_file"

# The release notes carry the hashes in machine-readable markers, e.g.
#   - actions-runner-osx-arm64-2.337.0.tar.gz <!-- BEGIN SHA osx-arm64 -->5a2c...<!-- END SHA osx-arm64 -->
# That published hash is the only thing that makes the download trustworthy, so
# a release whose notes do not carry it is refused rather than accepted.
eval "$(python3 - "$release_file" <<'PY'
import json, re, sys, shlex

rel = json.load(open(sys.argv[1], encoding="utf-8"))
tag = rel["tag_name"]
ver = tag.lstrip("v")
asset_name = "actions-runner-osx-arm64-%s.tar.gz" % ver
url = next((a["browser_download_url"] for a in rel.get("assets", [])
            if a["name"] == asset_name), "")
body = rel.get("body") or ""
m = re.search(r"<!--\s*BEGIN SHA osx-arm64\s*-->\s*([0-9a-fA-F]{64})\s*<!--\s*END SHA osx-arm64\s*-->", body)
sha = m.group(1).lower() if m else ""
if not url:
    # The asset is always published at the predictable path; fall back to it
    # rather than failing, but keep the hash requirement absolute.
    url = "https://github.com/actions/runner/releases/download/%s/%s" % (tag, asset_name)
print("RUNNER_TAG=%s" % shlex.quote(tag))
print("RUNNER_VER=%s" % shlex.quote(ver))
print("RUNNER_ASSET=%s" % shlex.quote(asset_name))
print("RUNNER_URL=%s" % shlex.quote(url))
print("RUNNER_SHA=%s" % shlex.quote(sha))
PY
)"
rm -f "$release_file"

[[ -n "${RUNNER_SHA:-}" ]] || die "release $RUNNER_TAG publishes no SHA-256 for osx-arm64; refusing to install an unverified binary"
echo "  release:  $RUNNER_TAG"
echo "  asset:    $RUNNER_ASSET"
echo "  url:      $RUNNER_URL"
echo "  sha256:   $RUNNER_SHA  (published in the release notes)"

# ---------------------------------------------------------------------------
# 2. Download and verify
# ---------------------------------------------------------------------------
say "Downloading and verifying"
dl_dir="$(mktemp -d "${TMPDIR:-/tmp}/ftrunner-XXXXXX")"
tarball="$dl_dir/$RUNNER_ASSET"
curl --fail --location --proto '=https' --tlsv1.2 --silent --show-error \
  --output "$tarball" "$RUNNER_URL"
got="$(shasum -a 256 "$tarball" | awk '{print $1}')"
if [[ "$got" != "$RUNNER_SHA" ]]; then
  rm -rf "$dl_dir"
  die "SHA-256 mismatch for $RUNNER_ASSET
  published: $RUNNER_SHA
  got:       $got
Nothing was unpacked. Do not retry blindly: check the release page."
fi
echo "  sha256 matches the published hash"

if (( DRY_RUN )); then
  echo
  say "--dry-run: verified the download and stopped"
  echo "  the tarball is at $tarball (delete it when you are done)"
  echo "  it would have been installed into:"
  echo "    runner:  $RUNNER_DIR"
  echo "    work:    $WORK_DIR   (wiped after every job)"
  echo "    caches:  $CACHE/cargo, $CACHE/rustup   (kept)"
  echo "    labels:  $LABELS"
  echo "    service: per-user launchd agent via $RUNNER_DIR/svc.sh"
  exit 0
fi

# ---------------------------------------------------------------------------
# 3. Unpack
# ---------------------------------------------------------------------------
[[ ! -e "$RUNNER_DIR/config.sh" ]] || die "a runner is already installed at $RUNNER_DIR; run --uninstall first"
say "Unpacking into $RUNNER_DIR"
mkdir -p "$RUNNER_DIR" "$WORK_DIR" "$HOOKS_DIR" "$CACHE/cargo" "$CACHE/rustup" "$BASE/logs"
tar xzf "$tarball" -C "$RUNNER_DIR"
rm -rf "$dl_dir"
printf '%s  %s\n' "$RUNNER_SHA" "$RUNNER_ASSET" > "$BASE/INSTALLED-SHA256"
echo "$RUNNER_TAG" > "$BASE/INSTALLED-VERSION"

# ---------------------------------------------------------------------------
# 4. Per-job hooks: a fresh working directory every time
# ---------------------------------------------------------------------------
# The runner is persistent (it accepts job after job), but no job may see
# anything the previous one left behind. `_actions` and `_tool` are the runner's
# own caches for downloaded actions and toolchains and are deliberately kept --
# they contain no repository state.
say "Writing the per-job hooks"
cat > "$HOOKS_DIR/job-started.sh" <<'HOOK'
#!/bin/bash
# Runs before every job (ACTIONS_RUNNER_HOOK_JOB_STARTED).
set -uo pipefail
echo "[flashtex] job starting on $(hostname -s); work root: ${RUNNER_WORKSPACE:-unset}"
# If the previous job's completion hook did not get to run (a crash, a
# `launchctl kickstart -k`), clear the workspace now rather than inheriting it.
if [[ -n "${RUNNER_WORKSPACE:-}" && "$RUNNER_WORKSPACE" == */_work/* ]]; then
  rm -rf "$RUNNER_WORKSPACE"
  mkdir -p "$RUNNER_WORKSPACE"
fi
exit 0
HOOK

cat > "$HOOKS_DIR/job-completed.sh" <<'HOOK'
#!/bin/bash
# Runs after every job (ACTIONS_RUNNER_HOOK_JOB_COMPLETED). This is what makes
# the working directory ephemeral: the next job starts from nothing.
set -uo pipefail
# Guard the rm: only ever inside the runner's own _work tree.
for d in "${RUNNER_WORKSPACE:-}" "${RUNNER_TEMP:-}"; do
  [[ -n "$d" ]] || continue
  case "$d" in
    */_work/*) rm -rf "$d" && echo "[flashtex] removed $d" ;;
    *) echo "[flashtex] refusing to remove $d: not under a _work directory" ;;
  esac
done
exit 0
HOOK
chmod +x "$HOOKS_DIR/job-started.sh" "$HOOKS_DIR/job-completed.sh"

# ---------------------------------------------------------------------------
# 5. Environment every job sees (.env in the runner root)
# ---------------------------------------------------------------------------
# CARGO_HOME and RUSTUP_HOME live outside _work so the registry, the git
# checkouts and the toolchains survive the wipe: that is where the time goes,
# not in target/.
#
# CARGO_TARGET_DIR is deliberately NOT set. The root workspace and
# crates/render-pipeline/vendor/ contain crates with the SAME package names at
# the same versions (see Cargo.toml), so one shared target directory would have
# them overwrite each other's artifacts. Build caching across jobs comes from
# Swatinem/rust-cache in the workflows instead.
say "Writing $RUNNER_DIR/.env"
cat > "$RUNNER_DIR/.env" <<ENV
ACTIONS_RUNNER_HOOK_JOB_STARTED=$HOOKS_DIR/job-started.sh
ACTIONS_RUNNER_HOOK_JOB_COMPLETED=$HOOKS_DIR/job-completed.sh
CARGO_HOME=$CACHE/cargo
RUSTUP_HOME=$CACHE/rustup
CARGO_TERM_COLOR=always
CARGO_INCREMENTAL=0
LANG=en_US.UTF-8
PATH=$CACHE/cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:/Library/TeX/texbin
ENV

# ---------------------------------------------------------------------------
# 6. Register
# ---------------------------------------------------------------------------
say "Registering with $REPO"
reg_token="$(gh api -X POST "repos/$REPO/actions/runners/registration-token" --jq .token)" \
  || die "could not get a registration token; gh needs admin on $REPO"

group_args=()
[[ -n "$GROUP" ]] && group_args=(--runnergroup "$GROUP")

# --disableupdate: the runner would otherwise replace its own binary with a
# newer release, which is precisely the binary whose hash was NOT verified here.
# To move to a new version, run --uninstall and then install with --version.
( cd "$RUNNER_DIR" && ./config.sh \
    --url "https://github.com/$REPO" \
    --token "$reg_token" \
    --name "$RUNNER_NAME" \
    --labels "$LABELS" \
    --work "$WORK_DIR" \
    --disableupdate \
    --unattended \
    --replace \
    "${group_args[@]}" )
unset reg_token

# ---------------------------------------------------------------------------
# 7. Per-user launchd service
# ---------------------------------------------------------------------------
# GitHub's svc.sh writes ~/Library/LaunchAgents/actions.runner.<repo>.<name>.plist
# and drives it with launchctl. It is a user agent: it starts at login and stops
# at logout, which is why these Macs should be logged in and set to never sleep
# (`sudo pmset -a sleep 0 disablesleep 1` on a machine that is always on mains).
say "Installing the per-user launchd service"
( cd "$RUNNER_DIR" && ./svc.sh install )
( cd "$RUNNER_DIR" && ./svc.sh start )

echo
say "Installed"
cat <<EOF
  runner:    $RUNNER_DIR   ($RUNNER_TAG, sha256 verified)
  work:      $WORK_DIR   (wiped after every job)
  caches:    $CACHE/cargo, $CACHE/rustup
  labels:    $LABELS
  agent:     ~/Library/LaunchAgents/actions.runner.*.plist
  logs:      $RUNNER_DIR/_diag

Verify it, in this order:
  (cd "$RUNNER_DIR" && ./svc.sh status)
  launchctl list | grep actions.runner
  gh api repos/$REPO/actions/runners --jq '.runners[] | {name, status, busy, labels: [.labels[].name]}'

Then tell the workflows to use it:
  gh variable set FLASHTEX_SELFHOSTED_MAC --repo $REPO --body 1

Until that variable is 1, every Mac job stays on GitHub-hosted runners, so the
fallback is the default and this runner can be registered and watched before
anything depends on it.
EOF
