# Comment before text argument: regression correction

Apply comment-fix.patch AFTER the cumulative Text candidate from f464c6d6.
This is a two-file delta, not another complete compiler patch. Clean application
and both final source hashes were verified against that exact parent.

The new test first fails because a comment between the text control word and
opening brace is treated as a missing argument. The fix skips Comment alongside
Space while finding the opener. Paragraph breaks remain separate. After the fix,
41 library plus seven text tests pass and strict all-target Clippy passes. This
adds one regression to the prior candidate; it does not rerun every earlier suite.

An actual compile with the preserved corrected binary produces an identical HW1
payload to the earlier 72-diagnostic capture. Exact input/output and binary hashes
are recorded separately; the earlier binary evidence is unchanged. No native
rendering, producer adoption, or broad text-shaping claim is made.
