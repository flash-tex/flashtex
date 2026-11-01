\pdfinfoomitdate=1 drops /CreationDate and /ModDate from the info dictionary (the %%CreationDate DSC comment inside the embedded font program is font data and remains; the grep needs the leading slash).
qpdf --qdf --object-streams=disable --normalize-content=n --deterministic-id main.pdf out.pdf && grep -a '/CreationDate' out.pdf  # no output: dates omitted
without the primitive: 1 match (/CreationDate present, likewise /ModDate), so =1 caused the omission
