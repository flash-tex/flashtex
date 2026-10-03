\pdfinfo writes /Title, /Author and /Keywords into the PDF info dictionary.
qpdf --qdf --object-streams=disable --normalize-content=n --deterministic-id main.pdf out.pdf && grep -a 'FlashTeXInfoTitle' out.pdf  # match: /Title (FlashTeXInfoTitle)
without the primitive: no match
