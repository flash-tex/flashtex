# Preserve literal text runs during math token preparation

One-file delta applied after the f464c6d6 cumulative Text candidate and 61bd63f7
comment correction. The full chain clean-applies and matches the final math.rs
hash. Compiler-side scratch only; authoritative adoption remains pending.

Previously every literal text character allocated its own token/String before
the text collector joined them again. Token preparation now preserves ASCII word
runs while inside recognized text groups. Outside math words still split for
script attachment; non-ASCII text still splits to preserve existing diagnostic
positions/count. Structural braces, escaped Word braces, comments before the
opener, nested groups and malformed openers retain their token-kind distinction.

53 library/text/robustness tests pass and strict all-target Clippy passes. Actual
release protocol output matches the original release binary on 207 deterministic
valid/malformed inputs (seed1042), including macros, Unicode and deep groups.
Each complete reply is compared, preserving diagnostics and source ranges.

Alternating before/after/before/after runs of the prior text-scaling harness gave
changed-compile medians (six samples per size/build): 5KB 1.087→0.911ms;
50KB 11.134→9.191ms; 500KB 119.862→103.272ms (about14% faster at500KB).
The earlier nonalternating run showed a larger difference and is not used as the
claimed improvement. Ambient load is uncontrolled, sample count is small, and the
fixture is one large unbreakable text box. This is not native paint latency or a
general LaTeX benchmark. No response text/pages were omitted for speed.

Exact binary hashes, compressed input/before/after responses and per-run timing
rows are retained. The timing replay script/requests are in benchmarks/text-scaling.
Producer Text spacing/encoding/ligature gaps remain separate adoption blockers.
