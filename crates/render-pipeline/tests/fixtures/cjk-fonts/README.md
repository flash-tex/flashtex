# CJK paint faces for `tests/cjk_env.rs`

Four subsets of Noto Sans CJK (the per-region "SubsetOTF" builds), one per
CJK.sty family the probe uses: `NotoSansJP` paints `C70/min`, `NotoSansSC`
`C70/gbsn`, `NotoSansTC` `C70/bsmi`, `NotoSansKR` `C70/mj`
(`crate::cjk::paint_families` names each family). They hold only the 68 CJK
characters of the probe and of `fixtures/real-world/unicode-accents/main.tex`.

Why they exist: a CJK character with no installed paint face keeps its box
but ships no glyph run, so a machine without CJK fonts (the ubuntu CI runner)
had nothing for the test to measure, while a Mac painted from Hiragino and
passed. The test indexes only this directory, so every machine paints from the
same faces. Positions come from the `C70` subfont metrics (`src/cjk.rs`), never
from these outlines.

Source: `https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/SubsetOTF/<R>/NotoSans<R>-Regular.otf`
(noto-cjk `main` at f8d157532fbfaeda587e826d4cd5b21a49186f7c), sha256
JP dff723ba…ac5073, SC faa6c9df…5ea9, TC 5bab0cb3…237537, KR 69975a0a…ed68.
Subset with fontTools `pyftsubset <font> --text-file=<the 68 characters>
--layout-features= --no-hinting --desubroutinize
--name-IDs=0,1,2,3,4,5,6,13,14`. Family names are unchanged ("Noto Sans JP"
etc.; the upstream copyright reserves only the name "Source").

License: SIL Open Font License 1.1, `OFL.txt` (upstream `Sans/LICENSE`).
Copyright 2014-2021 Adobe (http://www.adobe.com/).
