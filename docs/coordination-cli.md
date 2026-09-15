# Executable coordination protocol v1

Implementation: `scripts/coord.py`, Python 3.9+ standard library on macOS/Linux.
No API keys or model calls are needed except the explicit `publish` command.

## Join

Choose a unique ID, fetch main, and create `agent/<id>/register` from current main
in a clean clone/worktree. Then:

```sh
python3 scripts/coord.py register --id worker-a --machine mac-a --tool Codex --capability swift --capability xcode
git add coordination/agents/worker-a.json
python3 scripts/coord.py publish --allocation YOUR_AUTHORIZED_CURSOR_GRANT --implementation Codex -m 'coordination: register worker-a'
```

Use real IDs/grants. `register` writes only your JSON report and does not switch
branches, stage files, spend money, or publish. Only your own `agent/<id>/...`
branch may carry updates. Markdown registrations are discovered as legacy reports
for manual review; they are not silently converted to active structured workers.

## Check, read, adapt, acknowledge

```sh
python3 scripts/coord.py checkpoint
```

This fetches and outputs JSON containing new branch tips, changed file paths,
structured/legacy reports, stale/duplicate warnings, and authoritative assignments
from `origin/main`. Local state lives under `git rev-parse --git-path flashtex`;
it works in linked worktrees. The command never merges or modifies tracked files.
Observed commits are NOT marked reviewed. A failed fetch preserves the prior
snapshot with `fetch_ok: false`; do not rely on that snapshot as current.

Read the actual task and relevant diffs. On the branch named in the assignment,
carry your registration forward and acknowledge the exact published revision:

```sh
python3 scripts/coord.py ack --id worker-a --task FT-003 --revision 1 --adaptation 'Accepted the Swift shell task; shared messages come from protocol v1.'
python3 scripts/coord.py report --id worker-a --state in_progress --summary 'Source editor builds; preview pending' --next 'Connect the compile-result fixture' --eta 10 20 35 --usage unknown --evidence 'Subscription dashboard not available'
```

Use `report --review origin/main=FULL_SHA --adaptation '...'` to record a real
review and response. Publish the updated report with a coherent work checkpoint.
An assignment acknowledgement is tied to a revision and the source main commit;
new dispatch revisions require new acknowledgement. No automatic ACKs exist.

## Commander dispatch

On a Commander branch containing current main:

```sh
python3 scripts/coord.py dispatch --task FT-003 --agent worker-a --branch agent/worker-a/mac-shell --objective 'Build a native source/preview shell' --path apps/mac --acceptance 'Build with Xcode and render the versioned fixture' --minutes 45 --allocation APPROVED_GRANT
```

Dispatch writes `coordination/assignments/<task>.json`; it is provisional until
Cursor commits it and Commander publishes it to main. Active path overlaps and
silent transfer of active ownership are rejected. Commander verifies registrations,
funding, and capability before issuing the command. A grant string is an audit
reference, not a billing enforcement mechanism.

## Publication

Stage ONLY intended changes, resolve/preserve unstaged files, and call `publish`.
It actually invokes Cursor CLI, then requires exactly one new commit on the same
branch with the original staged tree, Cursor author/committer, truthful trailers,
and a clean working tree. Only then does it non-force push that task branch.
It never pushes main. Commander reviews/integrates the candidate separately.

On a timeout, rejected push, or validation failure, inspect actual Git state and
the remote before retrying. Cursor may have committed even when its process failed.
Do not rerun a paid session or overwrite history blindly.

### If a worker cannot use Cursor

Do not make another non-Cursor commit. Prepare a `git diff --binary` patch including
intended new files (use the index to include them), validation results, and a
handoff. Submit it as a GitHub issue under this repository so Commander can apply
it in an isolated worktree, review it, and have Cursor commit it on this machine.
Use a structured issue body / `gh issue create --body-file` to preserve literals.
Include task ID, base SHA, exact paths, and `Implementation-Agent`. Do not include
secrets, private captures, binaries exceeding issue limits, or unrelated changes.
If the patch exceeds issue limits, ask for an approved artifact-transfer route.
This is a manual commit-broker route, not an automatic patch execution service.
Commander treats submitted patches as untrusted changes and inspects them first.

## Background discovery

```sh
python3 scripts/coord.py watch --interval 60
```

The watcher fetches once per minute, updates local snapshots, and continues until stopped (or the optional `--until` timestamp). It never calls models, creates commits, pushes, applies changes,
or marks anything reviewed. Run under a session/service manager for persistence.
It discovers work; an active agent or human still performs review/dispatch.

## Validation

```sh
python3 -m unittest discover -s tests -v
```

Tests use temporary bare remotes/clones/worktrees and synthetic commits. Cursor is
replaced by a test double ONLY inside those isolated fixtures; no model calls or
real repository commits occur during testing. Production publication invokes the
real Cursor executable and verifies its output.

## Integrate concurrent work and resolve conflicts

Commander reviews the candidate diff and handoff, then pins its full fetched SHA:

```sh
python3 scripts/integrate.py prepare --ref origin/agent/worker-a/task --sha FULL_REVIEWED_SHA --name worker-a-task --worktree /absolute/new/worktree
```

This creates a separate `agent/commander/integrate-...` worktree based on current
main and performs `git merge --no-commit --no-ff`. It preserves the active checkout.
Input revisions, clean index entries, conflicts, and execution phases persist in
the worktree's Git metadata. Existing worktree destinations are never overwritten.

In that worktree, run:

```sh
python3 scripts/integrate.py finish --allocation AUTHORIZED_CURSOR_GRANT --check '["python3","-m","unittest","discover","-s","tests","-q"]'
```

Supply checks appropriate to the changed product too, using repeated `--check`
JSON argv arrays; commands execute without an implicit shell. Rust changes need
Rust checks; native changes require validation on a Mac. A trivial exit-zero
command is not acceptance evidence. Choose commands from reviewed project code.

The helper invokes Cursor once to resolve conflicted paths using both parents'
intent and execute the merge commit. It rejects edits to cleanly merged paths,
wrong parents/identity/provenance, dirty results, and failing validation. It checks
the final commit again after validation. Only a passing result is pushed to its
integration branch. No blanket ours/theirs policy is allowed; ambiguous intent or
changes beyond conflict paths require an active integration agent to investigate.

After reviewing the resulting diff and validation, promote the exact merge:

```sh
python3 scripts/integrate.py promote --sha FULL_VALIDATED_MERGE_SHA
```

Promotion fetches again, requires main still equals the prepared baseline, and
uses a non-force push. If main advanced, prepare a new combined integration and
validate it. Repository branch protections still apply.

A Cursor timeout can leave a valid commit. `finish` records its spending phase
before calling Cursor, so repeating `finish` reconciles existing Git state and
never starts a second paid call automatically. Failed validation leaves everything
in the integration worktree and never pushes. If no valid commit exists, inspect
and repair deliberately rather than retrying a model blindly. Store the relevant
validation evidence and resulting main SHA in the Commander handoff at publication;
the local execution packet alone is not a cross-machine handoff.


The current autonomous startup and next-task process is in
[autonomous-workers.md](autonomous-workers.md). New Cursor commits additionally
coauthor the authenticated GitHub user; publication verifies the exact trailer.
The documented mac-m1max-a primary-author exception is preserved.

Commander dispatch daemon (dedicated clean worktree):

```sh
python3 scripts/dispatch_loop.py --watch --interval 30 --publish --allocation cursor-continuous-dispatch
```

This consumes only explicit queued work and matching completion reports. It archives
completion evidence, advances assignment revision and next-task pointer, and uses
Cursor to publish the update. It does not invent work when a queue is exhausted,
mark reported-ready work integrated, or automatically spend another provider's funds.
Commander replenishes queues and performs actual integration in parallel. The daemon
fast-forwards its clean checkout when main advances and honors project stop control.

## Claims

Before starting a task, claim it so a second lane doesn't duplicate the work; release
or close it when you stop. Claims live on a dedicated `coordination-claims` branch
(never main), one file per task at `claims/<task-id>.json`, and are read/written with
first-non-force-push-wins semantics: every write is a plain push, so a losing write is
rejected by the remote and transparently retried on the winner's tip (this is also how
the branch itself gets created the first time it's needed). Every claims command runs
in a throwaway `git worktree`; it never touches your own checkout, branch, or index.

```sh
python3 scripts/coord.py claim FT-003 --actor worker-a --machine mac-a --branch agent/worker-a/mac-shell
python3 scripts/coord.py claims --touch FT-003 --actor worker-a   # at each checkpoint
python3 scripts/coord.py close FT-003 --actor worker-a --gh-ref https://github.com/.../pull/12
```

`claim` prints `CLAIMED` and exits 0 once a post-push re-read from origin confirms it.
If someone else already holds an open claim on that task it refuses without writing
anything, prints `LOST: held by <actor> since <started_utc>`, and exits 1. Calling
`claim` again as the same actor is idempotent: it only refreshes `updated_utc` (and any
newly given `--branch`/`--gh-ref`/`--note`), keeping the original `started_utc`. If the
write's outcome can't be confirmed on origin after retrying (transport failure, not a
lock refusal), the command prints `NOT_COUNTED: ...` and exits 3 — treat that as unknown,
not as failure or success, and re-run `coord.py claims` to see the actual current state
before deciding whether to retry.

`release`/`close` require you to hold the claim; a Commander may override with
`--force-by-commander <commander-id>` matching `commander_id` in `coordination/authority.json`.
`close` additionally records `--gh-ref` (the merged PR, typically).

```sh
python3 scripts/coord.py claims                       # list open claims
python3 scripts/coord.py claims --actor worker-a       # filter to one actor
python3 scripts/coord.py claims --json --stale 8       # open claims idle >8h
```

`claims --stale N` lists open claims whose `updated_utc` is more than `N` hours old
*and* whose recorded `--branch` has no commit on origin newer than that `updated_utc`
(a claim with no branch is stale by age alone); it exits non-zero when any are found,
so a `watch`-style loop can alert on it. `coord.py checkpoint` also surfaces the
caller's own open claims and any project-wide stale claims (at a fixed 24h default) as
part of its normal output.
