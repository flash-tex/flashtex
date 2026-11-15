# `\XeTeXinputencoding` with ICU converters (issue #2382)

Each directory is one case of `tests/input_encoding_icu.rs`: `p.tex` (and
the files it reads), and `p.log` and `p.xdv` as TeX Live 2026's xetex made
them (MacTeX 2026, macOS arm64, ICU 78.2; 2026-11-15):

```sh
SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 xetex -ini -etex -no-pdf -interaction=nonstopmode p.tex
```

The test runs the port the same way and compares the log from its `**`
line on (TeX Live names the file `./p.tex`, the port's `cwd` resolver
`p.tex`), and the XDV file byte for byte.

- `sbcs`, `unicode`: for each converter name, `\XeTeXdefaultencoding` names
  it, `\openin` opens a data file `dN.dat` and `\readline` reads its lines
  (`input_line`'s `ICUMAPPING` branch); the log shows each line as read and
  its character codes. `sbcs` reads the bytes 00–FF (but LF and CR) through
  single-byte tables (with `swaplfnl`, ICU's name matching, and names ICU
  does not open); `unicode` reads ill-formed and BOM-led lines through the
  UTF-8, CESU-8, US-ASCII, UTF-16 and UTF-32 converters; `mbcs` reads
  random lines, lead and trail byte sweeps, GB 18030 four-byte forms, SI/SO
  sequences and truncated sequences through multi-byte tables (GB 18030,
  EBCDIC SI/SO `ibm-1390` and its DBCS-only `ibm-16684`, the DBCS-only
  `ibm-971`, `gsm-03.38` with its extension table, extension-only tables,
  EUC-TW), and CJK text (encoded by Python's codecs) through `Shift_JIS`,
  `EUC-JP`, `GBK`, `GB18030`, `Big5`, `EUC-KR` and others.
- The others switch `\XeTeXinputencoding` in the document itself: a UTF-16
  name over ASCII text (`! Emergency stop`, as TeX Live), text really in
  UTF-16 or UTF-32 (each line then ends in U+FFFD, the LF's other byte),
  CESU-8, ill-formed UTF-8, cp1252, CR line ends, and
  `\XeTeXdefaultencoding` for an `\input` file.
