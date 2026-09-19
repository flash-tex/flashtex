# Beads cutover runbook

**Prerequisites:**
- This PR is reviewed.
- The owner decisions of 2026-09-13 stand: pins 1.2.2/2.3.3, metrics off, `BD_SMART_GATE=0`, Commander-only migration.
- Conventions are in [beads.md](beads.md).

**Who runs what:**

| Machine | Part |
|---|---|
| Commander machine (today `mac-m1max-a`, per `coordination/authority.json`) | Part A |
| Daniel `mac-m5pro-dq222` and Kabir `mac-m5pro-kabir` | Part B |
| Jaysen `mac-m1max-a` | also Part B for its non-Commander clones |

The steps were exercised on `GoKubar/flashtex-beads-trial` (2026-09-13).

**Real ledger: created 2026-09-13 ~08:30Z** from mac-m5pro-kabir (owner-authorized one-time init):
- config PR #101, `refs/dolt/data` pushed, 18 seed beads
- sync loops on mac-m5pro-kabir (launchd) and the NixOS PC (systemd user)
- **Part A steps 1–5 are therefore done; other machines start at Part B.**

> **Status: cleared on the trial repo** by soak run 4 (2026-09-13, PASS; beads.md §10). The owner authorized initial creation of the real ledger from mac-m5pro-kabir via a branch/PR plus `refs/dolt/data` (never a direct main write). The Commander still announces cutover; until then issue #2 stays the protocol.

## Part A: Commander machine

1. **Install and identify.**
   ```bash
   scripts/beads/install-pinned.sh          # prints bd 1.2.2 / dolt 2.3.3 + sha256
   mkdir -p ~/.config/flashtex && echo mac-m1max-a > ~/.config/flashtex/machine
   scripts/beads/bd version && ~/.local/share/flashtex-beads/bin/dolt version
   scripts/beads/bd metrics | head -1        # must print: Anonymous usage metrics: OFF
   ```
2. **Initialise the ledger** from a dedicated **standalone clone** on a branch. Never use a worktree of an existing checkout: bd would write `.beads/` into the main checkout.
   - `bd init` commits `.beads/` files to the *current branch* itself ("✓ Committed beads files to git"), so never run it on main.
   ```bash
   git clone --depth 1 --branch main git@github.com:flash-tex/flashtex.git ~/.local/share/flashtex-beads/ledger
   cd ~/.local/share/flashtex-beads/ledger && git checkout -b <agent>/beads-init
   scripts/beads/bd init --prefix ft --skip-agents --skip-hooks --non-interactive
   git config beads.role maintainer && chmod 700 .beads
   scripts/beads/bd dolt remote list          # expect origin git+ssh://git@github.com/flash-tex/flashtex.git
   scripts/beads/bd config set types.custom message   # optional; messages still use -t task (beads.md §7)
   ```
   - The wrapper permits `init` in flash-tex/flashtex only on the authority machine, **or** once, when `FLASHTEX_BEADS_OWNER_AUTHORIZED_INIT` is set to a non-empty authorization note and origin has no `refs/dolt/data` yet. The owner used this on 2026-09-13 from mac-m5pro-kabir.
   - VERIFIED on a fresh repo: `--skip-agents --skip-hooks` leaves AGENTS.md and CLAUDE.md byte-identical and `core.hooksPath` unset. Without `--skip-hooks`, `core.hooksPath` becomes `.beads/hooks`. The init commit contains only `.beads/{.gitignore,README.md,config.yaml,interactions.jsonl,metadata.json}` plus root `.gitignore` lines.
   - bd commits with the clone's git identity. Set the required local `user.name`/`user.email` first, then amend to add trailers **before** pushing.
   - If the branch already has a committed `.beads/config.yaml` with `sync.remote`, `bd init` bootstraps from that remote and ignores `--prefix` (VERIFIED). Init must run on a tree without `.beads/`.
3. **Authority bead**, mirroring `coordination/authority.json`:
   ```bash
   scripts/beads/bd create "Commander authority" --id ft-authority -t decision --silent \
     --metadata '{"commander_id":"<commander_id>","machine":"mac-m1max-a","authority_state":"active","epoch":1,
                  "claim_mode":"<from authority.json>","bd_pin":"1.2.2","dolt_pin":"2.3.3","upgrade_state":"frozen",
                  "updated_utc":"<utc>","heartbeat_utc":"<utc>"}'
   ```
4. **Seed only genuinely active work.**
   - Read `coordination/assignments/*.json` on current main. Seed rows whose `state` is not `integrated`, `cancelled` or `done`, **and** that have fresh live evidence per TASKS.md's staffing rule.
   - Historical `assigned` rows without a live owner become one `-l stale-assignment` bead each, unassigned, so the Commander decides explicitly rather than silently dropping them.
   - As of origin/main `f6645607`:
     - **FT-060..067:** `kabir-claude` / `mac-m5pro-kabir`, rev 1, state `assigned`, no dependencies recorded.
     - **FT-050..055:** `mac-claude-a` / `mac-m1max-a`, `assigned`.
     - **FT-030..046:** `daniel-*` on mac-m5pro-dq222, `assigned`; only if Daniel is live.
     - **FT-002/003/004/007/009/010/014/015/017/023/028/048/049:** historical; verify each before seeding.
   ```bash
   # one bead per active assignment; external ref keeps the JSON as history
   scripts/beads/bd create "FT-060 HW2 math completion + \\mathcal from NewCMMath" --id ft-060 -t task \
     --assignee kabir-claude -l machine:mac-m5pro-kabir,ft-task --external-ref coordination/assignments/FT-060.json \
     --notes "branch agent/kabir-claude/hw2-math-final; rev 1" --silent
   #   … repeat for FT-061..067 (branches: amsmath-envs, tikz-min, graphics-floats, hyphenation,
   #     compiler-perf, conversion-provider, supported-latex) and the other live rows.
   scripts/beads/bd update ft-060 --status in_progress   # ONLY if the owner has a fresh ACK/checkpoint
   ```
   - **Dependency edges:** only real ones, from the assignment `dependencies` arrays and TASKS.md.
     - FT-052 depends on the FT-051 compiler branch: `bd dep add ft-052 ft-051`.
     - FT-050's Mac lane consumes its contract, recorded as a decision bead.
     - FT-060..067 record no dependencies, so add none.
   - **Agent beads:** `bd create "agent kabir-claude" --id ft-agent-kabir-claude -t chore -l agent,machine:mac-m5pro-kabir`, one per live agent.
   - **Open contract negotiations become decision beads** (`--spec-id` = the registry path), e.g. FT-066 `provider_evidence` on transfer-v1.
5. **Publish.**
   ```bash
   scripts/beads/bd dolt push                     # creates refs/dolt/data + refs/heads/__dolt_remote_info__
   git ls-remote origin 'refs/dolt/*' refs/heads/__dolt_remote_info__
   git push -u origin commander/beads-init && gh pr create --draft …   # .beads/config.yaml, .beads/.gitignore, .gitignore
   ```
   - Merge that PR through the normal integration route. Other machines need `.beads/config.yaml` on main to bootstrap.
6. **Control docs**, in the same cutover PR, as Commander:
   - Apply the AGENTS.md "Beads coordination ledger" section and the CLAUDE.md line from this PR.
   - Fix the stale text (owner decision 6). Draft replacements:
     - AGENTS.md:87-88 `Current sole orchestrator: **orchestrator-astra** … on linux-primary` → `Current Commander: see the Beads bead ft-authority (mirror: coordination/authority.json).`
     - AGENTS.md:41 `Commander remains linux-primary while usable` → remove (authority record governs).
     - AGENTS.md:17, 28, 50, 81, 194 (Astra staffing banners) → mark `HISTORICAL (pre-2026-09-13)` or delete per owner.
     - AGENTS.md:96 `primary Codex agent on linux-primary as Commander` → `the agent named in ft-authority`.
     - AGENTS.md:130-135 recovery issues → `bd create -t bug -l recovery …` in the ledger (product bugs stay GitHub issues).
     - docs/commander-failover.md:3, 34, 64-66, 96, 122 (Astra/linux-primary witness) → replace with a pointer to beads.md §9. Keep the eligibility gates verbatim.
   - Make `coordination/authority.json` a mirror: add `"mirror_of": "beads:ft-authority"` and update it after each claim.
7. **Machine sync loop** on every machine. It is a supervisor, not a model call.
   - Install:
     - macOS: `scripts/beads/service/install-launchd.sh <main clone>`
     - Linux: the systemd `--user` unit in `scripts/beads/service/`, or the home-manager snippet.
   - Check: `scripts/beads/ledger-status` must print `FRESH`.
   - The loop does the following (beads.md §5–6):
     - pulls every 60 s
     - pushes unpushed commits (at most every 15 s)
     - backs off to 60 s on outages
     - on the merge-conflict wedge, runs `scripts/beads/ledger-recover` (lossless: lock-held settle in a copy, remote wins contested cells, loser notified)
     - writes `HOLD` and stops on anything it cannot settle losslessly
   - Delete-and-`bootstrap` is the operator's last resort only. It discards unpushed writes.

## Part B: every other machine (Daniel, Jaysen non-Commander clones, Kabir)

**The ledger lives in ONE standalone clone per machine, never in your main checkout.** bd resolves any git worktree to its main clone's root. Bootstrapping or initialising inside a normal checkout (or one of its worktrees) writes `.beads/` into that checkout (VERIFIED 2026-09-13). Agents route to the ledger clone through `BEADS_DIR`. The wrapper reads its default from `~/.config/flashtex/beads-dir`.

```bash
P=~/.local/share/flashtex-beads                                   # layout used on mac-m5pro-kabir and the NixOS PC
scripts/beads/install-pinned.sh                                   # from a checkout of the Beads tooling (PR #84 or main once merged)
mkdir -p $P/tools && git archive <ref-with-scripts/beads> scripts/beads | tar -x -C $P/tools
echo <machine-alias> > ~/.config/flashtex/machine
git clone --depth 1 --branch main git@github.com:flash-tex/flashtex.git $P/ledger   # standalone, NOT a worktree
cd $P/ledger && git config beads.role maintainer
$P/tools/scripts/beads/bd bootstrap --yes                         # "Synced database from git+ssh://git@github.com/flash-tex/flashtex.git"
chmod 700 .beads && echo $P/ledger/.beads > ~/.config/flashtex/beads-dir
# sync loop, pointed at the ledger clone:
#   macOS: render scripts/beads/service/dev.flashtex.beads-sync.plist with the tools path + ledger path, launchctl bootstrap gui/$(id -u) <plist>
#   Linux: render flashtex-beads-sync.service the same way into ~/.config/systemd/user/, systemctl --user enable --now flashtex-beads-sync
$P/tools/scripts/beads/ledger-status                              # must print FRESH
```

- **Agents** run `$P/tools/scripts/beads/{bd,claim,inbox,ledger-status}` from any directory. `BEADS_DIR` comes from `~/.config/flashtex/beads-dir`.
- **`bd bootstrap` leaves `.beads/` untracked** in the ledger clone until the `.beads` config PR (#101) is on main. That is expected; do not commit it from the ledger clone.
- **Linux without lingering** (`loginctl show-user $USER | grep Linger` shows `Linger=no`): the user unit stops when the last session ends. The owner enables lingering, or uses the home-manager/system service.

**Verification. Each item must pass before the machine stops using issue comments:**
1. `scripts/beads/bd version` shows `bd version 1.2.2 (6c124203e…`, and `dolt version` shows `2.3.3`. Post both, plus the tarball sha256 lines, in `docs/resources/machines/<alias>.md`.
2. `scripts/beads/bd metrics | head -1` shows `OFF`.
3. `scripts/beads/bd show ft-authority --json` names the current Commander.
4. `scripts/beads/bd list --assignee <each local agent> --json` matches that agent's assignment.
5. Round trip:
   - Create `-l msg,to:<commander_id>,from:<agent>,thread:<self> "cutover check <alias>"`, then push. The Commander reads it with `scripts/beads/inbox --agent <commander_id>`.
   - The Commander pulls, sees it, and closes it.
   - This machine pulls and sees it closed.
6. Wrapper refusal check: `scripts/beads/bd migrate schema` must print `refused: this machine is '<alias>', Commander machine is 'mac-m1max-a'`.

- **NixOS / other Linux without FHS:** the installer handles NixOS through a glibc-loader shim. Prefer `programs.nix-ld.enable = true;`.
- **Linux arm64 and Intel macOS:** hashes are included but untested. Report them if used.

## Cutover announcement

The Commander posts ONE comment on issue #2, then stops using issue comments for coordination:

> Coordination moved to Beads (bd 1.2.2 / dolt 2.3.3, pinned). From <UTC>:
> - Read work with `scripts/beads/bd ready`/`show`, claim with `bd update <id> --claim` + `bd dolt push` (a claim counts only after a successful push and re-read), close with `bd close`.
> - Message with `-l msg,to:<agent>` beads.
> - Authority is the bead `ft-authority`; `coordination/authority.json` is a mirror.
> - PRs/review/merge and product bugs stay on GitHub.
>
> Machines must pass `docs/coordination/beads-cutover-runbook.md` Part B before their next checkpoint. Issue comments on #2/#23/#52 are no longer read for dispatch.

Do not close or delete existing issues.

## Script call sites to migrate later (not changed in this PR)

| File (origin/main) | Current GitHub use | Beads replacement |
|---|---|---|
| `scripts/worker.py:236` | `gh issue create --title "Worker recovery needed: <id>"` | `scripts/beads/bd create -t bug -l recovery,machine:<m> --assignee <commander>` + push |
| `scripts/claude_worker.py:275` (+ error path `:332`) | `gh issue create --title "Claude worker recovery: <id>"` (journaled) | same as above; keep the journal, record the bead ID |
| `scripts/cooldown_dispatch.py:39` | `gh issue comment <issue> --repo flash-tex/flashtex --body …` | message bead `-l msg,to:<agent>` + push |
| `scripts/fleet_health.py:138` | `gh issue list --state open --limit 1000 --json …` | `bd list -l recovery --status open --json` after pull |
| `scripts/coord.py` (`ack`/`report`/`checkpoint`; `:358` `gh api user`) | git-file ledger + trailer lookup | ack/report also `bd update --append-notes`; `gh api user` for trailers stays |
| `scripts/claim_commander.py`, `scripts/commander_failover.py`, `scripts/select_successor.py` | authority.json claims | `scripts/beads/authority claim/heartbeat/handoff` per beads.md §9; keep gates |
| agents' claim/unclaim | (new) | `scripts/beads/claim <id> --actor <me>` / `--release`: never raw `bd update --claim` + ad-hoc undo (soak run 3 race) |
