@AGENTS.md

Read docs/INDEX.md, coordination/PROJECT.md, coordination/RESOURCES.md,
ORCHESTRATION.md, and coordination/COMMANDER.md before work. Agent onboarding
notes (formerly in the root README) are in docs/agents/README.md.

Current explicit user authorization: the 20x Claude Max plan on mac-m1max-a may
run project work and subagents. Parent mac-claude-a owns allocation/reports for
mac-pdf and mac-validation; use separate worktrees and publish coherent checkpoints.
Continue until user stop or full-project verification, reading updated assignments
at every checkpoint. Do not wait on permissions already granted in this session.

This authorization does not apply to Commander's protected personal Claude account
or authorize purchases/overages. All participants share the Max account's actual
quota. Other Claude accounts still require their separately verified funding route.
Preserve actual commit-executor provenance and the Mac-specific primary-author rule.

Interface contracts: `contracts/registry.json` is the source of truth for which
contracts exist, their canonical files and status (see contracts/README.md).
