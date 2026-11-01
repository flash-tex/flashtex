# Repeatable scaling replay

Build `crates/document-runtime` release example `replay`, then run
`python3 crates/preview-controller/examples/scaling_replay.py --compiler /absolute/original/compiler --replay /absolute/replay --output /tmp/scaling --edits 20`.
Supply `--compiler-source-sha` after independently verifying the binary's source.
The output directory holds deterministic full request transcripts, per-sample
results, stderr and a report with binary, generator and input hashes.

Three-file projects total exactly 5,000 / 50,000 / 500,000 source bytes. Edits
alternate between two included files near their beginning and end. Each result
from a persistent compiler is compared with an independently launched fresh
compiler. Equality includes the entire positioned-result envelope. Identical
errors are recorded separately from clean compilation; they are not success.

Recorded Linux release compiler9026d8a run:20 edits per size; 5KB and50KB all
compile cleanly and match fresh results, p95 approximately2.59ms and25.53ms.
500KB fails runtime output validation before a sample: direct invocation emits
valid10,135,609-byte JSON with301 pages, exceeding the8MiB runtime frame limit.
Tracked in GitHub issue21. No pages are skipped to make this test pass.

These are one synthetic prose workload and sequential edits, not a broad speed
claim. Transport timing excludes durable editor saves and native painting.
Reference LaTeX/PDF parity and visual quality are not measured. Retain failures
when comparing later runs; do not report only completed small cases.

The replay accepts an optional second argument for maximum frame bytes. The Python
runner exposes it as `--max-frame-bytes` (128..64MiB), default8MiB unchanged.
This is a measurement override, not a production policy change.

The16MiB experiment accepted17 clean, exactly equal500KB samples and then hit the
runtime's5-second response timeout. First persistent sample247ms; later examples
769ms and1,384ms, while fresh launches were approximately280–402ms. The report
retains the incomplete classification;17 successes do not imply20 completed edits.
This demonstrates that removing the transport cap is insufficient for the200ms
objective. Compiler incremental-path attribution requires separate investigation.

Timing clarification after the initial reports: runtime `warm_total_ms` starts
once request encoding is complete; it includes result JSON parsing/validation.
The updated replay adds `warm_call_to_result_ms`, measured before request cloning
and submit through observation, and uses that for summary percentiles. Fresh
launch timing now stops before comparing JSON values. Historical committed reports
retain their original measurement definitions and are not rewritten as new runs.
