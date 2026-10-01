\pdfuniqueresname=1 gives each page-resource name an extra suffix (for example /F31 becomes /F31F3qA10); the suffix text depends on the format build date and the job name, so the grep tests its presence and shape, not a literal value.
qpdf --qdf --object-streams=disable --normalize-content=n --deterministic-id main.pdf out.pdf && grep -a -E '/F[0-9]+[^ ]{4,} [0-9]+ 0 R' out.pdf  # matches: font names in the page resources followed by a 4+ character suffix
without the primitive: no match (plain /F31 /F32 /F35 instead)
