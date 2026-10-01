# PNG parser fuzzer (`tools/fuzz/parsers/png.py`)

Crash fuzzer for PNG image handling in the candidate pdfTeX engine: each
iteration mutates a seed PNG, embeds it in a minimal job
(`\pdfximage{fuzz.png}\setbox0\hbox{\pdfrefximage\pdflastximage}`
`\shipout\box0 \bye`, verified against real pdfTeX) and runs only the
candidate. Classes: `crash` (signal, exit 101, or `panicked at`), `hang`
(over `--timeout`), `ok`, `graceful-error`. Crashes and hangs are saved to
`OUT/<class>/<sha256-prefix>.png` with a `.json` sidecar (seed, mutation,
returncode, last output line, panic location) and deduped by panic location
or signal.

Seeds: `example-image.png`, `example-image-a.png` (colour type 0) and
`example-grid-100x100pt.png` (colour type 3) from the TeX Live `mwe` tree
(read at run time via `kpsewhich`, never copied in) plus two synthetic PNGs
built with `zlib`/`struct` (1x1 gray interlaced, 2x2 palette). Mutations:
byte flips, truncation, chunk delete/duplicate, IHDR width/height/depth/type/
interlace edits, length fields set to 0/1/0xFFFF/0x7FFFFFFF/0xFFFFFFFF, CRC
corrupt/zero/drop, IDAT reorder-or-split, IEND drop, huge tEXt and
gAMA/cHRM/sRGB/iCCP chunks.

```sh
python3 tools/fuzz/parsers/png.py --iterations 300 --seed 1 \
  --out /tmp/png-out --timeout 10
python3 -m unittest discover -s tools/fuzz/parsers
```
