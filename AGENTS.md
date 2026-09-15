## Context compaction at natural checkpoints

Latest explicit user policy: at a natural checkpoint, compact when actual context
usage exceeds 80%; at 60–80%, compact when much of the retained context is irrelevant.
Use reliable runtime telemetry and supported native compaction controls. If usage
telemetry or a compaction command is unavailable, state that limitation; never
invent percentages, force a session restart, or treat compaction as agent failure.

Before compaction save a durable checkpoint containing task ID/revision, exact
branch and worktree, current changes and dirty files, tested SHAs and evidence,
pending commands/publication journals/messages, next steps, ownership boundaries,
current staffing and billing restrictions. After compaction read that checkpoint,
current authority and assignments, reconcile any pending operation, and continue
without replaying uncertain paid calls or publications. Leads must propagate this
policy to their existing children. Do not reactivate drained workers just to compact.

> LATEST USER STAFFING OVERRIDE: retain THREE Astra engineers (root runtime
> performance, font-resources, rendering-core) plus the sole Commander. Bridge,
> project-index and edit-ledger may finish their current task, then STOP. Their
> queues are paused; do not restart, replace or reallocate them. This supersedes
> every older local seven-agent reset below. Remote staffing remains unchanged.

> LATEST USER OVERRIDE: Jaysen cooldown is lifted immediately. Restore the
> prepared15 independent engineering lanes NOW after verifying usable route.
> Daniel retains transferred heavy paths. Old08:21 wait instructions below are
> superseded. Never duplicate an already running session or purchase usage.

> LATEST USER STAFFING RESET: this host now has SEVEN active total: sole Astra
> Commander plus SIX product engineers (root assistant-context, font-resources,
> rendering-core, conversion-jobs, project-index, edit-ledger). The three paused
> product agents are explicitly reactivated in their preserved worktrees. This
> supersedes all older local4/paused3 wording below. No additional remote expansion
> follows from this change; Daniel/Jaysen existing explicit allocations continue.
> User reports plan reset; do not invent a numeric remaining balance or new funds.

> LATEST COOLDOWN: until 2026-09-12T08:21:08Z Jaysen heavy work is suspended;
> Daniel may take exact fenced handoffs after registration/ACK. Jaysen small tasks
> may use his existing remaining usage credits, explicitly authorized for this
> machine only. At that time both machines receive independent full queues,
> subject to verified usable routes/reset; no purchases or duplicate ownership.
> Commander remains linux-primary while usable; fallback is dynamically selected
> from fresh verified comparable remaining capacity, never fixed to Jaysen.

> ADDITIONAL USER AUTHORIZATION: Daniel's new Claude20x machine is allocated16
> engineering lanes FT030–045 (registration pending), without reducing other
> staffing. This is the explicit Daniel-only remote expansion exception. Existing
> billing restrictions, local4 cap and paused workers remain unchanged.

> LATEST STAFFING OVERRIDE: this computer has FOUR active agents total: sole
> commander orchestrator-astra, root preview-controller, compiler_corpus fonts,
> supervisor_api_review rendering. Bridge-context, project-index and edit-ledger
> agents are paused; preserve all dirty/published work. Never revive or replace
> them automatically and do not compensate by increasing staffing elsewhere.
> This overrides older six-engineer staffing text below. Other authorization
> and explicit-user-stop-only project continuity remain unchanged.

## Model and effort selection — all machines

Tokens come from shared Max-plan quotas; spend the strongest model where mistakes
are expensive. Dispatch by agent type (`.claude/agents/`), which sets the defaults:

| Work | Agent type | Model / effort |
|---|---|---|
| Engine correctness (math, TikZ, floats, hyphenation, line breaking, perf) | `engine-engineer` | Opus / high |
| Coordination tooling (Beads, contracts/, Agent Mail, failover) | `coordination-tooling` | Opus / high |
| Generated docs, drift gates, resource registers, PR write-ups | `docs-writer` | Sonnet / medium |
| Website/accessibility QA, screenshot sweeps, acceptance checks | `qa-reviewer` | Sonnet / medium |
| Read-only search, log/JSON/CI triage | `repo-scout` (or Explore) | Haiku / low |

- The Commander session stays on Opus: dispatch, integration and failover decisions.
- Default to `high`, not `max`. Use `max` only for a stuck, high-stakes problem after
  two serious attempts (e.g. a pdflatex line-break mismatch), and say so in the report.
- Subagents inherit the parent model unless a type or override says otherwise; a
  general-purpose spawn for QA/docs/search work should pass `model: sonnet`/`haiku`.
- When a machine's quota runs low, first move QA, docs and search down to Sonnet or
  Haiku, then defer them; cut Opus from engine work last. Report low quota immediately
  through the current coordination channel so the Commander can reallocate.

# FlashTeX: required agent collaboration protocol

These instructions apply to all work in this repository, across agents, computers,
and sessions. Read this file before planning or editing. Follow it throughout the
task, including after context compaction or resuming a session. User instructions
and higher-priority platform instructions take precedence. The agent onboarding
notes that used to live in the root README (start prompt, coordination CLI, where
handoffs are published) are in [docs/agents/README.md](docs/agents/README.md); the
root `README.md` is now the human-facing project page.

## Current user authorization — September 12

The user explicitly authorizes continuous autonomous project improvement until
they explicitly stop it. Verified completion starts another improvement cycle;
it is never an automatic stop condition. The
former 10am deadline and stabilization window are no longer stop conditions.
Do not stop solely because a task time estimate elapsed. Keep individual model
calls bounded, publish recovery evidence on failures, and continue other eligible
work. Do not wait indefinitely for a sleeping user's permission on actions already
authorized. Actual platform denials, missing authentication, and unapproved billing
remain blockers to report; never claim they have been bypassed or granted remotely.
Read `docs/autonomous-workers.md` for the executable startup and task loop.

## Designated standby exception

The user additionally authorized ONE NEW Opus standby on Jaysen mac-m1max-a,
`orchestrator-jaysen-opus`, as a candidate for eventual Commander takeover after verified
termination and stopped publishers. It stays read-only while Astra is active;
this does not increase engineering staffing or revive paused agents. Read
[the exact revival prompt and operational gaps](docs/commander-failover.md).

## Command and dispatch

Current sole orchestrator: **orchestrator-astra**, hosted agent handle
`/root/runtime_validator`, on linux-primary, explicitly selected by the user.
All organizational work, resources, queues and global integration belong to this
role. The three active hosted product engineers perform product work only. The root agent
continues product engineering and does not concurrently write main/control files.
Read `coordination/authority.json` before every global mutation; obsolete role names
below are historical. Sol explicitly handed over after stopping publication jobs.


The user designated the primary Codex agent on `linux-primary` as **Commander**,
responsible for orchestration, task/resource assignment, and integration of main.
Read `ORCHESTRATION.md`, `coordination/COMMANDER.md`, and
`coordination/COMMANDER-RESUME.md` at startup and after
compaction. Register capabilities in your own handoff; the Commander maintains
`coordination/ROSTER.md` and `coordination/TASKS.md`. Acknowledge your assignment
revision before implementation and publish changes, evidence, ETA, resource
state, and reviewed peer revisions at the required checkpoints. Task rows do not
prove agents are running. Follow the orchestration plan's ownership and reporting
rules; do not independently assign overlapping work or spend another agent's grant.

The protocol is executable: read `docs/coordination-cli.md` and use
`python3 scripts/coord.py checkpoint` at each checkpoint. Structured registrations
and reports live in `coordination/agents/<id>.json`; published assignments live in
`coordination/assignments/<task>.json` on main. Fetch/display never counts as review.
Use `ack` and `report --review ... --adaptation ...` after actually reading changes.
Use `publish` for guarded staged commits and task-branch pushes. After an observed
Cursor usage limit, the explicit user override below permits direct current-agent
Git commits with truthful provenance and the authenticated local user as coauthor.
Do not wait for exhausted Cursor quota. Other missing-authentication cases must
use an already-authorized execution route or publish a concrete recovery blocker.

Before starting a task, claim it with `coord.py claim <task-id> --actor <id> --machine
<alias>` on the dedicated `coordination-claims` branch (never main); it refuses with
`LOST: held by <actor> since <utc>` if another actor already holds it. Call `coord.py
claims --touch <task-id> --actor <id>` at each checkpoint to keep the claim fresh, and
`coord.py release`/`close` it (the latter with `--gh-ref`) when you stop or merge. See
[Claims](docs/coordination-cli.md#claims) for the full mechanics and exit codes.

There is exactly one active Commander. A successor may claim command only after
either (a) the current Commander publishes an explicit quiesced handoff naming that
successor and confirms its publication/integration jobs are stopped, or (b) the
successor independently verifies the exact Commander process/session terminated
and every Commander publication/dispatch/integration job is stopped. Silence, a
missed heartbeat, stale Git state, timeout, quota suspicion, or network failure is
never proof the Commander is offline. The successor fetches and pins current main,
selects one leader identity, publishes an atomic non-force authority claim, and
rereads that claim immediately before every main/control write. An old Commander
that resumes must reread authority and remain quiesced unless explicitly handed
command again. See the copyable revival prompt in `docs/autonomous-workers.md`.

Every blocked worker opens a GitHub recovery issue with task/revision, exact branch
and SHA, failing command, non-secret error, process state, resource state, and any
possibly in-flight call. The Commander triages each open recovery issue, assigns
one or more eligible non-overlapping resolvers after rereading every machine's
latest resource report, verifies the fix, and only then closes the issue. A comment
or task row is not proof that a local worker started; require an ACK plus actual
PID/session or equivalent live-process evidence.

## Current Claude Max authorization

The user explicitly requested more tasks and subagents on the 20x Claude Max plan
on mac-m1max-a. That machine's parent `mac-claude-a` may use its available plan
allowance for assigned project work and supervise `mac-pdf` and `mac-validation`
in separate worktrees. These share one account quota; they are not independent
balances. This scoped authorization supersedes older blanket Claude prohibitions
for that plan only. No overages/purchases or unrelated protected account use.

On `linux-primary`, Claude is API-only under the latest user instruction.
Never launch the local Max subscription, OAuth/keychain, or extra-usage route.
Opus work may start only after an existing funded API credential, actual available
credits, a provider-side cap, and a bounded project grant are verified. This grants
no purchase, new charge, auto-recharge, or overage. The Max 20x authorization above
remains confined to `mac-m1max-a`.

## Resource, deadline, and recovery rules — read first

Read `docs/INDEX.md`, `coordination/PROJECT.md`, and `coordination/RESOURCES.md`
at startup and after compaction. Read `docs/agent-operations.md` before delegating,
using a paid CLI/API, changing resource allocations, or making commits.

- Never use the user's protected personal Claude subscription allowance or incur
  unapproved charges. An existing Claude login is not authorization. New Claude work
  is blocked until a separate approved funding source and allocation are verified.
- The £75 identified by the user is Pro/Max extra-usage credit. Their included
  plan allowance must remain untouched. No extra-credit-only execution route has
  been verified, so do not run Claude tasks through that subscription.
- Subscription prices, usage multipliers, API balances, and different currencies
  are separate resources. Do not add them together or invent remaining balances.
- Every agent and child agent inherits the same deadline and spending restrictions.
  Child allocations come out of the parent's allocation; they are not extra money.
- Update your handoff with measured progress, next acceptance gate, remaining-time
  estimate/range, blockers, resource pool/allocation, usage evidence, and next step.
- Use bounded experiments with a stated expected benefit and a time/cost limit.
  Persist through setbacks by changing tactics; do not repeat failed attempts
  indefinitely, hide blockers, or relabel incomplete requirements as completed.
- Preserve 20% of confirmed remaining time and allocatable budget for integration
  and verification unless the user specifies another reserve. No funded allocations
  are active until the actual available resources are confirmed.
- Before compaction or handoff, save a concise resumption packet with branch/SHA,
  dirty files, exact next commands, tests, decisions, dependencies, and resource
  state. After resuming, verify it against Git rather than trusting stale notes.
- Use the absolute deadline and fixed final-verification window in
  `coordination/PROJECT.md`; never restart the clock after compaction. Unconfirmed
  account totals must remain unknown.

### Commit identity and truthful provenance

**Latest explicit user override (all computers):** when Cursor usage limits are
hit, the current implementing agent may execute Git commits directly. Do not wait
for Cursor quota, purchase more usage, or mislabel execution. Use the actual agent
identity and truthful `Implementation-Agent` / `Commit-Executor` trailers, plus
`Co-authored-by` for the GitHub user authenticated on that computer. Preserve the
mac-m1max-a primary-author exception. This supersedes older mandatory-Cursor wording
only for the observed Cursor-limit fallback; actual Cursor commits remain truthful.
The local Cursor limit was confirmed by terminal ActionRequiredError on the Astra
authority and bridge recovery publication attempts. Product work and publication
continue through direct-agent Git execution under this user authorization.


Commits actually executed by Cursor use the project automation identity
`Cursor <cursor@flashtex.invalid>` (except the Mac primary-author rule below).
Direct fallback commits use the actual implementing agent identity. This is a project label with a deliberately
non-deliverable address, not a verified Cursor employee or vendor account.
Use repository-local configuration or per-command identity; do not change global
Git identity or rewrite existing commits.


**Machine exception — `mac-m1max-a` (the user's own Mac):** the user directed
that everything committed and pushed from this computer carries the user as the
primary Git author, using the repository's configured `user.name`/`user.email`
(`jay3332`). Do not set the Cursor author on commits from this machine. The
The all-computer direct fallback remains available after an observed Cursor limit;
truthful trailers are still required here: name the implementing agent,
name the actual commit executor, and add `Co-authored-by: Cursor
<cursoragent@cursor.com>` only when Cursor CLI actually executed the commit.
