\pdfgentounicode=1 with \input glyphtounicode writes /ToUnicode CMaps; the \pdfglyphtounicode entry maps the dash glyph to U+00E9 (default Adobe Glyph List mapping is U+2013).
qpdf --qdf --object-streams=disable --normalize-content=n --deterministic-id main.pdf out.pdf && grep -a '00E9' out.pdf  # match: <7C> <00E9> inside the ToUnicode CMap
without the primitive lines: no match (CMap maps the same glyph to <2013> instead)
