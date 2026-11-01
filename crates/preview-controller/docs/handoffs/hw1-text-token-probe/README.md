# Text-token boundary probe

Executed the exact compiler lexer module using rustc and a minimal DocumentId/Span
shim; ten inputs, all token ranges slice their original UTF-8 source. This probes
lexing only, not full parser recovery or reference TeX behavior. Reproduce by
pointing probe.rs module path at the pinned lexer and compiling with rustc
--edition=2021. Hashes identify the actual module, binary and raw output.

A comment excludes its following newline; that newline becomes Space. Therefore
a collector that drops Comment and emits every Space cannot alone implement TeX
comment joining. Preserve this case in the parser acceptance suite, and use a
reference oracle before claiming correct comment/whitespace semantics. Multiple
ordinary spaces likewise collapse to one token; blank lines become ParBreak.

Escaped braces are Word tokens with spans including their backslash, while raw
braces are structural. A collector must test token kind for group depth, never
word contents. UTF-8 text retains byte spans. Missing opener and missing closer
remain distinguishable inputs; the collector must implement and test recovery.
