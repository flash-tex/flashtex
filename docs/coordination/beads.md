# Beads coordination ledger (bd 1.2.2 / dolt 2.3.3)

This document covers how FlashTeX agents coordinate through Beads instead of GitHub issue comments.

- **What moves:** the ledger (tasks, claims, status, handoffs, heartbeats, authority) and agent-to-agent messages.
- **What stays on GitHub:** code integration (branches, PRs, review, merge) and product bugs.

Evidence comes from the 2026-09-13 trial on the private scratch repo `GoKubar/flashtex-beads-trial`:
- **Machine A:** mac-m5pro-kabir, macOS arm64.
- **Machine B:** Kabir's NixOS PC, x86_64.

**Labels:**
- **VERIFIED:** run and observed.
- **SOURCE:** read in the v1.2.2 source or docs.
- **BELIEVED:** not yet tested.

## 1. Pinned versions

| Tool | Version | Asset | sha256 |
|---|---|---|---|
| bd | 1.2.2 (`6c124203e`) | `beads_1.2.2_darwin_arm64.tar.gz` | `2aa1245c666419900d2d6993a05049e92c40e0e601d19579cca5b07a7bb8021d` |
| bd | 1.2.2 | `beads_1.2.2_linux_amd64.tar.gz` | `8140098a51d3b81d5548d1c5e6db1a2d9930e5d141efe2a4bff7d079c4d321e8` |
| bd | 1.2.2 | `beads_1.2.2_darwin_amd64.tar.gz` (untested) | `e192dcb60f0f48d9463cd3bcf425d41fc0a632080cf9a06153c97ce12368c4bb` |
| bd | 1.2.2 | `beads_1.2.2_linux_arm64.tar.gz` (untested) | `501f38a1070d4b9b3b6261a86a3c92c4a52366869021560430a4bb0036afd83a` |
| bd | release `checksums.txt` | | `25507c2d3ac43d17a1e7dc6ea25141b0354300cebf3f3f549b7ba3d266ee4f69` |
| dolt | 2.3.3 | `dolt-darwin-arm64.tar.gz` | `55c11d34df78d7583f1130a1adef7763340f2aade33e68ccb0046854134ad08b` |
| dolt | 2.3.3 | `dolt-linux-amd64.tar.gz` | `4acd730a4c53991996854a72fbb1add102b0a583bd07411320efb65037a43d9d` |
| dolt | 2.3.3 | `dolt-darwin-amd64.tar.gz` (untested) | `e33f4fa00032054e38da78b31314f8e93fa9eb950c587ac5f29ba5c6402b3f2a` |
| dolt | 2.3.3 | `dolt-linux-arm64.tar.gz` (untested) | `850a880aece6587cb9251ea0f07eb51fcc0a37450471fd89e03ac2fba1fdaed3` |

Dolt publishes no checksums file, so its hashes are the GitHub release asset digests.

**Rules:**
- Never install bd 1.2.0 or 1.2.1. Running 1.2.1 once migrates the DB to schema v65, which 1.2.2 refuses to open.
- No `1.3.0-rc`. Stay on 1.2.2 until 1.3.0 is stable **and** the owner approves.
- No Homebrew or npm, because they cannot hold a version.

**Install:** `scripts/beads/install-pinned.sh`
- It downloads, verifies the sha256, and installs into `~/.local/share/flashtex-beads/`. It is idempotent.
- VERIFIED on both trial machines; the re-run skips the download.
- **NixOS:** bd is a dynamically linked glibc binary, so it fails with `Could not start dynamically linked executable` (VERIFIED). dolt is static and runs as-is.
  - Without nix-ld, the installer writes a shim that runs the unmodified verified binary through nixpkgs' glibc loader, with a GC root (VERIFIED on NixOS 26.11).
  - The cleaner system fix is `programs.nix-ld.enable = true;` in the NixOS configuration (owner action).
  - nixpkgs `beads` is 1.0.3, so it is not usable for the pin (VERIFIED `nix eval nixpkgs#beads.version`).

**Always call `scripts/beads/bd`, never bd directly.** VERIFIED behaviours:

| Guard | Observed |
|---|---|
| Version mismatch | `flashtex-bd: bd version mismatch: 'bd version 1.2.1 (x)' (pinned 1.2.2). Refusing.` (exit 3) |
| `BD_SMART_GATE=0`, `BD_DISABLE_METRICS=1` | exported on every call; pinned `dolt` first on PATH (bd shells out to `dolt` for git remotes, SOURCE) |
| `bd dolt push --force` | always refused |
| `bd migrate` (not `--inspect`/`--dry-run`), `bd upgrade ack`, `bd doctor --fix`, `flatten`, `rename-prefix`, `admin`, `bd init` in flash-tex/flashtex, or `BD_ALLOW_REMOTE_MIGRATE` set | refused unless the machine (`FLASHTEX_MACHINE` or `~/.config/flashtex/machine`) equals the authority record's `machine` **and** `authority_state=active`. Refusals seen: `this machine is 'mac-m5pro-kabir', Commander machine is 'mac-m1max-a' (origin/main:coordination/authority.json)`; `authority_state='vacant' in beads:trial-authority (upgrades frozen while no Commander is active)` |

- **Authority source for the wrapper:** the Beads bead `ft-authority` (metadata `machine`, `authority_state`), falling back to `coordination/authority.json` on origin/main.

## 2. Metrics off (owner decision 2)

- bd 1.2.2 enables anonymous metrics by default. A fresh HOME prints `Anonymous usage metrics: ON` and writes `metrics: {disabled: false}` (VERIFIED).
- **Two mechanisms, both used:**
  1. `bd metrics off` writes `metrics.disabled: true` to the user-global `~/.config/bd/config.yaml` (VERIFIED on macOS and NixOS; SOURCE `cmd/bd/metrics.go`). `install-pinned.sh` runs it.
  2. The env var `BD_DISABLE_METRICS=1` overrides config. `bd metrics` then reports `OFF` with `Note: BD_DISABLE_METRICS=1 is overriding your saved config (which is "on")` (VERIFIED). The wrapper and NixOS shim always export it.
- The shim also needs metrics off for correctness: the metrics flusher re-executes `os.Executable()` (SOURCE `internal/metrics/spawn.go`).

## 3. Schema migrations and upgrades (owner decision 3)

**The gate:**
- Every machine runs `BD_SMART_GATE=0`. bd's default "smart" gate may auto-migrate as a "safe first-mover" when the remote is at the same schema (SOURCE `internal/storage/schema/smart_remote_migrate_gate.go`, convergence floor v43).
- **VERIFIED:** a remote-backed DB created by bd 1.0.4 (schema v32), opened with 1.2.2 and `BD_SMART_GATE=0`:
  - Writes and `bd dolt pull` exit 1 with `refusing to auto-apply 21 pending schema migrations to a remote-backed database (v32 -> v53): migrating clones independently forks the schema (#4259)`.
  - Reads continue with `Read-only command: continuing on schema v32 without migrating. Writes are blocked`.
- With the gate unset, the same v32 DB also blocked, because it is below the v43 floor. The smart-gate auto-migrate at ≥v43 is SOURCE/BELIEVED only.
- **Only the Commander machine migrates.** Upgrades are frozen while `authority_state` is not `active`. Migration rights move with a completed Commander claim (§8).

**Upgrade procedure** (after owner approval of a new pin; SOURCE `getting-started/upgrading.md` v1.2.2):
1. Commander records `upgrade_state=planned` on `ft-authority` and announces a freeze bead.
2. Every machine, **with the old binary**: `scripts/beads/bd dolt pull && scripts/beads/bd dolt push`, then stops writing. Pushes and pulls are refused once a newer binary with pending migrations is installed.
3. Commander machine only:
   - `bd export --all -o <backup>.jsonl` and `bd backup`
   - install the new pin (update `install-pinned.sh` hashes in a PR first)
   - `BD_ALLOW_REMOTE_MIGRATE=1 scripts/beads/bd migrate`
   - `scripts/beads/bd dolt push`
4. Every other machine: run the updated `install-pinned.sh`, move `.beads/embeddeddolt` aside, then `scripts/beads/bd bootstrap --yes`. Never `bd migrate` locally: two independent migrations fork the schema irrecoverably (#4259).
5. Commander sets `upgrade_state=frozen` and updates the pins in this file.

## 4. Storage mode: embedded, one DB per machine, one sync loop per machine

Measured on the Mac with 8 and 16 concurrent `bd` processes (create / list / `ready --claim`):

| Mode | Writers × ops | Wall | Succeeded | Failures |
|---|---|---|---|---|
| embedded | 8 × 20 | 53 s | 160/160 | none; 40 claims, no duplicate IDs |
| embedded | 16 × 10 | 52 s | 160/160 | none |
| server (`bd init --server`, auto-started per-project `dolt sql-server`) | 8 × 20 | 11 s | creates all OK | 9/40 claims returned `Error 1213 (40001): serialization failure` |
| server | 16 × 10 | 10 s | 134/160 | 26/53 claims exit 1 with `serialization failure` |

**Decision: embedded mode.**
- It serialises through a file lock, was about 5× slower, and produced zero failures.
- Server mode drops concurrent claims with exit 1, which would need a retry wrapper around every write, plus a server lifecycle on every machine.
- Revisit only if lock latency becomes the bottleneck.

**Consequences:**
- All git worktrees of a clone share the main clone's `.beads/embeddeddolt`: `bd where` from a worktree prints the main clone's DB, and a bead created in a worktree is visible in main (VERIFIED). So a 15–20-agent machine has **one** ledger DB.
- Writers serialise on Dolt's storage lock `.dolt/noms/LOCK`.
  - A bd write **blocks** while it is held, then succeeds. VERIFIED: waits of 10 s and 100 s, exit 0.
  - The `.beads/embeddeddolt/.lock` flock is taken only by `bd init` (SOURCE `cmd/bd/init.go:952`).
  - Reads (`bd list/show/ready`) do not wait.
- Agents do **not** run pull loops. Exactly one machine-level `scripts/beads/sync-loop` does (§5). Agents push only inside `scripts/beads/claim`.

## 5. Sync loop, staleness and sync convention

**One loop per machine:** `scripts/beads/sync-loop --repo <main clone>`, python3 stdlib, macOS and NixOS.
- **Single instance:** a flock on `<state>/sync-loop.lock`. A second copy exits 75.
- **Pull** every 60 s.
- **Push.** Every 5 s it checks for unpushed commits with a read-only `dolt sql … dolt_log('origin/main..main')` (0.085 s). It pushes when there are any, at most every 15 s. A rejected push pulls and retries up to 3 times. Never `--force`.
- **Backoff** on transport errors: 10 s doubling, cap `--max-backoff`.
  - Soak: the 300 s cap left B at lag 355 s for about 3.5 min after a 3-min outage ended.
  - **Use `--max-backoff 60`** in the service files; post-outage lag is then about one minute.
- **Wedge:** runs `ledger-recover` (§6). `<state>/RECOVERING` exists while it runs.
  - exit 4 (lost a race): retried within 3 s
  - exit 2: writes `<state>/HOLD`, sets `state=needs_operator`, exits 2. Service files do not restart on exit 2, and later starts refuse while HOLD exists.
- **Status** goes atomically to `<state>/status.json` (`~/.local/state/flashtex-beads`). Fields:
  - `state` (ok | degraded | recovering | needs_operator | stopped)
  - `last_pull_ok_utc`, `last_push_ok_utc`, `lag_seconds`, `unpushed_commits`
  - `consecutive_failures`, `backoff_seconds`, `pushes`, `push_rejections`
  - `recoveries`, `recovery_attempts`, `last_error`, `last_recovery`
- **Staleness:** `scripts/beads/ledger-status` prints `ledger as of <UTC> (lag Ns) on <machine>: FRESH|STALE|NEEDS_OPERATOR`. Exit 0/1/2; STALE also when no loop runs.
- **Services** (`scripts/beads/service/`):
  - `install-launchd.sh` + `dev.flashtex.beads-sync.plist` (macOS user agent, `KeepAlive.SuccessfulExit=false`)
  - `flashtex-beads-sync.service` (systemd `--user`, `RestartPreventExitStatus=2 75`)
  - `home-manager-snippet.nix` (proposal for GoKubar/nixos-config, not applied)

**Transport:** the ledger lives on the code repo's remote under `refs/dolt/data` (VERIFIED via `git ls-remote`).
- The first `bd dolt push` also creates a visible **branch** `refs/heads/__dolt_remote_info__` containing `DOLT_REMOTE.md` (VERIFIED).
  - It is acceptable (owner decision 3). Do not delete it; exclude it from branch-cleanup scripts.
- Plain PR flow is unaffected (VERIFIED, trial PR #1).
- Measured push latency, one small change: Mac about 7.7 s, NixOS PC about 5.5 s (VERIFIED). The slower machine loses push races under heavy write rates (§10).

**Fresh clone:** `scripts/beads/bd bootstrap --yes`, then `git config beads.role maintainer`, `chmod 700 .beads`.

**Reads:** before choosing work, claiming, closing and at checkpoints. Quote the `ledger as of` time from `ledger-status`.

## 6. Claim rule and collision recovery

**A claim counts only after `bd update <id> --claim` has been followed by a successful `bd dolt push` and a re-read shows you as assignee.** Use `scripts/beads/claim <id> --actor <agent>`, which implements exactly this:
- exit 0 `CLAIMED`
- exit 1 `LOST*` / `NOT_CLAIMABLE`
- exit 3 `NOT_COUNTED*` (transport failure, or the machine ledger is recovering)

On any non-zero exit it undoes its own local claim, so an uncounted claim is never pushed later. A rejected push is retried after pull and re-read, up to 6 times with jitter.

**What a cross-machine collision does** (VERIFIED, trial beads `trial-eey`, `trial-c3p`, `trial-72g`):
1. A and B both `--claim` offline. Both print `✓ Updated issue`.
2. The first pusher wins (`Push complete.`).
3. The loser's push fails: `hint: Updates were rejected because the tip of your current branch is behind`.
4. The loser's pull fails, every time: `Error: merge origin/main: merge conflicts in issues require operator resolution; merge aborted and working set restored` (exit 1). The whole machine ledger is wedged.

**Recovery candidates tested on a wedged copy** (VERIFIED on the trial repo):

| Candidate | Result |
|---|---|
| `bd doctor --fix` / `bd sql` | `'bd doctor' is not yet supported in embedded mode`; `'bd sql' is not yet supported in embedded mode` |
| `bd vc merge origin/main --strategy theirs` | `Error 1105: Merge conflict detected, @autocommit transaction rolled back` (exit 1) |
| dolt 2.3.3 CLI on `.beads/embeddeddolt/<db>`: `dolt merge origin/main` + `dolt conflicts resolve --theirs issues` | settles the claim (`our_assignee: agent-Y, their_assignee: agent-X`) **but drops the loser's other edits to the same row** (appended notes → `None`) → lossy |
| move `.beads/embeddeddolt` aside + `bd bootstrap --yes` | works, discards every unpushed write on the machine → last resort only |
| **`scripts/beads/ledger-recover`** (chosen) | lossless; details below |

**`scripts/beads/ledger-recover`, the chosen recovery.** The machine sync loop runs it automatically when it sees the wedge.
1. **Lock.** Hold the live DB's storage lock `.dolt/noms/LOCK` for the whole recovery.
   - bd processes block on it rather than fail. VERIFIED: a bd write waited 100 s and then succeeded.
   - So no write can interleave. Soak run 2 showed that writes landing mid-recovery re-conflict.
2. **Copy.** Take a pristine backup and a work copy.
3. **Settle in the work copy** with the pinned dolt CLI; bd's own pull has already run `DOLT_FETCH` (SOURCE `versioncontrolops/remotes.go`). Repeat up to 12 rounds:
   1. `dolt fetch origin`, `dolt merge origin/main`
   2. settle each conflicted `issues` row **column by column**:
      - a column only one side changed keeps that change
      - `notes` both appended: winner's notes + loser's suffix
      - `updated_at`: the later value
      - `metadata`: key-wise
      - any other column both changed (`assignee`, `status`, `started_at`, …): **remote wins**. The loser's value goes into `<state>/recoveries/<utc>.json` and a `claim-lost` message bead to the losing assignee.
   3. commit, then non-force `dolt push origin main`
4. **Restore.** Copy the settled `.dolt` back into the live DB file by file, keeping `LOCK` and `config.json`. Release the lock. Then `bd dolt pull` (a no-op check), `bd recompute-blocked`, notify, `bd dolt push`.
5. **Refusals.** Conflicts outside `issues`, schema conflicts, constraint violations or delete-vs-modify rows are refused: exit 2, live DB untouched, the loop writes `HOLD` and stops.
   - VERIFIED refusal path: a NixOS account without a dolt identity produced `error_live_untouched` → `needs_operator` → HOLD. The work copy now sets a local identity.

**Evidence (VERIFIED):**
- **Same-row edits kept.** Loser's unpushed note on the contested row, a metadata key, and a new bead all survived; winner's assignee kept.
  - Mac: `X sees: trial-p1i agent-X in_progress 'Z progress note' {'j': 'x', 'k': 'z', 'base': '1'}`, plus `msg: trial-79d claim lost: trial-p1i is held by agent-X`.
- **Both cross-machine directions** via `sync-loop --once`:
  - B lost: `B status: ok recoveries 1` … `A sees: trial-c3p agent-A 'B offline note'`, `A sees B follow-up: trial-dfa`.
  - A lost: `A after: trial-72g agent-B 'A offline note'`, `B sees A follow-up: trial-cab`.
- **Concurrent writer.** 20 `bd create` running on the loser during recovery: `lock_held_seconds 8.8`, `live_pull_after_restore: Pull complete.`, `zconc count: 20` on both clones.

**Protocol:**
- Agents never run `ledger-recover`. `claim` reports `LOST_collision_sync_loop_recovers` and refuses new claims while `<state>/RECOVERING` or `HOLD` exists.
- Keep code on the git branch. Unpushed ledger notes survive recovery, or are recorded.
- Never `--force`.

## 7. Messaging convention

- **`-t message` does not work cross-machine.** In 1.2.2 `message` is an infrastructure type routed to the local, dolt-ignored `wisps` table. B never saw `trial-wisp-bde` (VERIFIED). `bd mail` only delegates to an unconfigured provider.
- **Convention** (VERIFIED A→B→A, drill 2026-09-13 07:27–07:35Z, beads `trial-ljie`, `trial-7pfa`, `trial-ljie.1`):
  - **Send:** `bd create "<subject>" -t task --assignee <to> -l msg,to:<to>,from:<me>,thread:<root-id> --description "<body>"`. A root message labels itself `thread:<own id>`. Then push.
  - **Inbox:** `scripts/beads/inbox --agent <me>` lists open `msg` beads labelled `to:<me>`, `to:all` or `to:machine:<alias>`, headed by the ledger freshness line.
  - **Reply:** a new message with the same `thread:<root>` label, plus `bd dep add <reply> <parent> --type relates-to`.
    - A `--parent <msg>` reply also syncs, but **inherits the parent's `to:`/`from:` labels**. The drill reply showed `to=agent-A,agent-B`. Always pass `--no-inherit-labels` with `--parent`.
  - **Thread by ID after sync:** `scripts/beads/inbox --thread <root>`, `bd dep list <root> --direction=up` (shows both the `relates-to` and `parent-child` replies) and `bd children <root>` all worked on the other machine. `bd show <root> --thread` showed only the root, so do not rely on it.
  - **Ack:** `bd close <id> -r read`, then push. The other machine saw the closes.
- **Latency:** loop poll (≤60 s) plus push/pull (5.5–7.7 s each).
  - The drill's 427 s round trip was dominated by slow bd operations on a long-lived test clone. A `bd dolt pull` there took 127 s after the soak; fresh clones were fast.
  - Treat clone age and `refs/dolt/data` growth as a performance item to watch (BELIEVED cause: local git-remote-cache size).
- **Other sync facts (VERIFIED):** `bd kv`, `--set-metadata`, `bd set-state` event beads and labels, and `types.custom` all reach the other machine.

## 8. Low-usage signalling and reallocation

**Agent, the moment its tool reports a usage warning, a limit, or a low reading** (VERIFIED drill, agent bead `trial-1gte`, task `trial-ee3o`):
1. Push code to the task branch. Then `bd update <task> --append-notes "handoff: branch=<b> sha=<sha> tests=<…> next=<…>"`.
2. `bd set-state ft-agent-<id> capacity=low --reason "<exact tool output>"`. This creates an event bead and the label `capacity:low`. Also `bd update ft-agent-<id> --set-metadata usage_verbatim='<json>' --set-metadata usage_source='<command>' --set-metadata observed_utc=<utc>`.
   - Example recorded verbatim from `~/.claude/bin/claude-usage --json`: `{"five_hour":{"utilization":69.0,"resets_at":"2026-09-13T09:30:00.968565+00:00",…}}`.
3. If it cannot finish: `scripts/beads/claim <task> --actor <me> --release`. This is an atomic unclaim; plain `bd assign <task> ""` also works when no recovery can be running. Then `bd update <task> --add-label handoff-ready`, and push.

Report the exact tool text only. Never estimate or sum unlike resources.

**Commander (scripted, no model call), VERIFIED from the other machine:**
- `bd list -l capacity:low` showed the agent bead with the verbatim usage.
- `bd list -l handoff-ready --status open` showed the task.
- `bd assign <task> <agent>` plus `--set-metadata reassigned_from=<id>` and `--remove-label handoff-ready`, then a message `-l msg,to:<agent>,thread:<task>`, then push.
- The first machine then saw `assignee agent-B2`, `meta {'reassigned_from': 'agent-A'}`, and `inbox --agent agent-B2` listed the message.

Grants still come from `coordination/RESOURCES.md`.

## 9. Commander authority and failover

**Record:** the bead `ft-authority` (type `decision`). Metadata:
- `commander_id`, `machine`, `authority_state` (active | handoff | vacant)
- `epoch`, `claim_mode`, `claim_evidence`, `previous_commander_id`, `designated_successor`
- `updated_utc`, `heartbeat_utc`
- `bd_pin`, `dolt_pin`, `upgrade_state`

Tool: `scripts/beads/authority show | heartbeat | handoff | claim`. The wrapper reads `machine` and `authority_state`.

**Claim** (unchanged AGENTS.md gates; a stale heartbeat is never sufficient):
1. `authority claim --actor <id> --machine <alias> --mode handoff|verified-termination|user-directed --evidence "<verification or quoted instruction>"`
2. It pulls, checks eligibility and writes `epoch+1` (non-force push with bounded pull/retry). Then it pulls again and re-reads.
   - `CLAIMED` only when the re-read shows this actor, this machine and the new epoch.
   - A lost collision gives `LOST_collision_stand_down`; the sync loop recovers with remote-wins.

**Drill** (VERIFIED on the trial repo, bead `trial-authority`, A = mac-m5pro-kabir, B = NixOS PC):

| Step | Observed |
|---|---|
| A holds epoch 1, heartbeat | `{"cmd": "heartbeat", "push": "ok"}`; wrapper on A: `bd migrate permitted on Commander machine mac-m5pro-kabir (beads:trial-authority)` |
| B reads, wrapper on B | `heartbeat_age_seconds: 13`; `bd migrate refused: this machine is 'nixos', Commander machine is 'mac-m5pro-kabir'` |
| A "dead"; B claims | `{"result": "CLAIMED", "epoch": 2, "previous": "commander-A", "mode": "user-directed"}`; wrapper on B: `permitted on Commander machine nixos` |
| A resumes | re-read shows `commander-B / nixos / 2`; wrapper on A `refused: … Commander machine is 'nixos'`; `authority heartbeat` → `commander-A is not the Commander` (exit 1) |
| Concurrent epoch-3 claims | A `LOST_collision_stand_down`; B `CLAIMED epoch 3` |
| **Bug found** | after recovery the record read `commander-B2` with `machine: mac-m5pro-kabir`: the key-wise metadata merge mixed the two claims, which would grant migrate rights to the wrong machine. **Fixed:** if any metadata key conflicts, the remote object wins whole (unit-checked: `{'commander_id': 'commander-B2', 'machine': 'nixos', 'epoch': 3}`). |
| Concurrent claims after the fix | A `LOST_collision_stand_down`, B `CLAIMED epoch 4`; after `sync-loop --once` both machines read `{'commander_id': 'commander-B4', 'machine': 'nixos', 'epoch': 4, 'authority_state': 'active'}`; wrapper on A `refused: … Commander machine is 'nixos'`, on B `permitted on Commander machine nixos` |

## 10. Verified vs believed

| Claim | Status |
|---|---|
| Identical `bd version 1.2.2` / `dolt version 2.3.3` on macOS arm64 and NixOS x86_64, sha256-verified | VERIFIED |
| NixOS: pinned bd runs via the glibc-loader shim; nix-ld not yet enabled on the PC (`/lib64/ld-linux-x86-64.so.2 -> …stub-ld…`, direct run `Could not start dynamically linked executable`) | VERIFIED; installer prefers direct execution when nix-ld makes it work: BELIEVED (untestable until the owner rebuilds) |
| Create/claim/close/blocked/ready round trips across machines | VERIFIED |
| Offline creates merge with no ID collision or loss | VERIFIED |
| Claim collision: first pusher wins; loser wedged until recovery | VERIFIED |
| `ledger-recover` lossless recovery, both directions, same-row edits kept, concurrent writer blocked | VERIFIED (§6) |
| `bd init --skip-agents --skip-hooks` leaves AGENTS.md/CLAUDE.md byte-identical and `core.hooksPath` unset; commit contains only `.beads/{.gitignore,README.md,config.yaml,interactions.jsonl,metadata.json}` (+ root `.gitignore` lines); without `--skip-hooks` `core.hooksPath` becomes `.beads/hooks` | VERIFIED (fresh repo with no `.beads`) |
| `bd init` in a repo whose committed `.beads/config.yaml` has `sync.remote` bootstraps from that remote instead (ignores `--prefix`) | VERIFIED |
| Messaging A→B→A with threads; low-usage signal and cross-machine reassignment | VERIFIED (§7, §8) |
| Commander failover drill, wrapper permission moving with the claim | VERIFIED (§9) |
| Sync loop status/staleness, backoff, HOLD on refusal, single instance | VERIFIED |
| **Soak (16 + 8 agents, 32 min, 7 collisions, 2 outages, loop kill -9)** | **PASSED run 4** on the ledger-mutex fix (run 3 failed; see soak report) |
| Sync on Daniel's / Jaysen's machines | BELIEVED (runbook) |

### Soak report (trial repo, 2026-09-13)

- **Run 1** (06:08Z) was aborted after about 1 min. `claim` retried a rejected push only once, and Mac claims livelocked against the NixOS PC's pushes. Fixed with 6 jittered attempts.
- **Run 2** (06:13Z, 60–150 s work cycles) failed.
  - A's recovery kept losing push races (`remote moved during recovery`).
  - A commit-on-clean-merge bug put the loop into HOLD.
  - Then, even after the settle pushed, the live pull re-conflicted, because agents kept writing to the contested rows mid-recovery.
  - Fixed by holding bd's storage lock for the whole recovery, settling in a copy and restoring in place.
- **Run 3** (06:36Z, 180–360 s cycles), result:
  - 24 agents; 80 counted claims and 80 closes; 21 follow-ups created, none missing.
  - Ledgers identical: 285 beads each, same dolt HEAD `84h28qe1r82g4ilp1vob2t5nj9q63c1s`.
  - 7/7 deliberate collisions had exactly one winner. 7 recoveries on A and 3 on B, all successful. Survived 2 outages and a `kill -9` of B's loop.
  - Max lag: A 557 s (loop `ok` but a pull/push call stalled; BELIEVED transport stall hitting the old 300 s subprocess timeout, now 120 s), B 410 s (post-outage backoff; service files now `--max-backoff 60`).
  - Storage growth: `.beads` A 166→447 MB (includes run-2 churn), B 39→89 MB. `refs/dolt/data` pack 5.95→12.99 MiB (repacked).
  - **FAILED: `trial-g7of` claimed and closed by two agents** (`r3A-13` at 06:46/06:50, `r3B-5` at 06:55/07:01).
  - Root cause, from `bd history`: B's `claim` undo checked "still mine?" and then wrote `status=open assignee=""`. The write blocked on the storage lock held by a recovery and landed after the restore (`02:50:34.851 (None, open)`), reopening a task its owner had closed. The machines then flip-flopped until the reopen won, and a second agent legitimately claimed it.
  - Also found: the recovery record's detailed rows were overwritten by the summary (fixed).
- **Fix** (after run 3): a machine ledger mutex `<state>/ledger-write.lock`, held by `claim`'s check+undo (and `--release`) and by `ledger-recover`.
  - Deterministic reproduction with test-only delays, same ordering as run 3:
    - without the mutex: `FINAL Z: None open None   X: None open None` (close reverted)
    - with it: `FINAL Z: agent-X closed done by agent-X   X: agent-X closed done by agent-X`
- **Run 4** (07:48Z, same load as run 3; fresh clones; `--max-backoff 60`, 120 s transport timeout; owner-approved), **PASSED**:
  - **Load:** 24 agents; 87 counted claims, 87 closes; 31 follow-ups created, 0 missing.
  - **Ledgers identical:** 403 beads each, same dolt HEAD `nl43jlcadej3iciuh3fhdqjkjce1718e`.
  - **Double claims/closes:** 0 tasks claimed by two agents, 0 closed by two agents, 0 log-vs-ledger disagreements.
  - **Collisions:** 7/7 deliberate collisions had exactly one winner.
  - **Recoveries:** A 10/10 `recovered`, B 4/4 `recovered`. No HOLD, no refusal, no error.
  - **Faults, all injected OK:** 2 B outages (3 min, 2 min), `kill -9` of B's loop plus restart.
  - **Lag** (10 s samples):
    - A: max 381 s, p50 98 s, p95 290 s
    - B: max 258 s, p50 32 s, p95 157 s
    - The lag comes from sustained cross-machine write load and push-race retries at these simulated rates. FlashTeX task-boundary rates are far lower.
  - **Storage:** `.beads` A 50→129 MB, B 76→129 MB. `refs/dolt/data` pack 14.78→22.88 MiB over run 4; the trial repo carries all 4 runs.
  - Evidence: `scripts/beads/tests/soak_verify.py` output `"PASS": true`.
- **Throughput finding:** at 60–150 s cycles (about 0.3 claims/s over 24 agents) the slower pusher (Mac 7.7 s vs PC 5.5 s per push) starves. FlashTeX task-boundary rates are far lower, but this is the measured ceiling.
