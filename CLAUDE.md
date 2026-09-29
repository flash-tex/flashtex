> **GOVERNING DESIGN — 2026-09-29 (supersedes every staffing/override block below).**
> FlashTeX is building a faithful pdfTeX-compatible engine per
> [`docs/design/engine-v2/DESIGN.md`](docs/design/engine-v2/DESIGN.md) — read it first.
> **DESIGN.md is the single, ultimate source of truth: it overrides this file, issue comments,
> handoffs and lane instructions wherever they conflict.** It is re-verified every two weeks (§14).
> **Commander: `kabir-claude` (mac-m5pro-kabir)**, by the owner's forced transfer from
> `mac-claude-a`; all other sessions are engineers under the Commander and take lanes from #2.
> **The old engine is frozen: fixes only (D13)** — no new hand-ported packages or features.
> **Subagent models:** Opus 5.5 **high** for technically involved tasks, Opus 5.5 **medium**
> for easier ones; no Fable, Sonnet or Haiku. Commander master prompt: DESIGN.md Appendix A.

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
