# Text producer bounds defect at65ce2dd

Exact producer65ce2dd9933d0bed40fd083c6cd4b39b90899e7b. Independent static
review prompted by rendering reviewer. Tiny Rust arithmetic reproduction ran;
no full producer build or end-to-end long-document reproduction was performed.

mathtext.rs::shape_run lines266/296 cast glyphs.len() to u16 for both spaces
and ink glyphs. MathRec::run_glyph/otf_glyph in typeset.rs97–110 use that gid as
an index back into the run. Index65536 therefore becomes0,65537 becomes1.
The shaped box keeps its later geometry while paint/text lookup refers to an
unrelated earlier glyph. Counting source characters is insufficient: count actual
shaped glyph entries including spaces and multiglyph clusters. This is an index
into a run, not an original font glyph ID.

TextSink::atom calls handle_char(texts.len()) without a checked bound.
handle_index recognizes only0xF0000..0x100000. Text index65536 produces valid
Unicode0x100000 but is no longer recognized by text_glyph/substitute; index131072
produces0x110000 and handle_char's expect panics. Existing handle tests cover only
0,1,255,65535. These are distinct limits from the per-run glyph index overflow.

Minimal Rust arithmetic observation:

```text
run_index=65535 stored_gid=65535 lookup_index=65535
run_index=65536 stored_gid=0 lookup_index=0
run_index=65537 stored_gid=1 lookup_index=1
text_index=65535 scalar=fffff valid_char=true recognized=true
text_index=65536 scalar=100000 valid_char=true recognized=false
text_index=131071 scalar=10ffff valid_char=true recognized=false
text_index=131072 scalar=110000 valid_char=false recognized=false
```

Owner remedy: prefer an explicitly indexed hbox/run representation that supports
large content, or split run storage while preserving geometry, clusters and source
provenance. Until such support exists, checked refusal must emit an explicit
source-bound error and keep the worker alive; never silently truncate or saturate
indices. Check capacity before mutating sink/run state so recovery cannot leave
partially assigned handles that appear valid. Do not treat temporary refusal as
full compatibility or pixel parity.

Required regressions exercise boundaries65535/65536/65537, contrasting first and
last glyphs so aliasing is visible, spaces near the boundary, and many Text atoms.
Test under actual Text-enabled compiler/producer route, including a valid request
after the oversized one. Keep existing small Text metric/cache/diagnostic gates.
Owner implementation belongs to the existing producer Text lane; root has not
modified producer sources or allocated a replacement worker.
