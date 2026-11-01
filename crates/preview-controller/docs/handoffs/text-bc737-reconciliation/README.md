# Text reconciliation for Kabir's current compiler

Owner handoff; isolated apply checks only. No authoritative compiler mutation,
build, adoption, producer repin, or pixel-parity claim.

Exact base: `bc73712632345dea800267561e7fec7135cfe465` on
`agent/claude/compiler-foundation`. This patch replaces the stale-base cumulative
patch **only for adopting Text on this base**. Do not apply both patches.
Owner coordination: GH issue1 comment5647108963 requests base confirmation and
checks whether another worker has begun Text. Wait for that answer before builds
or implementation beyond isolated reconciliation; a newer base requires rechecking.

## Reconciliation map

| File | Included change | Existing changes preserved |
| --- | --- | --- |
| src/math.rs | Text nucleus, Roman font intent, literal argument/comment handling, ASCII word-run optimization, span traversal | Symbol table, delimiter pairing and unknown-delimiter diagnostic |
| src/layout.rs | Honor explicit MathItem font | Heading policy and owner ligature test |
| src/incremental.rs | Clone Text while shifting source spans | All other incremental behavior |
| tests/math_text.rs | Seven existing Text regression tests | Existing test files untouched |

All paths above are relative to crates/compiler. Parser, export, JSON/protocol,
fonts, dependencies and producer sources are absent from this patch. In particular,
the five already-adopted symbols and starred heading changes are not replayed.
The final patch applies cleanly to a fresh isolated copy of the exact base; all
four resulting source hashes match evidence.json. Forward and reverse checks pass;
symbol-table and delimiter-reader sections also compare exactly with the base.

The original cumulative candidate's 104 passing tests and lint result do not prove
this reconciled combination compiles. Required owner acceptance: build compiler and
all enum consumers, run math_text plus delimiter/heading/symbol/navigation and
incremental-equivalence tests, then replay the recorded Text protocol session with
the bounded probe. Do not reuse old expected replies for fixtures where the owner's
new delimiter behavior intentionally changes diagnostics. Producer Text adaptation
and space/brace/ligature fidelity remain the existing producer owner's responsibility.

The test fixture comes from the previously reviewed Text candidate, not a newly
invented parser variant. Renderer review f261b36c/a5e7fa7c documents the remaining
producer gaps. Native pixel comparison and typing-to-visible latency are separate
acceptance gates and remain unmet by this source-only check.

## Static consumer inventory on the pinned base

Repository-wide Rust search for `Nucleus::` and `MathItem {` found compiler enum
matches only in `src/math.rs` and `src/incremental.rs`. The patch covers layout,
recursive offset copying, minimum-source-offset traversal, and incremental source
remapping. All three MathItem constructors are in math.rs: literal nucleus,
radical sign, and fraction rule. Text supplies Roman font intent; the latter two
supply None to retain current symbol/rule behavior. layout.rs consumes the explicit
font when emitting positioned items.

The similarly named enum in crates/math-layout is a different type and is not
modified here. Its Text support does not remove the producer adapter obligation.
External/vendor consumers are outside this compiler-branch inventory; the existing
producer owner must retain the reviewed hash/shift/Ord Text conversion adapter.

These are actual test target names present on bc737126 plus the proposed fixture:

```sh
cargo test --manifest-path crates/compiler/Cargo.toml --lib \
  --test math_text --test delimiters --test acceptance --test burst_edits \
  --test corpus_gate --test layout_capabilities --test pinned_fixtures \
  --test recovery --test references_and_figures --test restart \
  --test robustness --test unsupported_inventory
cargo clippy --manifest-path crates/compiler/Cargo.toml --all-targets -- -D warnings
```

These commands are the next owner gate, not executed results. On this base,
delimiters.rs also contains the membership/export, starred-heading and unknown
control-word delimiter regressions. Do not copy old commands naming separate
starred_sections/membership/quantifiers test targets absent from this branch.
