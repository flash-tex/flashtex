\pdfnames adds a name-tree entry to the PDF name dictionary; the \pdfdest line registers the named destination destA alongside it.
qpdf --qdf --object-streams=disable --normalize-content=n --deterministic-id main.pdf out.pdf && grep -a 'NameTreeHit123' out.pdf  # match: /ValueXYZ (NameTreeHit123)
without the primitive lines: no match
