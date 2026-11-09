# Working with agents

Relocated verbatim from the root `README.md` (its former "Working with agents"
section) so the landing page can describe the product. The relative link targets
were adjusted for this directory, and the Commander and master-plan pointers were
brought up to date on 2026-11-01; the rules remain governed by
[AGENTS.md](../../AGENTS.md) and the governing
[engine-v2 design](../design/engine-v2/DESIGN.md).

**Executable tools:** [coordination CLI](../coordination-cli.md).
Run `python3 scripts/coord.py checkpoint` to fetch updates and discover assignments.
Registration, reports, explicit acknowledgements, dispatch, guarded Cursor
publication, and a deadline-bounded watcher are implemented with no Python dependencies.

Claim before starting a task (`coord.py claim <task-id> --actor <id> --machine <alias>`),
call `coord.py claims --touch <task-id> --actor <id>` at each checkpoint, and close it
on merge (`coord.py close <task-id> --actor <id> --gh-ref <url>`) — see
[Claims](../coordination-cli.md#claims). Claims live on the dedicated
`coordination-claims` branch; never claim on main.

Start with the [orchestration master plan](../../ORCHESTRATION.md) and the
[Commander bulletin](../../coordination/COMMANDER.md). The Commander is `kabir-claude`
(mac-m5pro-kabir), by the owner's transfer (DESIGN.md header; Appendix A is its master
prompt); every other session is an engineer under it. Workers register, acknowledge
bounded assignments, and publish progress to their own branches; Commander integrates main.

**Every agent must read [AGENTS.md](../../AGENTS.md) before planning or editing.** It
defines task ownership, frequent Git checkpoints, adaptation to other agents'
changes, handoffs, and integration rules for all computers and sessions.

Start each agent with:

> Read the root AGENTS.md and follow its collaboration protocol throughout this
> task. Fetch and inspect current main and relevant peer branches before editing.
> Publish your task ownership and handoff on your own branch. At each checkpoint,
> review relevant changes and adapt your implementation before continuing.

The [engine-v2 design](../design/engine-v2/DESIGN.md) is the governing design and
the source of truth for goals and phases. The
[original product master plan](../../latex-master-plan.md) is the founders' first plan:
its compiler written from scratch was retired at P5 (S7, 2026-11-01). Active agents publish their own handoffs under `coordination/` on
their task branches; those updates can be read before they merge into main.

Use [the shared knowledge index](../INDEX.md) to recover context after a restart
or compaction. Check [project deadlines](../../coordination/PROJECT.md) and
[resource permissions](../../coordination/RESOURCES.md) before paid delegation. Personal
Claude usage is prohibited; unverified funding must never fall back to it.

Claude Code imports the shared instructions through `CLAUDE.md`; Cursor has a
small always-applied rule under `.cursor/rules/`. These are discovery aids, not a
replacement for an external supervisor or provider-side spending controls.

## Shared build slots: `scripts/agents/slot.sh`

The Mac build locks and the NixOS PC job slots are shared by every lane. Take them
with `scripts/agents/slot.sh` so each slot carries an owner, a pid and a time limit,
and so waiters queue fairly instead of polling `mkdir`.

| Pool | Slots | Where |
|---|---|---|
| `mac-build` | 3: `mac-build.lock`, `mac-build.lock2`, `mac-build.lock3` | this Mac |
| `pc` | 4: `pc-slot1` .. `pc-slot4` | the PC, over `ssh nixos` |

The slot directories are the ones lanes used to `mkdir` by hand, in
`~/.cache/flashtex-agents/commander/` on each host, so old-style locks and
`slot.sh` share one pool.

```
# Hold a slot for exactly one command (preferred; released on exit or Ctrl-C):
scripts/agents/slot.sh run mac-build <lane> --max-minutes 20 -- nice -n 19 cargo check -p <crate>
scripts/agents/slot.sh run pc <lane> --max-minutes 45 -- systemd-run --user --scope --slice=flashtex.slice -p MemoryMax=10G nice cargo test ...

# Or acquire and release around several commands (prints the slot name):
scripts/agents/slot.sh acquire pc <lane> --max-minutes 30 --priority 60 --pid <long-lived pid>
scripts/agents/slot.sh release pc <lane>

scripts/agents/slot.sh status            # both pools, owners, ages and the queue
```

- **Owner file.** Each slot dir holds `owner` with `lane`, `pid`, `host`, `start`,
  `start_utc`, `max_minutes` and `priority`. `release` removes only slots whose owner
  `lane` matches.
- **Which pid.** `acquire` records its caller's pid unless `--pid` is given. Agent
  tool calls each run in a fresh shell that exits at once, so from an agent use
  `run`, or pass `--pid` of a process that lives as long as the work. Otherwise the
  slot looks dead and the next waiter breaks it. For the `pc` pool, `run` executes
  the command on the PC, so the PC can see the pid.
- **Breaking.** A slot is broken only when its pid is on the same host and dead, or
  when it is past `max_minutes` + 10 and its pid can't be seen (another host, or a
  hand-made lock with no owner file, whose age is its mtime and whose limit is 60
  minutes). A live pid on the same host is never broken. Every break is appended to
  `slot-breaks.log` in the slot root with the old owner.
- **Queue.** Waiters drop a ticket in `<pool>.queue/`. The highest `--priority`
  (default 50) goes first, then the earliest waiter. Tickets of dead waiters are
  dropped. `--wait-minutes N` gives up after N minutes (exit 3).
- **Portability.** bash 3.2 and BSD `stat` on macOS, GNU on NixOS. The `pc` pool
  pipes the script to the PC over ssh, so the PC needs no checkout.
  `SLOT_PC_HOST=local` (or `SLOT_LOCAL=1`) uses the pool on the current host.
- **Self-test:** `/bin/bash scripts/agents/slot-selftest.sh` uses a temporary root
  and no ssh; it passes on macOS bash 3.2 and on the PC.
