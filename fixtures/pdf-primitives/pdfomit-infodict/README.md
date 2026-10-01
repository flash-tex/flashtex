\pdfomitinfodict=1 removes the /Info reference from the trailer (no info dictionary is written at all).
qpdf --qdf --object-streams=disable --normalize-content=n --deterministic-id main.pdf out.pdf && grep -a '^  /Info [0-9]' out.pdf  # no output: trailer has no /Info
without the primitive: 1 match (trailer /Info present), so =1 caused the omission
