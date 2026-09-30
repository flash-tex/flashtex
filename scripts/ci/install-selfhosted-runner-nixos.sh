#!/usr/bin/env bash
# Install (or re-provision) one GitHub Actions runner instance on the owner's
# NixOS PC, as a per-user systemd service, with the rules a PUBLIC repository
# requires (docs/design/engine-v2/DESIGN.md §9.3). The Linux counterpart of
# scripts/ci/install-selfhosted-runner.sh; see docs/ci-cd.md.
#
# WHY. The owner, 2026-09-29: "any tests that can be run on the nixos pc should
# be run there, as all other machines are laptops". One instance is not enough
# for a CI run's parallel Linux jobs, so the PC runs several, each with its own
# everything: runner root, work directory, CARGO_HOME and RUSTUP_HOME. Sharing
# rustup between concurrent jobs broke builds on the Macs (one job's toolchain
# update deleted files under another job's compiler).
#
# WHAT IT DOES, for instance N (1, 2, 3, ...)
#   1. Uses nixpkgs' `github-runner` from ~/.nix-profile (install it with
#      `nix profile install nixpkgs#github-runner`). Its integrity comes from
#      the Nix store: a content-addressed path fetched from a signed binary
#      cache, not a tarball downloaded by this script. `disableUpdate` is set,
#      so the runner never replaces that binary with an unverified release.
#   2. Root ~/.flashtex-actions-runner (N=1) or ~/.flashtex-actions-runner-N,
#      work directory <root>/_work, emptied before and after every job by the
#      runner's job hooks.
#   3. CARGO_HOME and RUSTUP_HOME under ~/.cache/flashtex-actions-runner/<root>/,
#      outside the work directory (the registry and toolchains survive the
#      wipe) and never shared with another instance or with the interactive
#      user's ~/.cargo and ~/.rustup.
#   4. PATH for every job: TeX Live 2026 first (~/texlive/2026/bin/x86_64-linux;
#      the Nix profile carries an older TeX Live whose pdftex must not win),
#      then the Nix profile (qpdf, git, python3, gh), then the system.
#   5. Registers the instance against this repository, unless <root>/.runner
#      already exists (then it only rewrites hooks, environment and unit).
#   6. Writes ~/.config/systemd/user/<service>.service, enables and (re)starts it.
#      Lingering must be on (`loginctl enable-linger`, set once by the owner)
#      so the services run without a login session. No root anywhere.
#
# THE REGISTRATION TOKEN is read from the environment variable
# FLASHTEX_RUNNER_TOKEN and never printed. From a Mac with `gh` authenticated
# as a repository admin:
#   gh api -X POST repos/flash-tex/flashtex/actions/runners/registration-token --jq .token |
#     ssh kubar@nixos 'IFS= read -r FLASHTEX_RUNNER_TOKEN; export FLASHTEX_RUNNER_TOKEN;
#                      bash ~/install-selfhosted-runner-nixos.sh --instance 2'
# --uninstall needs a removal token the same way (.../runners/remove-token).
#
# SECURITY. As for the Macs: ci.yml and nightly.yml send a job here only for
# merge_group, push to main, workflow_dispatch and schedule -- never for
# pull_request or a branch push -- so a fork's code never reaches this machine.
# The runner is repository-scoped and runs as an ordinary user. Anyone who can
# push to main or queue a merge can run commands here; that is the accepted
# trade-off.
#
# USAGE
#   install-selfhosted-runner-nixos.sh --instance 2 --dry-run   # show the plan
#   install-selfhosted-runner-nixos.sh --instance 2             # install and start
#   install-selfhosted-runner-nixos.sh --instance 1 --no-restart
#   install-selfhosted-runner-nixos.sh --instance 3 --uninstall
set -euo pipefail

REPO="flash-tex/flashtex"
INSTANCE=""
NAME=""
LABELS="self-hosted,Linux,X64,flashtex-linux"
TEXLIVE_BIN="$HOME/texlive/2026/bin/x86_64-linux"
NIX_BIN="$HOME/.nix-profile/bin"
DRY_RUN=0
UNINSTALL=0
RESTART=1

die() { echo "install-selfhosted-runner-nixos.sh: $*" >&2; exit 1; }
say() { printf '\033[1m==> %s\033[0m\n' "$*"; }
header() {
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --instance) INSTANCE="${2:?--instance needs a number}"; shift 2 ;;
    --name) NAME="${2:?--name needs a runner name}"; shift 2 ;;
    --labels) LABELS="${2:?--labels needs a comma-separated list}"; shift 2 ;;
    --repo) REPO="${2:?--repo needs owner/name}"; shift 2 ;;
    --texlive) TEXLIVE_BIN="${2:?--texlive needs a bin directory}"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --uninstall) UNINSTALL=1; shift ;;
    --no-restart) RESTART=0; shift ;;
    -h|--help) header; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
done

[[ "$INSTANCE" =~ ^[1-9][0-9]*$ ]] || die "--instance N is required (1, 2, 3, ...)"

# ---------------------------------------------------------------------------
# Preconditions
# ---------------------------------------------------------------------------
[[ "$(uname -s)" == Linux ]] || die "this installs the Linux runner; this is $(uname -s)"
[[ "$(uname -m)" == x86_64 ]] || die "the labels say X64; this machine is $(uname -m)"
[[ "$(id -u)" != "0" ]] || die "do not run this as root. It installs a PER-USER systemd service."
[[ -z "${SUDO_USER:-}" ]] || die "do not run this under sudo (SUDO_USER is set)."
[[ -x "$NIX_BIN/Runner.Listener" && -x "$NIX_BIN/config.sh" ]] \
  || die "no nixpkgs github-runner in $NIX_BIN; nix profile install nixpkgs#github-runner"
command -v systemctl >/dev/null || die "systemctl is required"

if (( INSTANCE == 1 )); then
  BASE="$HOME/.flashtex-actions-runner"
  SERVICE="flashtex-actions-runner"
  : "${NAME:=$(hostname -s)-7800x3d}"
else
  BASE="$HOME/.flashtex-actions-runner-$INSTANCE"
  SERVICE="flashtex-actions-runner-$INSTANCE"
  : "${NAME:=$(hostname -s)-7800x3d-$INSTANCE}"
fi
case "$BASE" in *" "*) die "runner paths must not contain spaces: $BASE" ;; esac

WORK_DIR="$BASE/_work"
HOOKS_DIR="$BASE/hooks"
TOOLS="$HOME/.cache/flashtex-actions-runner/$(basename "$BASE")"
UNIT="$HOME/.config/systemd/user/$SERVICE.service"
JOB_PATH="$TOOLS/cargo/bin:$TEXLIVE_BIN:/run/wrappers/bin:$NIX_BIN:/etc/profiles/per-user/$USER/bin:/nix/var/nix/profiles/default/bin:/run/current-system/sw/bin"

# ---------------------------------------------------------------------------
# --uninstall
# ---------------------------------------------------------------------------
if (( UNINSTALL )); then
  [[ -d "$BASE" ]] || die "no runner at $BASE"
  (( ! DRY_RUN )) || { echo "would stop $SERVICE, deregister $NAME and delete $BASE"; exit 0; }
  say "Stopping $SERVICE"
  systemctl --user disable --now "$SERVICE" 2>/dev/null || true
  rm -f "$UNIT"
  systemctl --user daemon-reload
  say "Deregistering $NAME from $REPO"
  if [[ -n "${FLASHTEX_RUNNER_TOKEN:-}" ]]; then
    RUNNER_ROOT="$BASE" "$NIX_BIN/config.sh" remove --token "$FLASHTEX_RUNNER_TOKEN" \
      || echo "config.sh remove failed; remove $NAME in Settings > Actions > Runners" >&2
  else
    echo "no FLASHTEX_RUNNER_TOKEN (a removal token); remove $NAME in Settings > Actions > Runners" >&2
  fi
  rm -rf "$BASE"
  echo "done. The toolchains under $TOOLS are left in place."
  exit 0
fi

[[ -x "$TEXLIVE_BIN/pdftex" ]] || die "no TeX Live at $TEXLIVE_BIN (pass --texlive DIR)"
[[ -x "$NIX_BIN/qpdf" ]] || die "qpdf is missing: nix profile install nixpkgs#qpdf"

registered=0
[[ -f "$BASE/.runner" ]] && registered=1

say "Instance $INSTANCE"
cat <<EOF
  name:     $NAME   ($([ $registered = 1 ] && echo "already registered; environment only" || echo "will register"))
  labels:   $LABELS
  root:     $BASE
  work:     $WORK_DIR   (emptied before and after every job)
  cargo:    $TOOLS/cargo
  rustup:   $TOOLS/rustup
  TeX:      $("$TEXLIVE_BIN/pdftex" --version | head -1)
  qpdf:     $("$NIX_BIN/qpdf" --version | head -1)
  service:  $UNIT
EOF
if (( DRY_RUN )); then
  say "--dry-run: nothing written"
  exit 0
fi

mkdir -p "$BASE" "$WORK_DIR" "$HOOKS_DIR" "$TOOLS/cargo" "$TOOLS/rustup" "$(dirname "$UNIT")"

# ---------------------------------------------------------------------------
# Register (only a new instance)
# ---------------------------------------------------------------------------
if (( ! registered )); then
  [[ -n "${FLASHTEX_RUNNER_TOKEN:-}" ]] \
    || die "set FLASHTEX_RUNNER_TOKEN to a registration token (see the header); it is never printed"
  say "Registering $NAME with $REPO"
  RUNNER_ROOT="$BASE" "$NIX_BIN/config.sh" \
    --url "https://github.com/$REPO" \
    --token "$FLASHTEX_RUNNER_TOKEN" \
    --name "$NAME" \
    --labels "$LABELS" \
    --work "$WORK_DIR" \
    --disableupdate \
    --unattended \
    --replace >/dev/null
  unset FLASHTEX_RUNNER_TOKEN
fi

# ---------------------------------------------------------------------------
# Job hooks: a fresh working directory every time
# ---------------------------------------------------------------------------
say "Writing the job hooks"
cat > "$HOOKS_DIR/job-started.sh" <<'HOOK'
#!/usr/bin/env bash
# Start every job from an empty workspace. The runner has already created
# $GITHUB_WORKSPACE and uses it as the cwd of the next step, so empty it in
# place rather than deleting the directory.
set -uo pipefail
echo "[flashtex] job starting on $(hostname -s) (${RUNNER_NAME:-?}); workspace: ${GITHUB_WORKSPACE:-unset}"
if [[ -n "${GITHUB_WORKSPACE:-}" && "$GITHUB_WORKSPACE" == */_work/* ]]; then
  mkdir -p "$GITHUB_WORKSPACE"
  find "$GITHUB_WORKSPACE" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
fi
exit 0
HOOK
cat > "$HOOKS_DIR/job-completed.sh" <<'HOOK'
#!/usr/bin/env bash
# Empty the job's workspace and temp dir in place after every job. Only ever
# inside the runner's own _work tree.
set -uo pipefail
for d in "${GITHUB_WORKSPACE:-}" "${RUNNER_TEMP:-}"; do
  [[ -n "$d" && -d "$d" ]] || continue
  case "$d" in
    */_work/*) find "$d" -mindepth 1 -maxdepth 1 -exec rm -rf {} + && echo "[flashtex] emptied $d" ;;
    *) echo "[flashtex] refusing to touch $d: not under a _work directory" ;;
  esac
done
exit 0
HOOK
chmod +x "$HOOKS_DIR/job-started.sh" "$HOOKS_DIR/job-completed.sh"

# ---------------------------------------------------------------------------
# Environment: .env (the runner's environment) and .path (every job's PATH)
# ---------------------------------------------------------------------------
# CARGO_TARGET_DIR is deliberately not set (see the Mac installer): build
# caching across jobs is Swatinem/rust-cache's job.
say "Writing $BASE/.env and $BASE/.path"
cat > "$BASE/.env" <<ENV
ACTIONS_RUNNER_HOOK_JOB_STARTED=$HOOKS_DIR/job-started.sh
ACTIONS_RUNNER_HOOK_JOB_COMPLETED=$HOOKS_DIR/job-completed.sh
CARGO_HOME=$TOOLS/cargo
RUSTUP_HOME=$TOOLS/rustup
CARGO_TERM_COLOR=always
CARGO_INCREMENTAL=0
LANG=en_US.UTF-8
PATH=$JOB_PATH
ENV
printf '%s\n' "$JOB_PATH" > "$BASE/.path"

# ---------------------------------------------------------------------------
# Per-user systemd service
# ---------------------------------------------------------------------------
# Nice=10 and MemoryHigh keep three concurrent release builds from starving
# the desktop: MemoryHigh throttles an instance rather than killing it.
say "Writing $UNIT"
cat > "$UNIT" <<UNIT
[Unit]
Description=GitHub Actions runner $NAME for $REPO (self-hosted Linux, instance $INSTANCE)
After=network-online.target
[Service]
Environment=RUNNER_ROOT=$BASE
Environment=PATH=$NIX_BIN:/run/current-system/sw/bin:/usr/bin:/bin
WorkingDirectory=$BASE
ExecStart=$NIX_BIN/Runner.Listener run --startuptype service
Restart=always
RestartSec=10
Nice=10
MemoryHigh=9G
[Install]
WantedBy=default.target
UNIT
systemctl --user daemon-reload
systemctl --user enable "$SERVICE" >/dev/null 2>&1
if (( RESTART )); then
  # A restart cancels the job in progress, so re-provisioning a busy instance
  # should pass --no-restart and restart it once it is idle.
  systemctl --user restart "$SERVICE"
  say "Started $SERVICE"
else
  say "Wrote $SERVICE; not restarted (--no-restart). Restart it when idle."
fi
cat <<EOF

Verify:
  systemctl --user status $SERVICE
  gh api repos/$REPO/actions/runners --jq '.runners[] | {name, status, busy}'
EOF
