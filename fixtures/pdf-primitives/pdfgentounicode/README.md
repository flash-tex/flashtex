\pdfgentounicode=1 with \input glyphtounicode writes /ToUnicode CMaps; the \pdfglyphtounicode entry maps the dash glyph to U+00E9 (the default Adobe Glyph List mapping for the em dash is U+2014).
qpdf --qdf --object-streams=disable --normalize-content=n --deterministic-id main.pdf out.pdf && grep -a '00E9' out.pdf  # match: <7C> <00E9> inside the ToUnicode CMap
without the primitive lines: no match (the CMap's bfrange <7B> <7C> <2013> maps the same glyph to the default instead)
