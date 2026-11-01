\pdfomitprocset=1 drops the /ProcSet array from page resources (bare ProcSet strings in %% DSC comments inside embedded font programs are font data and remain).
qpdf --qdf --object-streams=disable --normalize-content=n --deterministic-id main.pdf out.pdf && grep -a '/ProcSet \[' out.pdf  # no output: /ProcSet array omitted
without the primitive: 1 match (the /ProcSet array present), so =1 caused the omission
