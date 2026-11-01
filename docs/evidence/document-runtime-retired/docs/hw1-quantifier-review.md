# Quantifier candidate artifact audit

Pinned8daaab996a2f18183b17b65ef5d6e48d50034bda. Read-only audit checks all six
manifest artifacts, retained executable94b3eae31e4837e2c76375827246a5751421ce1cfcf97ea7ee4261b60a794940,
original request/output correlation and immutable sourcef725e238. Stderr is empty.
The diagnostic multiset changes119→103 with exactly16 unsupported-command errors
removed and no additions or unrelated suppression. Ten universal and six
existential glyph items have source spans exactly equal to those removed records;
each span slices its original `\forall` or `\exists` UTF8 command.

Unlike the prior standalone membership capture, this candidate retains exact
stdin and preserved executable identity. That does not retroactively close the
membership gaps. This audit performs no compiler replay and establishes no combined
candidate, production adoption, native rendering or PDF visual/byte parity.
Renderer reviews Symbol encoding separately; FT002 owner remains the adopter.

## Complete three-candidate capture

Separately pinned97a15831. Audit checks all six combined manifest files, exact
archived stdin/stdout correlation, immutable source and independently rehashed
preserved executable284ba9ff9179e12a7ebc6b0635bb0b657e24fe5c2d91f552dae76a6c11060cc5.
The diagnostic multiset119→84 consists of49 removed records and14 added explicit
formatting errors (hfill/normalfont), not35 pure removals. Twelve membership,
ten universal and six existential symbols retain exact command source spans;
those28 spans match the corresponding removed unsupported-command records.
No unrelated remaining error is silently counted as success.

`combined-candidate.patch` SHA94d5bc23f160370b42e20f43ee42c2d03ec52b7212aae43c7465c06fbf85aee3
is explicitly based on1c02a2bc50874770d15a4decbaa5c3a376ee3ae9. Its scope is
compiler parser/layout/math/export plus three focused test files. It combines the
adjacent math/export table additions; independently applying overlapping earlier
patches on top would be incorrect. This is the complete isolated handoff, not a
commit to the authoritative compiler tree. Application/build/test logs remain
owner evidence; this read-only audit does not execute patches or repeat95 tests.
Existing FT002 owner alone adopts it. Whole-PDF/native parity remains unverified.
