# Compiler/cache review of Text producer65ce2dd

Exact reviewed producer:65ce2dd9933d0bed40fd083c6cd4b39b90899e7b.
Read-only review of compiler adaptation, cache/source identity and diagnostic
coverage. No build or oracle run. Renderer owns the separate font/metric review.

The implemented convert_math_with recursively carries one TextSink through groups,
fractions, radicals and scripts. Its compiler Nucleus::Text arm is feature-gated;
the default vendored compiler lacks that variant. Existing six math_text tests
construct TextSink lists directly. Default55 passing tests therefore do not establish
that an adopted Text compiler and enabled feature compile together.

Actual incremental.rs still has only Symbol/Fraction/Radical arms. Adoption requires
Text hashing with a distinct tag (recorded patch uses3 plus the entire text) and
Text leaf handling in shift_math, preserving the outer atom span and script shifts.
The evidence adapter.patch contains these changes but ALSO an obsolete typeset
conversion hunk used for the before binary. Do not apply the whole historical patch
over the new TextSink implementation; extract only the required cache/shift changes.
Feature activation and compiler repin remain coordinated owner actions.

The current public convert_math wrapper discards its temporary TextSink. Repository
search found no production callers at this SHA; production uses convert_math_with.
If a future caller uses that wrapper for Text, its handles have no retained run data.
Keep it restricted/documented or require the sink-bearing interface before exposing
Text-enabled conversion to additional callers.

math_items uses source_of(MathRec.span) for clusters, so navigation remains at the
enclosing expression span. A shaped ffi cluster retains its full output text and
byte range. This is not evidence of character-precise source navigation inside a
Text argument, and no Text argument range should be inferred from glyph indices.

The six recorded probes cover successful comment/brace/label/ligature/script/space
cases. They do not by themselves prove preservation of compiler recovery diagnostics.
After adoption, replay the malformed Text cases (missing/unclosed argument, unsupported
command, Unicode shaping diagnostic) and incremental Text↔Symbol edits from the root
compiler packet. Require the same explicit diagnostic policy and source identities;
never suppress errors just to admit a glyph run or reuse a cached math box.

The newly added trailing-linebreak guard deliberately refuses the entire affected
paragraph with paragraph_final_linebreak instead of allowing the vendored panic.
Its following-paragraph recovery is owner-tested; this is not full TeX-equivalent
partial layout. The paragraph owner still needs to handle the empty final line.
That change is separate from successful Text probe diagnostic equivalence.

replay.py records returncode but does not reject nonzero exits before reading an
existing display.json, and reuses output directories. A rerun may read stale display
output after worker failure. Treat committed captures as historical evidence; before
new acceptance runs use a fresh output directory and require successful worker exit
and newly generated output. This review did not rerun those captures.
