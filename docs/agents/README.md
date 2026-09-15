# Working with agents

Relocated verbatim from the root `README.md` (its former "Working with agents"
section) so the landing page can describe the product. Only the relative link
targets were adjusted for this directory; the rules are unchanged and remain
governed by [AGENTS.md](../../AGENTS.md).

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
[Commander bulletin](../../coordination/COMMANDER.md). The user designated the primary
Codex agent on `linux-primary` as Commander. Workers register, acknowledge bounded
assignments, and publish progress to their own branches; Commander integrates main.

**Every agent must read [AGENTS.md](../../AGENTS.md) before planning or editing.** It
defines task ownership, frequent Git checkpoints, adaptation to other agents'
changes, handoffs, and integration rules for all computers and sessions.

Start each agent with:

> Read the root AGENTS.md and follow its collaboration protocol throughout this
> task. Fetch and inspect current main and relevant peer branches before editing.
> Publish your task ownership and handoff on your own branch. At each checkpoint,
> review relevant changes and adapt your implementation before continuing.

See the [product and engineering master plan](../../latex-master-plan.md) for product
requirements. Active agents publish their own handoffs under `coordination/` on
their task branches; those updates can be read before they merge into main.

Use [the shared knowledge index](../INDEX.md) to recover context after a restart
or compaction. Check [project deadlines](../../coordination/PROJECT.md) and
[resource permissions](../../coordination/RESOURCES.md) before paid delegation. Personal
Claude usage is prohibited; unverified funding must never fall back to it.

Claude Code imports the shared instructions through `CLAUDE.md`; Cursor has a
small always-applied rule under `.cursor/rules/`. These are discovery aids, not a
replacement for an external supervisor or provider-side spending controls.
