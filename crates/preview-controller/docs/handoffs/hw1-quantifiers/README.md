# HW1 quantifier candidate

Isolated exact base1c02a2bc50874770d15a4decbaa5c3a376ee3ae9, separate from membership and starred headings. Existing command/export tables gain forall→U+2200→Symbol0x22 and exists→U+2203→Symbol0x24. Renderer6367ac34 verifies Adobe encoding, existing Ord math class, CM slots0x38/0x39 and actual font cmap. No hardcoded OpenType GIDs or parser fork. exists! remains a quantifier plus separate exclamation mark.

41 library and2 focused tests pass: exact UTF8 command spans, scripts, macros, separate bang, incremental-clean equality and explicit unknown diagnostics. Actual immutable HW1 output has103 diagnostics,10 forall glyphs and6 exists glyphs, each source slice exactly its command. Sixteen unsupported quantifier diagnostics are removed; remaining gaps stay explicit. Exact stdin/stdout/stderr bytes and executable/source hashes are preserved; executable path is in evidence.json.

An initial offline evidence-check script used the Rust internal span key instead of wire source.start_byte/end_byte and stopped with KeyError after saving raw output. The corrected check inspected the same saved output, without rerunning the compiler. No product change was needed.

No compiler-owner adoption, producer rebuild, native performance or pixel-parity claim. Combined testing is next. Primary encoding source: https://www.unicode.org/Public/MAPPINGS/VENDORS/ADOBE/symbol.txt . Compiler v1 spans are token-exact; downstream whole-math span granularity is unchanged.

Combined acceptance: all three candidates pass95 library/integration tests
(3 existing ignored), and strict all-target Clippy passes. The unchanged homework
has84 diagnostics,12 membership/10 universal/6 existential items, each mapped to
its exact original command. All14 unsupported hfill/normalfont errors remain.
Raw combined input/output/stderr, source/executable hashes and preserved executable
path are in combined-evidence.json.

Membership and quantifier patches edit adjacent table entries, so direct sequential
application failed its context check. The isolated combined tree explicitly keeps
all three command/export entries. combined-candidate.patch is the reviewed merge
against the same exact base, including starred diagnostics; apply it instead of,
not on top of, the separate candidates. Authoritative compiler files remain
unchanged by this handoff. No native/reference-PDF acceptance follows from these
compiler checks.
