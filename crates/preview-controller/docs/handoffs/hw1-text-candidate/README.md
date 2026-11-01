# Isolated compiler Text AST candidate

Cumulative patch against exact 1c02a2bc50874770d15a4decbaa5c3a376ee3ae9,
including prior starred sections, membership, quantifiers and logical symbols.
Replaces the prior cumulative patch; do not stack. Clean archive application
matches all ten tested source hashes. No authoritative compiler adoption.

Adds Nucleus::Text(String), a bounded iterative collector over existing tokens,
explicit Roman MathItem font intent, layout propagation and both span-shifting
visitors. Literal grouping, normalized spaces, comment-newline joining and
escaped braces are handled without a second lexer/parser. Unsupported nested
commands, non-ASCII shaping, paragraph breaks and excessive group depth remain
diagnosed with recovered literal output. Missing opener leaves the next atom;
unclosed groups retain partial text. General amsmath shaping is not claimed.

101 library/integration tests pass across combined-tests.log (96) and the separate
robustness.log (5), three existing ignored; strict all-target Clippy passes.
Initial builds exposed an unhandled incremental span-mapping visitor; error logs
are retained, and the final patch adds its Text arm. Five original HW1 text groups
have exact command-through-group source spans; 72 diagnostics remain. Raw
stdin/stdout and preserved executable hash are included. No PDF raster or native
timing claim. The first evidence aggregation expected 101 in the 96-test log;
the omitted robustness target was then run separately, without source changes.

Producer adoption is BLOCKED on its broader spacing/encoding/resource profile.
Reviewer 9ce5e9e0 provides separate exact-base hash tag3/content, shift leaf and
Ord + math-layout Text conversion handoff; no dependency repin or actual producer
build is included here. Existing text_glyph uses a per-character path rather than
a complete shaped text run. Compiler tests for spaces/braces do not establish
producer fidelity for those inputs. Before adoption the producer needs correct
space/encoding support or explicit unsupported diagnostics for unverified inputs.

## Source-navigation follow-up

Two added tests pass: macro-generated text points exactly to its invocation, and
text in an included chapter keeps exact chapter byte spans through entry edits,
UTF-8 chapter edits, document-order reversal and content changes. Every project
step compares complete incremental pages and diagnostics with a clean compile.
Six current math_text tests plus 97 other tests give 103 distinct passing tests;
this is not the sum of repeated runs. New tests and strict lint pass; the updated
cumulative patch applies cleanly and matches all ten current source hashes.
Only test code changed after the original binary/HW1 capture, which remains
identified by its preserved executable hash. Native click handling is untested.
