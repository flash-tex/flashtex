# Independent three-state helper capture review

Reviewed root capture `518a2ce3` without launching a workload. The reproducible
`tools/audit_helper_9aa.py` verifies nine compressed artifacts, records their
original hashes, checks both diagnostic files, 78 existing asset hashes, pinned
harness/client hashes and actual frozen helper298/producer347 binary hashes.

All three states preserve the full direct v1 and v2 results and exact nested raw
sibling spelling. Each candidate follows its matching v1, remains untrusted with
source actions disabled, and binds editor revisions1–3 separately from compile
revisions2–4. Source hashes/UTF8 lengths, request IDs, successful capability
acceptance and empty diagnostics agree. Edit acknowledgements retain exact text.

The direct producer request is reconstructed by the owner harness; helper stdin
was not captured. Reopen success is an executed harness assertion and provenance
claim, but the reopened snapshot itself was not archived, so this audit cannot
independently reproduce that equality from captured bytes. The live diagnostic
snapshot has no partial tail; later stderr may be absent. No native or performance
claim follows from this audit.

## Heading projection handoff

Read `8f8678c2` isolated compiler patch only. It represents starred headings as
empty `Heading.number`, preserves counters and prior label values, and separately
suppresses empty numbers in compiler layout. Exact producer9aa `src/adapter.rs`
already tests `!number.is_empty() && level <= secnumdepth` before emitting number
text and following glue. Its `UnitKind::Heading` projection borrows the compiler
number. No duplicate producer star parser or number suppression is needed. This
is source compatibility review, not execution of the patched compiler through the
producer or HW1 acceptance.

## Full snapshot contract

Draft mainad922ea1 agrees with current runtime pairing, complete source bindings
and timeout. For precision: v1 Preview can precede sibling validation, and stale
or cancelled accepted results still drain their sibling before the next dispatch.
The linked size note correctly distinguishes tiny-cap failure envelopes from
strict wire compliance. No contract or runtime production changes were made.
