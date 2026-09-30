\pdftrailer appends entries to the PDF trailer dictionary (\pdftrailerid was tried too but qpdf --deterministic-id rewrites the /ID, so its effect is invisible after normalisation; see check-in).
qpdf --qdf --object-streams=disable --normalize-content=n --deterministic-id main.pdf out.pdf && grep -a 'TrailerValueABC' out.pdf  # match: /CustomKeyXYZ (TrailerValueABC) in the trailer
without the primitive: no match
