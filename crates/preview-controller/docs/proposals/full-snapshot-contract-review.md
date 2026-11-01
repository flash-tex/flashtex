# Helper review of complete display-list-v2 contract consolidation

Review input: Commander-owned draft
`docs/contracts/runtime-v1-display-list-v2.md`, SHA-256
`1d670c998c83c41d3f5957da09c75be44c17eda993d6d3da7187af2436190353`.
Implementation review base: helper f8177134. This is a review handoff, not an edit
to the authoritative contract, a new capability, or wire activation.

## Corrections required before adoption

1. Helper enablement performs producer negotiation itself. In
   `src/display.rs::configure_display_candidates`, enablement checks availability
   and exclusivity, adds display-list-v2 while preserving other capabilities,
   configures runtime candidate acceptance, clears the previous submission and
   invokes compile_current. The caller awaits its ACK before expecting candidates;
   it must not issue a redundant second capability request as the draft suggests.
2. The ACK carries preview_error. enabled=true is policy state, not evidence that
   the submitted compile succeeded, the producer accepted v2, or a candidate was
   rendered. Keep declined-capability and failed-compilation outcomes explicit.
   The helper may already have changed policy when preview_error is returned.
3. Name the runtime default8MiB limit as an accepted compiler-output frame limit,
   not an ambiguous input/request allowance. Helper stdin has its own1MiB bound;
   complete helper output is16MiB including newline. Configured producer frames
   are bounded by15MiB here to reserve envelope headroom. Re-serialization growth
   can still refuse output; reserved space is not proof every result fits.

Suggested replacement for the enablement paragraph:

> The helper route is explicitly opt-in and default OFF. The client supplies the
> exact helper capability and renderer-support confirmation, then waits for the
> configure_display_candidates result. This operation also enrolls the producer
> layout capability, preserves other requested capabilities, invalidates the prior
> submission and requests compilation. Its enabled policy flag and preview_error
> must be interpreted separately; successful enablement is not producer acceptance,
> a renderability check or paint acknowledgement.

## Boundaries verified against current helper code

- Raw and Value strategies have separate acknowledged capability strings. Strategy
  selection occurs before startup; restart preserves strategy and resets enablement.
- Display candidates and historical snapshots remain mutually exclusive. A conflict
  does not silently replace another enabled policy.
- Candidate validation compares current request/project/compile identity and the
  entire submitted/current index snapshot, including exact document membership,
  editor revisions, UTF-8 byte lengths and content hashes. Candidate envelope
  metadata does not provide independent authority.
- Required replies keep FIFO priority; queued optional frames can be evicted, and
  a started write is nonpreemptible. Neither admission nor write completion proves
  installation. Complete helper frame limits apply after wrapping/serialization.
- Candidate source actions remain disabled until the native consumer supplies
  current authoritative state and performs strict rendering/source validation.
- GH37's f8177134 adds explicit compile-generation equality to ordinary preview
  currentness and poll admission. This does not change wire versions or promote
  historical snapshots to current output.

Validation evidence: the generation mismatch regression failed before the fix;
17 lifecycle and30 stdio checks, including original compiler gates, and strict
all-target lint passed afterward. This review adds no tests or performance run.
Native adoption and full producer/renderer fidelity remain separate owner gates.

The page/delta proposal remains separate. Preserve full v1/v2 and decline behavior;
this consolidation must not implicitly approve filtered pages, reconstructed bases,
new resync messages or increased budgets.
