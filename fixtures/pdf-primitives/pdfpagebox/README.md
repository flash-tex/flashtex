\pdfpagebox=1 asks for the media box but \pdfforcepagebox=4 (trim) overrides it, so \pdfximage embeds the trim box (first build the support PDF in this directory: pdftex gen.tex; \pdfforcepagebox also logs an "obsolete" warning but still works).
qpdf --qdf --object-streams=disable --normalize-content=n --deterministic-id main.pdf out.pdf && grep -a '^    290$' out.pdf  # match: trim-box edge inside the included /BBox
without the primitives: no match (the default crop box is embedded instead: /BBox 20 30 280 370)
