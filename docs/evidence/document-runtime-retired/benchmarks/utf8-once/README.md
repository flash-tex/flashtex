# UTF-8-once JSON parsing experiment

Validate the entire frame as UTF-8 once, then deserialize with serde_json::from_str. Preserve Value representation, all semantic/source validation, stale-response handling and malformed-frame failure. No production frame-limit change.

Alternating parser order over 20 pairs on one captured 10,135,609-byte compiler JSON reply: median from_slice 62.924 ms, UTF-8-once 55.996 ms; median paired ratio 0.89751, 19/20 pairs faster. Every parsed Value equals the original. Input SHA256 2f525a17b95b4d8d8595116810d41ebaf9f432fa211c67f3b0dbbbacb207bdd3. This captured reply predates the pinned4425 full-pipeline experiments; it is a parser microbenchmark, not a compiler comparison. Reproduce using release example parse_compare with any captured compiler frame.

Full replay: two optimized runs and a repeated baseline, each 60/60 exact-clean samples. Baseline profiling and compiler provenance are in ../profile-baseline. First optimized500KB median347.069 ms vs original357.649 ms, but second optimized p95797.474 ms vs repeatedbaseline436.692 ms under shared-host contention. Do not claim a stable end-to-end percentage or sub200ms result. Optimized first-run median parsing78.548 ms vs original85.843 ms supports the isolated finding.

Existing20 runtime tests including real compiler passed; added invalid UTF-8 frame terminal/no-preview regression passes; all-target strict Clippy passes. Artifacts retain full timing distributions and immutable input/compiler hashes. Initial baseline repeat failed to execute because a copied executable lacked its execute bit; fixed before rerun, and failed attempt is excluded from timing evidence.
