\pdfomitcharset=1 drops the /CharSet entry from the embedded font descriptor.
qpdf --qdf --object-streams=disable --normalize-content=n --deterministic-id main.pdf out.pdf && grep -a '/CharSet' out.pdf  # no output: /CharSet omitted
without the primitive: 1 match (/CharSet present), so =1 caused the omission
