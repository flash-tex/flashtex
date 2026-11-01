# Current cumulative HW1 compiler candidate

Apply this single patch to exact base1c02a2bc50874770d15a4decbaa5c3a376ee3ae9.
It REPLACES earlier cumulative patches and deltas; do not apply those as well.
Includes starred headings and honest unsupported-heading diagnostics; membership,
quantifier and logical mappings; explicit Text AST/Roman font; comment-separated
text arguments; preserved ASCII text runs. Full clean-archive application matches
all ten source/test hashes. This remains an isolated compiler-owner handoff.

Final selected library/integration gate: 104 passed, three existing ignored; strict
all-target lint passed on unchanged final sources. Optimized release binary's
complete HW1 reply matches the comment-corrected candidate (72 diagnostics).
No new visual or native latency acceptance is claimed. See hw1-text-runs for
207-input full-output comparisons and measured synthetic allocation improvement.

Producer reviewer f261b36c built the preoptimization Text candidate and confirmed
five labels, with real spacing/brace/ligature gaps. Producer adoption requires
correct text metrics/encoding/ligature support or explicit diagnostics for those
unimplemented cases. Its three-arm AST adapter is a separate producer-owner patch.
Whole-PDF bytes need not match reference compilers; rendered page fidelity is the
user's target. Resource hashes here identify actual artifacts and preserve integrity.
