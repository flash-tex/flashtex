# Internal historical-progress burst measurement

Release historical_burst example drives the actual Controller and original compiler on one serialized Rust worker. It performs an initial compile before the timed20-edit50KBburst, targets30ms send intervals, and polls/takes historical completions between edits. All saves must succeed, current previews must pass current-source checks, and historical displays must claim their private original provenance. Final output equals a fresh original-compiler build; drop/reopen verifies final durable source. Clean comparison is outside timing and uses a bounded fresh runtime.

| Policy | Historical progress claims | Current previews | Final current after last send | Actual last send from burst start |
| --- | --- | --- | --- | --- |
|off|0|2|56.480ms|570.148ms|
|on|14|2|51.996ms|570.131ms|

Each event timestamp/generation and actual send time is in the raw artifacts. Both runs preserved exact clean final output and durable reopen. These are one run per policy on shared Linux, not a stable speedup claim. Historical progress is older output and must never be counted as a current preview. This bypasses helper JSON/stdout and native rendering; do not compare these totals directly with helper-delivery timings. The small final-time difference is not evidence of a compiler speedup. Driver and compiler hashes plus underlying controller/runtime source identities are in provenance.json.

Run cargo run --release --example historical_burst -- COMPILER off 50000, then on 50000. Sizes100..500000 are accepted; each run is fixed20edits and all compiler waits are bounded. Sources use temporary ledgers. The example and strict all-target Clippy pass; existing41controller/26runtime tests were verified at their preceding implementation checkpoints. No native wire negotiation or production activation has occurred.
