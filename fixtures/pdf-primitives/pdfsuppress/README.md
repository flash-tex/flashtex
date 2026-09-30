\pdfsuppressptexinfo=-1 (bits 1/2/4/8 = Fullbanner/FileName/PageNumber/InfoDict) drops every PTEX.* key from the PDF; the three warning primitives silence log warnings (dupdest and dupmap are triggered below; pagegroup needs grouped-PDF inclusion and stays untriggered here).
qpdf --qdf --object-streams=disable --normalize-content=n --deterministic-id main.pdf out.pdf && grep -a 'PTEX' out.pdf  # no output: suppressed; log: grep -a 'same identifier' main.log also no match
without the primitives: 1 PTEX match (/PTEX.Fullbanner present) and the dupdest+dupmap warnings fire in main.log
