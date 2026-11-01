# FT-005 is unassigned and three agents are building it

Reported by `claude` on mac-m5pro-kabir, 2026-09-12T06:05Z, from a survey of all
remote task branches. This is an integration risk, not a complaint: nobody did
anything unreasonable, and the cause is a gap on the board rather than any agent
overstepping.

## Cause

`coordination/TASKS.md` lists **FT-005 "Rust layout/output and source mapping" as
Unassigned**. Its dependency, FT-002, became ready, so three agents independently
started the same work. Two of them write into `crates/compiler`, which is the
declared `owned_paths` of FT-002, assigned to `claude`.

## What exists right now

| Branch | Writes | Approach |
|---|---|---|
| `agent/claude/compiler-foundation` | `crates/compiler/src/pdf.rs`, `src/metrics.rs` | original serializer, AFM tables typed in, zero dependencies |
| `agent/aarush-macbook/pdf-output` | **`crates/compiler/src/pdf.rs`, `src/protocol.rs`, `Cargo.toml`** | `pdf-writer` 0.15 + `tempfile` |
| `agent/mac-pdf/pdf-output` | `crates/pdf/` (new crate, ~4 400 lines) | separate crate with native visual checks |
| `agent/mac-claude-a/fontmetrics` | `crates/fontmetrics/` (new crate, ~2 600 lines) | base-14 advances measured via CoreText |

So there are **three PDF implementations and two font-metric implementations**.

## Two consequences that matter before integration

**A dependency would enter a crate that is deliberately dependency-free.**
`agent/aarush-macbook/pdf-output` adds `pdf-writer = "0.15"` and `tempfile = "3"`
to `crates/compiler/Cargo.toml`. That crate is currently std-only on purpose, so
it builds offline and deterministically. Adding registry dependencies changes
that property for every machine and every CI run. That is a decision for the
Commander, not a side effect of a merge.

**That branch was written against superseded code.** Its own header says
"Glyph metrics: inherited from layout.rs placeholder ratios". Those placeholders
were removed when real advance widths landed on `agent/claude/compiler-foundation`
at `de1020c`. Merging it as-is would reintroduce reasoning that no longer holds.

## An honest assessment of my own work

`crates/fontmetrics` is probably the better font-metric implementation, and I say
that as the author of the one it would replace. Mine is AFM values typed into
`crates/compiler/src/metrics.rs`. Theirs is measured from the system with
CoreText, carries a verification tool against CoreText, is still zero-dependency,
and — the part that matters most — is built around making the compiler's layout,
the Mac preview's drawing and the PDF writer's `/Widths` agree. Three components
measuring words differently is exactly how a preview and its export drift apart.

If the Commander consolidates on `crates/fontmetrics`, I will delete
`crates/compiler/src/metrics.rs` and depend on that crate instead. I am not
attached to my version.

## What I am asking the Commander to decide

1. **Assign FT-005 to exactly one owner**, and say whether `crates/pdf` or
   `crates/compiler/src/pdf.rs` is the surviving home for PDF output.
2. **Rule on the dependency question** for `crates/compiler`: stay std-only, or
   accept `pdf-writer`. Either is defensible; drifting into it by merge is not.
3. **Consolidate font metrics**, most likely onto `crates/fontmetrics`.
4. Tell agents not to write into another task's `owned_paths`. `crates/compiler`
   belongs to FT-002 until the board says otherwise.

## What I am doing meanwhile

Continuing FT-002 revision 3 inside `crates/compiler` only. I am not merging,
reverting or touching any of the branches above — they belong to their authors.
I will adopt whatever the Commander decides.
