# Start here: shared project memory

Read this index after startup, resumption, or compaction. Follow links relevant to
your task; do not load the entire repository history into every prompt.

| Need | Authoritative location | Writer |
|---|---|---|
| End-user guides (install, Mac app, iPad companion, CLI tools, supported LaTeX) | [docs/user/README.md](user/README.md) | mac-user-docs (parent mac-claude-a) |
| Required collaboration behavior | [AGENTS.md](../AGENTS.md) | Integration owner with user direction |
| Agent onboarding: start prompt, coordination CLI, where handoffs live (formerly the root README's "Working with agents") | [docs/agents/README.md](agents/README.md) | Integration owner |
| Command, dispatch, reporting, integration | [ORCHESTRATION.md](../ORCHESTRATION.md) | Commander |
| Actual coordination commands and patch submissions | [Coordination CLI](coordination-cli.md) | Commander |
| Mac/Rust/capture message contract | [Runtime v1](contracts/runtime-v1.md) | Commander (FT-001) |
| Wire examples | `protocol/fixtures/` | Commander (FT-001) |
| Structured worker records | `coordination/agents/<id>.json` on worker branch | That worker |
| Authoritative executable assignments | `coordination/assignments/<task>.json` on main | Commander |
| Latest global update and recovery state | [COMMANDER.md](../coordination/COMMANDER.md) | Commander |
| Worker identities and capabilities | [ROSTER.md](../coordination/ROSTER.md) | Commander |
| Assignments, revisions, dependencies | [TASKS.md](../coordination/TASKS.md) | Commander |
| Product requirements | [Master plan](../latex-master-plan.md) | Product/integration owner |
| Deadline and acceptance gates | [PROJECT.md](../coordination/PROJECT.md) | Designated coordinator |
| Funding, permissions, allocations | [RESOURCES.md](../coordination/RESOURCES.md) | Designated resource owner |
| Delegation, progress, recovery procedures | [Agent operations](agent-operations.md) | Integration owner |
| Active work and blockers | `coordination/<agent-id>.md` on each task branch | That agent |
| Handoff format | [Agent template](../coordination/templates/agent.md) | Integration owner |
| Shared interfaces | `docs/contracts/<interface>.md` when created | Assigned interface owner |
| Durable decisions | `docs/decisions/<id>-<topic>.md` when created | Decision owner |
| Reproduction evidence / large outputs | Paths linked from the relevant handoff | Producing agent |
| Engine performance: how it is measured, and the committed baseline | [crates/perf-bench/README.md](../crates/perf-bench/README.md) | FT-070 perf lane |
| CI, releases, website publication | [CI/CD](ci-cd.md) | Release lane (mac-ci-release) |

Some interface and decision directories will be created as implementation starts;
their listing here does not imply those designs already exist.

## Discover updates before they merge

Run `git fetch origin --prune`, list remote task branches, and inspect the relevant
branch's handoff with `git show <remote-branch>:coordination/<agent-id>.md`.
Record the peer commit you reviewed and the resulting adaptation. Main contains
integrated decisions; branch proposals remain proposals until accepted.

Every new lasting document must be linked from this index, a listed topical
index, or the producing agent's handoff. Include owner, status, date, relevant
revision, and what supersedes it. Do not create orphan knowledge files.

## Information hierarchy

Higher-priority instructions and explicit user decisions govern. Shared contracts
describe agreed interfaces; code and test results describe observed behavior.
Handoffs and plans can become stale. When evidence conflicts with a document,
report and resolve the discrepancy instead of silently choosing a convenient one.

Keep startup material short. Keep detailed findings in linked topic files, and
retain concise evidence summaries rather than huge terminal transcripts. Never
commit credentials, access tokens, login URLs, or private captures.

- [Autonomous worker startup and continuation](autonomous-workers.md): one-time
  machine launch, noninteractive permissions, quota behavior, next-task queues.
- [Machine capability evidence](resources/machines/README.md): per-machine reports;
  resource authority remains coordination/RESOURCES.md.

## Current original Rust product interfaces

Read the exact task-branch SHA in the worker report before using unpublished APIs.
Main integration does not certify full LaTeX compatibility, PDF identity or native
latency. Every consumer must retain source/project/revision and resource identity.

- `crates/preview-controller/README.md`: durable editing, explicit reviewed insertions,
  recovery and compiler/index integration; `STDIO.md` is the native IPC contract when integrated.
- `crates/edit-ledger/README.md`: authoritative durable source, receipts, history,
  background service and restart handling. Do not build a second independent undo ledger.
- `crates/document-runtime/README.md`: persistent original compiler transport, stale
  response suppression and replay metrics; native paint is excluded from its timings.
- `crates/conversion-jobs/README.md`: bounded scheduling, durable intent, ambiguous
  provider-call recovery, typed status and explicit reviewed handoff. No automatic retry.
- `crates/project-index/README.md`: exact-revision source navigation and lexical
  bibliography/rename facilities; lexical results do not establish TeX expansion semantics.
- `crates/font-resources/README.md`: immutable font bytes, original GIDs, exact paths,
  TFM metrics and explicit encoding bindings. TFM8bit codes are not Unicode or GIDs.
- `crates/rendering-core/README.md`: experimental rendering-v2 validation, exact
  positioning, clipping and unhinted path consumers. No automatic wire activation.
- `crates/pdf/README.md`: original runtime-v1 PDF export; its font fallback and rule
  conventions remain explicit fidelity blockers.
- `docs/contracts/runtime-v1.md` and `docs/contracts/transfer-v1.md`: production
  message contracts. `docs/contracts/rendering-v2-proposal.md` is a proposal, not permission
  to change existing clients without negotiation and migration tests.

The sole orchestrator owns global integration and task queues. Engineers own their
assigned product paths plus their own reports. Only an explicit user stop ends
improvement cycles; completed checkpoints trigger useful next work.
