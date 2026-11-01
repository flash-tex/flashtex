# HW1 logical command candidate

Isolated exact base1c02a2bc50874770d15a4decbaa5c3a376ee3ae9. Recognizes vee→U+2228 and Rightarrow→U+21D2 with verified Adobe SymbolDA/DE export bytes. Renderer5e566513 verifies existing Bin/Rel classes, CM slots5F/29 and actual font cmap; no GID literals or aliases introduced.

41 library+2 focused tests pass. Actual unchanged HW1 has112 diagnostics, four vee and three Rightarrow glyphs with exact command spans. mid, setminus and Longrightarrow remain explicitly unsupported; mathbb/formatting errors are unchanged. Exact input/output/stderr and original executable hash are retained before any combined build.

This standalone candidate does not include prior headings/membership/quantifier changes. Authoritative compiler adoption remains pending. No native/PDF parity claim; downstream whole-math provenance limitation remains. Primary mapping: https://www.unicode.org/Public/MAPPINGS/VENDORS/ADOBE/symbol.txt .

## Combined four-candidate gate

`combined-candidate.patch` combines starred headings, membership, quantifiers and
these logical symbols against the same exact base. It replaces the separate
patches; do not stack them. Clean archive application passes and all eight patched
source/test files match the tested hashes in `combined-apply-check.json`.

The combined run passes 97 library/integration tests (three existing ignored) and
strict all-target Clippy. Unchanged HW1 produces 77 diagnostics and 35 exact
command-bound symbol spans: 12 membership, 10 universal, six existential, four
disjunction and three implication symbols. Remaining diagnostics explicitly retain
four mid, two setminus, one Longrightarrow, eleven mathbb, seven hfill and seven
normalfont occurrences. These results do not establish native rendering or PDF
parity. Exact combined stdin, stdout, stderr, preserved executable SHA and source
hashes are recorded separately from the standalone run.
