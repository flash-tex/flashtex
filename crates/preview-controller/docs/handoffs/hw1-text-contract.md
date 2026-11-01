# Explicit math text: owner handoff proposal

Read-only inspection after combined HW1 candidate 1cf2fae7. Compiler base
1c02a2bc50874770d15a4decbaa5c3a376ee3ae9; producer inspected at
9aaec57a019c6a0073419eeb3ec90f922f5b367c. No implementation or adoption claimed.

## Representation and consumers

Add a distinct compiler math `Nucleus::Text` representation. Do not encode text
as a multi-character Symbol: the producer currently converts that case to a
math group of ordinary character atoms, which does not select text glyphs.
The producer `typeset.rs:1098` conversion must emit an Ord atom containing
math-layout `Nucleus::Text`; `Atom::text_op` selects Op and is inappropriate.
Existing `math-layout/src/layout.rs:222` visits `text_glyph` per character at the
current style size. That establishes a Roman glyph route, not full text shaping,
kerning, ligatures or arbitrary TeX text semantics.

Producer `incremental.rs:237` structural hashing needs a distinct Text tag plus
its contents. Its span-shifting visitor at line389 needs the new leaf handling.
Compiler `math.rs` layout, shift_list and min_start visitors require exhaustive
arms. The compiler legacy MathItem carries no font choice, and
`layout.rs:370` infers the font from text: add an explicit text-font distinction
through this path so the intended face survives placement and PDF export.
Both paths must be reviewed; making one exhaustive match compile is insufficient.

## Parsing and recovery contract to agree

Use a bounded group collector over the existing token stream, not required_group:
that routine parses math and discards spaces. Retain token/document spans and
macro-call provenance. Iteratively collect balanced braces within the existing
math depth/input limits. A missing opener must not consume the next math atom;
an unclosed argument must diagnose and preserve bounded partial output.

Initial corpus groups are (a), (b), (c), (d), and. These contain no spaces, but
space behavior must be explicit before broad acceptance: TokenKind::Space is
already a normalized token, not the original whitespace bytes. Decide how spaces,
comments and grouping map to TeX text semantics; do not concatenate non-space
characters and silently claim support. Unsupported nested commands, math shifts,
paragraph breaks, font switches and shaping forms must remain diagnostic with
source positions. Do not erase an unsupported command while labeling its group
fully supported. Literal ASCII scope can be a first milestone, not the final
compatibility boundary.

## Acceptance before publication/adoption

- Parser: five unchanged HW1 groups, empty/missing/unclosed groups, nested braces,
  scripts, macro-produced groups and exact original-source diagnostic spans.
- Text-versus-symbol distinction: Roman face and Ord spacing; same bytes as a
  math Symbol must not alias the incremental cache fingerprint.
- Incremental versus clean: edits inside text, text-to-math replacement, preceding
  UTF-8 edits and multi-document spans, preserving complete diagnostics/output.
- Producer: exact compiler dependency pin, converted AST, real Roman glyph IDs,
  resources, script-size behavior and export extraction. Retain unsupported cases.
- Oracle: compare the bounded text fixtures against reference LaTeX geometry and
  raster output; existing per-character text_glyph is not proof of pixel parity.

Compiler owner must approve the AST change; producer owner owns its adapter,
hash/shift updates and dependency adoption. This document authorizes no overlapping
writes. mathbb remains a separate alphabet/font/export implementation.
