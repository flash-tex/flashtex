# One-pass display field validation

Gather common text-item fields in one borrowed object traversal rather than repeated Value indexing. Keep source/font absence distinct from JSON null, preserve all original geometry/font/source/capability checks, and return the original Value without discarding extensions. Rule checks retain their original field lookups. No protocol or frame-limit changes.

Baseline e5a804d; same pinned compiler/input as ../profile-baseline. First sequential500KB comparison:20baseline+20candidate edits, all exact-clean. Median validation21.257ms vs14.154ms. End-to-end median357.667ms vs344.120ms is a shared-host observation, not a stable gain claim.

Reverse-order confirmation failed: candidate0completed and baseline4completed before both reported compiler response timeout. The commander confirmed concurrent font/render integration builds08:51–08:54 UTC. At08:53:40 load average16.21/8.45/5.33; these failures remain here and are not interpreted as candidate-specific regressions.

Isolated alternating comparison:20pairs with identical parsed input, clone outside timing, exact output equality after validation. Baseline median20.330ms; candidate13.272ms; median paired ratio0.648607,20/20pairs faster (~35% phase reduction). Input SHA256 2f525a17b95b4d8d8595116810d41ebaf9f432fa211c67f3b0dbbbacb207bdd3, captured original compiler reply, not a reference-PDF oracle.

The opt-in comparison-harness.rs.txt preserves the baseline validator and fixture adapter used. In an isolated scratch checkout, append it to src/lib.rs and run cargo test --release --lib compare_validation -- --ignored --nocapture with the two fixture paths adjusted to corresponding captured request/reply. It is not part of production code or the default test suite. Paired log retains each sample. Production candidate passed21runtime tests including the original compiler and malformed/capability/source cases, plus strict all-target Clippy. Performance evidence is text-heavy; no equivalent gain is claimed for rule-heavy or extension-heavy results.
