The = map entries select which font file is embedded (BaseFont follows the embedded file's internal name, CMSL10/CMBXTI10); a bare or + entry cannot override the default map, which is read first.
qpdf --qdf --object-streams=disable --normalize-content=n --deterministic-id main.pdf out.pdf && grep -a -e 'CMSL10' -e 'CMBXTI10' out.pdf  # matches: subset BaseFonts of the swapped-in files
without the primitive lines: no match (plain CMR10/CMBX10 embedded instead)
