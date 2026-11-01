# Native consumer acceptance packet

This is a handoff for the existing Mac preview owner. Neither optional display mode
is enabled by default. Keep source actions disabled on every untrusted candidate.

## Tested producer/helper boundary

- `97c3a19b`: startup-gated raw helper with fixed decoder strategy, explicit
  capability acknowledgement, current-source checks and restart reset.
- `7817e4e8` (runtime owner): two-pass raw validation; constructor/API unchanged.
- `f5524794`: actual producer65dbe7d three-state replay through release helper
  `e39f50e594703c0d4b4bd6848e72fd2fe5169678`. Exact original helper candidate
  JSONL and producer stdout are under `../benchmarks/display-helper-raw100`.
- `549b4841`: unchanged producer6e69661 and three original26-page source states;
  all v1 output matches direct results, v2 explicitly declines, durable reopen exact.
- `6874316e`: buffered required replies, tested with11 binary unit checks and26
  stdio checks including the explicitly configured original compiler. This source
  is newer than the binary used in the above replay; rebuild before app testing.

No source/reply speed ratio should be inferred across these different runs.

## Configuration and current identity

For the experimental raw route only, set startup `display_transport:"raw-prototype"`.
Omit it for the established Value path. Enable with the existing request type
`configure_display_candidates` and payload
`{"capability":"display-candidates-raw-v1","enabled":true,"renderer_support_confirmed":true}`.
Wait for that exact capability acknowledgement. Value mode uses
`display-candidates-v1` instead; capability mismatch rejects before changing policy.

Restart preserves decoder strategy but disables display delivery. Renegotiate
before requesting display-list-v2. Historical snapshots and display candidates
remain mutually exclusive. Required v1 results and durable replies retain priority.

Before paint, compare the current session, project, request, compile generation,
source-version map and membership generation with the candidate; then validate the
nested source hashes/lengths, fonts and renderer-supported geometry. Compile
generations are not document revisions: the replay deliberately uses2/3/4 versus
1/2/3. The source editor/ledger remain authoritative. Never enable source actions
merely because transport correlation passed.

## Required remaining native checks

1. Use the bundled, rebuilt helper and actual producer for native keystroke-to-paint
   tests, recording source-to-painted-generation identity and queue/load conditions.
   Direct producer-to-Swift tests do not establish this helper route.
2. Exercise restart during candidate validation, typing before stale paint,
   page-count changes and window close. Confirm final durable source and default
   v1 fallback when v2 is declined; keep all26 pages in the multipage fixture.
3. Run strict renderer validation on original captured JSONL, preserving duplicate,
   finite-number and depth rejection. Parsed/re-serialized step JSON is metadata
   evidence, not original-byte proof. The renderer owner has independently accepted
   all3 raw captures and produced identical raw/normalized/escaped PDFs at3040a0ce.
4. Resolve and test native font discovery-to-CGFont byte identity (GH31). A content
   hash checked before reopening a font file does not authenticate later bytes.
5. Report native responsiveness with workload, percentile sample size, exact build
   identities and current/historical paint distinction. Linux receipt timings here
   are not native paint evidence or a universal sub200ms claim.

## Recovery invariants already checked locally

Wrong capability, corrupt source hash and stale sibling refuse; source editing and
kill/reopen recovery remain valid. Optional serialization refuses complete frames
that exceed the limit without partial output or loss of subsequent durable ACKs.
A stalled writer terminates through the existing watchdog. Required oversized
responses return the existing source-may-be-durable warning, then subsequent ACKs
remain ordered. If even that small error cannot fit, output stops for recovery.
No output caps, queue sizes or source approval boundaries were relaxed.
