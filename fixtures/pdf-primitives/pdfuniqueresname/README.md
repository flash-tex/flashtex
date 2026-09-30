\pdfuniqueresname=1 gives each page-resource name a unique suffix (/F31 becomes /F31F3qA10); the suffix is stable across rebuilds with SOURCE_DATE_EPOCH=0.
qpdf --qdf --object-streams=disable --normalize-content=n --deterministic-id main.pdf out.pdf && grep -a -o '/F[0-9][0-9]*[A-Za-z][A-Za-z0-9]*' out.pdf  # matches: /F31F3qA10 /F32F3qA10 /F35F3qA10
without the primitive: no match (plain /F31 /F32 /F35 instead)
