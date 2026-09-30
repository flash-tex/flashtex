# `display-list-v3`: the preview wire format and the engine-host protocol

- **Status:** version 3.1, implemented. 3.0: lane P3-DISPLAYLIST
  (2026-09-29); 3.1 (the resident, incremental host: §6): lane
  P3P4-HOST-UNIFY (2026-09-29). Producer: `crates/flashtex-engine`
  (`src/displaylist/`, `src/host/`).
  Reference decoder and client: `crates/display-list-v3` (Rust crate
  `flashtex-display-list`).
- **Licence:** this specification and the reference crate are **MIT**. A
  client needs nothing else: no engine code, no GPL code (DESIGN.md §3).
- **Governing design:** [DESIGN.md](../design/engine-v2/DESIGN.md) §3 (the
  licence boundary is a process boundary with a public, versioned protocol),
  §6.1 (display list), §6.2 (preview renderer).

The Mac app (MIT) never links the engine (GPL-2.0-or-later). It starts the
engine host, `flashtex-host`, and talks to it over a Unix socket. For each
compile the host streams one **page** message per `\shipout`, as the engine
ships the page out, plus the **fonts**, **images** and **source spans** the
pages use, **diagnostics**, and a final **done**. Everything a page shows is
in its message: positioned glyphs, rules, vector paths, images, reusable
forms, clipping, colour, link rectangles and destinations, each item tagged
with the source position that produced it.

The host keeps one **resident, incremental engine** per document (DESIGN.md
§5): after the first compile, a `COMPILE` that carries the editor's edits
re-typesets from the last page before the edit, so the edited page arrives
first, and stops once the document is the same as before from there on.

`display-list-v3` extends `display-list-v2`
([rendering-v2 proposal](../contracts/rendering-v2-proposal.md),
`apps/mac/Sources/FlashTeXProtocol/RenderingV2.swift`): v2's glyph runs with
absolute origins, rules, paths with clips, images and links remain, now
produced by the pdfTeX-compatible engine and exact to the PDF. What changes:

| | v2 | v3 |
|---|---|---|
| producer | the old layout engine | the engine, from the page's PDF content stream |
| encoding | one JSON line per document | binary frames, one per page, streamed at ship-out |
| units | `bp_2pow20` ticks | scaled points (sp) for positions; the PDF's own numbers for matrices and paths |
| positions | the old engine's | **exactly pdflatex's PDF** (§4.2), checked on every parity fixture |
| fonts | OpenType/TrueType bytes | the Type 1 programs pdflatex embeds, plus the encoding (§5.1) |
| text/source | clusters with byte ranges | per-item source spans (file, line) and per-glyph column (§5.3) |
| caching | none | per-page content hash (§4.6) |

## 1. Conventions

- All integers are **little-endian**. `u8 u16 u32` unsigned; `i32` two's
  complement; `f64` IEEE 754 binary64.
- Strings in binary bodies are byte strings with a length prefix; in JSON,
  UTF-8.
- JSON bodies are RFC 8259 objects. A reader ignores keys it does not know.
- **sp** (scaled point): 1/65536 TeX point; 1 bp (PDF point) = 65781.76 sp
  exactly (6578176/100).
- **Stream space**: the PDF's user space of the page (or form) at the
  start of its content stream: points (bp), origin at the bottom-left corner
  of the box, y up.
- **Page space** (for positions): sp, origin at the top-left corner of the
  box, x right, y down.

## 2. Framing

Every message, on the socket and in a file, is one frame:

```
u32   length      bytes that follow (kind + body), 1 ..= 2^28
u8    kind
...   body        length - 1 bytes
```

A file of display lists (`FLASHTEX_DISPLAY_LIST=path`, §6.6) is the engine's
frames back to back, without `HELLO`. A frame longer than 2^28 bytes, or of
length 0, is a corrupt stream: the reader stops (§7).

| kind | name | direction | body |
|---|---|---|---|
| `0x01` | `HELLO` | client → host | JSON (§6.2) |
| `0x02` | `COMPILE` | client → host | JSON (§6.3) |
| `0x03` | `CANCEL` | client → host | JSON `{"id": n}` |
| `0x04` | `BYE` | client → host | JSON `{}` |
| `0x41` | `HELLO` | host → client | JSON (§6.2) |
| `0x42` | `STARTED` | host → client | JSON (§6.4) |
| `0x43` | `FONT` | host → client | binary (§5.1) |
| `0x44` | `IMAGE` | host → client | JSON (§5.2) |
| `0x45` | `PAGE` | host → client | binary (§4) |
| `0x46` | `FORM` | host → client | binary (§4, form) |
| `0x47` | `SOURCES` | host → client | JSON (§5.3) |
| `0x48` | `DIAGNOSTIC` | host → client | JSON (§6.4) |
| `0x49` | `DONE` | host → client | JSON (§6.4) |
| `0x4A` | `ERROR` | host → client | JSON (§7) |
| `0x4B` | `PAGES` | host → client | JSON (§6.4; 3.1) |

## 3. Versioning

- The protocol name is `display-list-v3`; the version is `[major, minor]`,
  now `[3, 1]`.
- **Major** changes break readers (a new item opcode, a changed layout).
  Peers of different majors refuse each other at `HELLO` (§6.2).
- **Minor** changes only add: new JSON keys, new page sections (§4.1), new
  message kinds. A reader skips a section tag or message kind it does not
  know, and ignores unknown JSON keys. It never guesses at an unknown item
  opcode: that is a major change.
- **3.1** adds, for a client that says `[3, 1]` in its `HELLO`: the
  `COMPILE` keys `incremental`, `viewport`, `buffers`, `edits` and
  `export` (§6.3); the `STARTED` keys `mode`, `keep` and `incremental`, the
  `DONE` keys of §6.4 and the `PAGES` message; span re-declaration in
  `SOURCES` (§5.3). A 3.0 client sees 3.0 behaviour: every compile sends
  every page, in order.

## 4. `PAGE` and `FORM`

One `PAGE` per `\shipout`, in ship-out order. One `FORM` per `\pdfxform`
box the engine writes (pdfTeX writes a form when it is `\immediate`, or
after the first page that uses it). Both bodies have the same layout.

### 4.1 Layout

```
u32     index        PAGE: 0-based index of the page in the document: the
                     number of pages shipped before it (pdfTeX's
                     `total_pages`), which is the ship-out index of a
                     full run
                     FORM: the form's id (pdfTeX's `/Fm<n>` number)
u32     flags        bit 0 INCOMPLETE, bit 1 NO_GEOMETRY (§4.7)
i32     width        PAGE: TeX's page width (sp);  FORM: box width (sp)
i32     height       PAGE: TeX's page height (sp); FORM: height + depth (sp)
i32[10] counts       \count0 .. \count9 at ship-out (FORM: zeros)
f64[4]  box          the PDF box in bp: MediaBox (PAGE) or BBox (FORM),
                     [0 0 w h] as pdfTeX writes it
u8[32]  hash         content hash (§4.6)
u32     n            number of sections
n × {  u32 tag;  u32 len;  u8[len] data  }
```

The fixed header is 124 bytes. Section tags:

| tag | section | data |
|---|---|---|
| 1 | `MATRICES` | `u32 m`, then `m × f64[6]` (a b c d e f). Referenced as 1..m; **0 is the identity** and is not stored. |
| 2 | `PATHS` | `u32 n`, then n paths (§4.4) |
| 3 | `ITEMS` | the item stream (§4.3), to the end of the section |
| 4 | `LINKS` | `u32 n`, then n links (§4.5) |
| 5 | `DESTS` | `u32 n`, then n destinations (§4.5) |
| 6 | `UNSUPPORTED` | `u32 n`, then n × {`u16 len`, UTF-8 text}: what the page used that v3 cannot express |

Any other tag: skip `len` bytes (a later minor version's section).

### 4.2 Positions: exactly the PDF's

The engine does not lay anything out for the display list: it reads the
page's PDF content stream as pdfTeX's own `pdf_hlist_out`/`pdf_vlist_out`
wrote it, and interprets it as a PDF viewer does (text matrix, `Td`/`TD`/
`Tm`/`T*`, `TJ` adjustments, each glyph's advance from the font's
`/Widths`, `cm` and `q`/`Q`), in exact decimal arithmetic. A glyph origin
is then rounded **once** to page space:

```
x = round(X · 65781.76)            X, Y: the origin in stream space (bp)
y = round((H − Y) · 65781.76)      H: the box height (box[3]) in bp
round: to nearest, halves away from zero
```

So every glyph and rule in a display list is where **pdflatex's PDF** puts
it, to within half a scaled point (7.6 × 10⁻⁶ bp). This is checked on every
parity fixture against the pinned pdfTeX's PDF by an independent
implementation (`tools/displaylist/check_positions.py`, rational
arithmetic over the qpdf-normalised streams): **82/82 fixtures, 175 pages,
93,509 glyphs and 700 rules exact (0 sp)** (evidence:
`docs/evidence/display-list-v3-2026-09-29/`).

### 4.3 Items

Items are in **painting order**: the order the PDF paints. A reader
executes them in order; the state they set is sticky:

| opcode | item | operands | meaning |
|---|---|---|---|
| `0x01` | GLYPH | `u16 font, u16 code, i32 x, i32 y, u16 col` | draw glyph `code` of font resource `font` with its origin at (x, y) (page space), with the current glyph matrix, fill/stroke colour and text render mode. `col`: source column (§5.3) |
| `0x02` | RULE | `u8 kind, i32 x, i32 y, i32 w, i32 h` | a rule: left, top, width, height (page space, sp), §4.4 |
| `0x03` | PATH | `u32 n` | paint path n of `PATHS` |
| `0x04` | CLIP | `u32 n` | intersect the clip with path n (until the matching RESTORE) |
| `0x05` | IMAGE | `u32 id, u32 matrix` | draw image resource `id`: the matrix maps the unit square to stream space |
| `0x06` | FORM | `u32 id, u32 matrix` | draw form `id`: the matrix maps the form's stream space to this stream space |
| `0x07` | SAVE | | save the graphics state (clip, fill/stroke colour, text render mode) |
| `0x08` | RESTORE | | restore it |
| `0x09` | FILL_COLOR | `u8 n, f64[n]` | n = 1 DeviceGray, 3 DeviceRGB, 4 DeviceCMYK |
| `0x0A` | STROKE_COLOR | `u8 n, f64[n]` | the same, for stroking |
| `0x0B` | MATRIX | `u32 n` | the glyph matrix for the following glyphs: entries a b c d of matrix n (§4.4) |
| `0x0C` | SPAN | `u32 span` | the source span of the following items (0: none) (§5.3) |
| `0x0D` | TEXT_RENDER | `u8 mode` | PDF text render mode: 0 fill, 1 stroke, 2 fill then stroke, 3 invisible |
| `0x0E` | UNSUPPORTED | `u32 n` | entry n of `UNSUPPORTED` was skipped here |

The state at the start of every page and form: fill and stroke colour
DeviceGray 0 (black), text render mode 0, glyph matrix unset, span 0, the
clip the whole box. SAVE/RESTORE scope the colours, the text render mode
and the clip, as PDF's `q`/`Q` do; the glyph matrix and the span are not
graphics state and are not restored.

### 4.4 Glyphs, rules, paths

**Glyphs.** A glyph is drawn exactly as the PDF draws it. Its outline, in
glyph space, is mapped by the font's `FontMatrix` (for Type 1,
`[0.001 0 0 0.001 0 0]`, or `font_matrix` of §5.1) to text space, then by
the glyph matrix `[a b c d]` to stream space, then placed at the origin.
The glyph matrix is the linear part of PDF's text rendering matrix
`[Tfs·Th 0 0 Tfs 0 0] × Tm × CTM`: for upright text it is
`[size 0 0 size]` with `size` the `Tf` size in bp (e.g. 9.9626 for
10 pt), and it carries font expansion (`Tm`), rotation and scaling. A
client drawing in a top-left, y-down space in points draws an outline
point whose text-space coordinates (after the `FontMatrix`) are (u, w) at

```
X = x / 65781.76 + u·a + w·c
Y = y / 65781.76 − (u·b + w·d)
```

**Rules.** pdfTeX draws a rule (`pdf_set_rule`) as a filled rectangle, or,
when it is at most 1 bp thick, as a stroked line with butt caps. The item
keeps which, so that a rasteriser draws the same pixels as the PDF:

| kind | PDF | draw |
|---|---|---|
| 0 FILL | `x y w h re f` | fill the rectangle with the fill colour |
| 1 STROKE_H | `0 0 m w 0 l S`, line width = h | stroke the horizontal centre line from left to right, line width h, butt caps, stroke colour |
| 2 STROKE_V | `0 0 m 0 h l S`, line width = w | stroke the vertical centre line, line width w, butt caps, stroke colour |

A RULE is exactly one of those shapes under a CTM without rotation or
scaling, whatever drew it (pdfTeX's rules, leaders, or a `\pdfliteral`
that draws the same). Its rectangle is the area the PDF covers; each of its
four edges is rounded to sp independently (as §4.2), and `w`, `h` are the
differences.

**Paths** (`\pdfliteral` graphics: TikZ/pgf, `\pdfsetmatrix`, colour
boxes, …):

```
u8   paint     bits: 1 fill (nonzero), 2 fill (even-odd), 4 stroke,
               8 clip (nonzero), 16 clip (even-odd)
u32  matrix    the CTM: maps the segments' user space to stream space
if paint & 4:
  f64 width; u8 cap; u8 join; f64 miter; u16 k; f64[k] dash; f64 phase
u32  s         segments
s × { u8 op; f64 coordinates }
     0 move x y   1 line x y   2 curve x1 y1 x2 y2 x3 y3   3 close
```

Coordinates and stroke parameters are the PDF's numbers, in user space;
the client concatenates the matrix (and its own flip), then fills and/or
strokes with the current colours, exactly as the PDF paints. A CLIP item
names a path whose paint bits are the clip bits only.

### 4.5 Links and destinations

Links are pdfTeX's link annotations of the page (`\pdfstartlink`, hyperref),
as it will write them, with their final rectangles:

```
i32[4] rect     left, top, right, bottom (page space, sp; includes the link margin)
u32    span     source span of the \pdfstartlink
u8     kind     1 goto name, 2 goto num, 3 goto page, 4 URI, 5 raw action, 6 thread
u32 fl; u8[fl]  file (`goto file(...)`), or empty
u32 dl; u8[dl]  data: the name (1), the number in decimal (2),
                "N spec" (3: page N, then the destination spec),
                the URI (4), the action dictionary as pdfTeX writes it (5),
                the thread id (6)
```

Destinations (`\pdfdest`, hyperref anchors) on the page:

```
u8     named    1: name is a name; 0: a number in decimal
u32 nl; u8[nl]  name
u8     kind     0 xyz, 1 fit, 2 fith, 3 fitv, 4 fitb, 5 fitbh, 6 fitbv, 7 fitr
i32[4] rect     left, top, right, bottom (page space, sp); xyz uses left, top
i32    zoom     xyz zoom in thousandths, 0 = keep
```

A `goto name` link resolves to the page whose `DESTS` hold that name.

### 4.6 Content hash

`hash` is SHA-256 over what the page draws, so that a client can cache a
rasterised page across compiles and sessions:

```
"display-list-v3 content\0"
u8   1 for a FORM, 0 for a PAGE
i32  width, i32 height, f64[4] box
for MATRICES, PATHS, ITEMS', UNSUPPORTED:   u64 length, then the section data
     ITEMS' = ITEMS without SPAN items and with every GLYPH's col = 0
for each font id the items use, in order of first use:   u16 id, u8[32] key
for each image id the items use, in order of first use:  u32 id, u8[32] key
```

Source spans do not change the hash (editing a line above a page shifts its
spans, not its pixels); resource *keys* (§5), not ids, identify what is
drawn. Forms are referenced by id: a client caching a page that draws forms
keys it with the forms' hashes too.

### 4.7 Flags

- **INCOMPLETE**: the stream used something v3 cannot express; the
  `UNSUPPORTED` section says what (`gs` — transparency or other extended
  graphics state, `sh` — shading, `inline image`, a pattern or separation
  colour space, `Tr 4–7` — text clipping, an XObject that is not pdfTeX's
  `/Im` or `/Fm`, an unknown operator), and an `UNSUPPORTED` item marks
  where. The other items are still exact. A client that needs the page
  pixel-identical renders the PDF page instead (`DONE.pdf`).
- **NO_GEOMETRY**: the page was shipped in `\pdfdraftmode` (no content
  stream exists): the header is valid, there are no items.

## 5. Resources

Ids are valid for **one compile**, or, for an incremental client (§6.3),
for the connection until a `STARTED` says `"keep": false`. Every resource
a `PAGE` uses is sent before that `PAGE`: a `FONT` or `IMAGE` for an id
the client has not been sent, or that stood for another resource (a
later `FONT` for an id **rebinds** it). A client resolves a page's ids
when the page arrives, so a rebinding never changes a page it holds. A
`FORM` a page uses may come after the page (pdfTeX writes forms after the
page that first uses them) but before the next `PAGE` or `DONE`.

### 5.1 `FONT`

One per PDF font object (`/F<n>`) a page or form uses, before its first use:

```
u32     id       glyphs' `font` (pdfTeX's internal font number n of /F<n>)
u8[32]  key      identifies program + encoding + transform across compiles
u32 jl; u8[jl]   JSON (below)
u32 pl; u8[pl]   the font program (empty: see "held" below)
```

| JSON key | meaning |
|---|---|
| `pdf_name` | the PDF resource name, e.g. `F41` |
| `tex_name`, `tex_size` | the TFM name and its size in sp (the font that owns `/F<n>`; other sizes of it share the resource, their size is in the glyph matrix) |
| `ps_name` | PostScript name from the font map |
| `format` | `type1`: the program is a Type 1 font file (PFB if it starts with 0x80, else PFA); `none`: not embedded (a base-14 font the viewer supplies: draw with the named font); `truetype`, `opentype`, `type3` are reserved (the engine refuses those fonts today) |
| `file` | the font file the engine read |
| `program_sha256`, `program_bytes` | of the complete program |
| `encoding` | 256 glyph names: code → glyph. From the font map's `.enc` file when the font is re-encoded, else the program's built-in `/Encoding` |
| `slant`, `extend` | the map entry's SlantFont/ExtendFont ×1000 (0 = none) |
| `font_matrix` | when slanted or extended: the `/FontMatrix` pdfTeX writes into the embedded font, as the PDF's text ("a b c d e f"); use it instead of the program's |

**Drawing a glyph:** `name = encoding[code]`; draw the program's charstring
of that name. The PDF embeds a *subset* of this program (writet1) whose
charstrings, for every glyph the document uses, are the program's own: the
outlines, and so the pixels, are the same.

**Held programs:** `COMPILE.have_fonts` lists keys the client already has;
for those the host sends the `FONT` frame with an empty program
(`program_bytes` still gives the real length). Measured: 2.5 MB of fonts
for a first compile of `hyperref-toc`, 116 kB per compile after.

`key` = SHA-256(`"display-list-v3 font\0"`, format, `0x00`,
SHA-256(program), each of the 256 names followed by `0x00` (when there is an
encoding), `i32 slant`, `i32 extend`).

### 5.2 `IMAGE`

JSON, before the first `PAGE` that draws it:

| key | meaning |
|---|---|
| `id` | the IMAGE item's id (pdfTeX's `/Im<n>` number) |
| `key` | hex; changes when the file (path, size, mtime) or the selection changes |
| `type` | `png`, `jpeg`, `jbig2`, `pdf` |
| `file` | absolute path of the image file the engine read |
| `width`, `height` | pixels (PDF: the box, in bp) |
| `rotate`, `x_res`, `y_res` | as pdfTeX read them |
| `page`, `page_box`, `orig_x`, `orig_y` | PDF only: the page included, the box (`media`, `crop`, `bleed`, `trim`, `art`) and its origin |

The IMAGE item's matrix maps the unit square to stream space, exactly as
the PDF's `cm` before `/Im Do`. For a PDF image the unit square stands for
the included page's box (`width` × `height` bp from `orig_x`, `orig_y`);
draw that box of that page of the file. The engine copies PNG and JPEG data
unchanged where pdfTeX does, so decoding the file gives the PDF's pixels.

### 5.3 `SOURCES`: SyncTeX-equivalent source mapping

```json
{"files": [[1, "/abs/path/main.tex"], ...], "spans": [[7, 1, 42], ...]}
```

`files`: id → absolute path. `spans`: `[span, file, line]`, 1-based lines.
Each entry is sent once per compile (per connection, for an incremental
client), before the first page that uses it. Span ids are names that
outlive compiles: after an edit that moves lines, the next compile's
`SOURCES` **re-declares** the spans the client holds that moved (a later
entry for a span id replaces the earlier one), so pages kept from before
the edit point at their lines.

Every item carries the span of the SPAN item before it; a GLYPH also
carries `col`, the 0-based byte column. What they mean — the engine records,
for every node it allocates, where it was reading at that moment:

- a character typed in the file: its line and column;
- a character from a macro (`\section{Title}`, `\LaTeX`, inputenc's UTF-8):
  the line and column where the macro call ended;
- a word the hyphenator rebuilt: the position of its first letter;
- a `\copy`: the original's position;
- material the output routine makes (running heads, page numbers, the
  page's own boxes): **no span** (0), rather than a wrong one;
- `col = 0xFFFF`: no column is known.

**Forward search** (source → page): find the glyphs whose span is
(file, line) and whose `col` is nearest the caret. **Inverse search**
(click → source): the glyph under the point gives (file, line, col).
Spans are ids, not byte offsets: an edit shifts lines, and the next
compile's `SOURCES` say where every span now is.

## 6. The engine host and its socket

### 6.1 Transport

`flashtex-host --socket PATH [--engine PATH] [--format NAME]... [--no-warm]
[--s0-cache DIR] [--budget BYTES] [--timed SECONDS]` first finds the TeX
Live the engine will read (without a shell environment: the app's PATH is
launchd's) or the bundle, and makes each format ready (default
`pdflatex`): the engine loads it once, exactly as a compile will, from
`FLASHTEX_FORMATS` or else the format cache, which builds it from that TeX
Live as fmtutil does the first time (about 4 s) and validates it after
(about 0.1 s including the load). It prints one JSON line saying what it
chose, then (unless `--no-warm`) warms the resident engine up on a
one-page document (kpathsea, the font map, the format: `{"warm_ms": …}`),
then listens on a Unix-domain stream socket at `PATH` (mode 0600) and
prints `flashtex-host: listening on PATH` when ready.

The host runs **one resident engine**, on one thread, for one document at
a time (the job: `root`, `main`, `format`, `output_dir`, `jobname`,
`shell_escape`); a `COMPILE` for another job replaces it. The app starts
one host per open document, as a separate process (the licence boundary is
this process boundary). Compiles run one at a time, in the order they
arrive. With `--s0-cache DIR` (or `FLASHTEX_S0_CACHE`), each document's
begin-document snapshot S₀ is saved there after a full run, and the first
compile of the document in a new host starts from it when nothing it read
has changed (DESIGN.md §1.2's reopen target); each save prints one line,
`flashtex-host: {"saved_s0": PATH, "bytes": N, "ms": T}`. `--budget` and `--timed` are
the checkpoints' memory budget (default 1 GiB) and timed interval (default
0.02 s). `--engine` names the engine program that `export` compiles and
the format preparation run (default: `flashtex-host` itself, which runs as
the engine when invoked as `pdftex`).

### 6.2 `HELLO`

The client speaks first:

```json
{"protocol": "display-list-v3", "version": [3, 1], "client": "FlashTeX 1.2"}
```

The host answers with its own `HELLO`, or with `ERROR` `{"code":
"version"}` and closes if the major differs:

```json
{"protocol": "display-list-v3", "version": [3, 1], "server": "flashtex-host 0.1.0",
 "engine": "pdfTeX 3.141592653-2.6-1.40.29 (FlashTeX engine)",
 "capabilities": ["compile", "cancel", "diagnostics", "font-programs", "have-fonts",
                  "resident", "incremental", "buffers", "edits", "viewport",
                  "pages-status", "export"],
 "texmf": {"texlive": "/Library/TeX/texbin (PATH) -> /usr/local/texlive/2026/bin/universal-darwin",
           "resolver": "kpathsea (/Library/TeX/texbin)",
           "formats": [{"name": "pdflatex", "status": "ready", "ms": 93.8}]}}
```

`texmf.texlive` is null when no TeX Live was found (the resolver is then
the bundle, if one is configured); a format whose `status` is `failed`
carries `error`, and compiles with it will fail: the app says so before
the user compiles.

### 6.3 `COMPILE`

```json
{"id": 12, "root": "/Users/me/paper", "main": "main.tex",
 "format": "pdflatex", "shell_escape": "default",
 "output_dir": "/Users/me/Library/Caches/FlashTeX/paper", "jobname": "main",
 "have_fonts": ["3f0c…", "…"]}
```

| key | required | meaning |
|---|---|---|
| `id` | yes | the client's number for this compile; every reply carries it |
| `root` | yes | the project directory (absolute); the engine runs there |
| `main` | yes | the main file, relative to `root` (no `..`) |
| `format` | no | format name, default `pdflatex` |
| `shell_escape` | no | `\write18`: `default` (texmf.cnf's: restricted in TeX Live), `off`, `restricted`, `on` |
| `output_dir` | no | where the PDF, log and auxiliary files go (default: a per-connection temporary directory) |
| `jobname` | no | default: the main file's name |
| `have_fonts` | no | font keys (hex) the client holds (§5.1) |
| `incremental` | no | 3.1: `true` keeps the pages and resource ids of this connection's earlier compiles of the document: the host sends only pages that changed, and `PAGES` (default `false`: every page, every compile, as in 3.0) |
| `viewport` | no | 3.1: the page (0-based) the client shows; the run stops there first, says so (`PAGES`), then typesets the rest |
| `buffers` | no | 3.1: `[{"path", "text"}]`: files as the editor has them (path relative to `root`); the host writes each to its file, as saving would, before compiling |
| `edits` | no | 3.1: `[{"path", "offset", "delete", "insert"}]`: byte splices of files under `root`, applied in order, before compiling |
| `export` | no | `true`: a one-shot run of the engine as a child process instead of the resident engine: `DONE.pdf` is the compressed PDF pdflatex would write (P-T2), not the preview's |

The engine runs as pdflatex would:
`pdftex -fmt=FORMAT -interaction=nonstopmode -file-line-error -output-directory=DIR -jobname=JOB [shell flag] MAIN`,
in the resident engine: the first compile of a document is a full run; a
later one restarts from the last checkpoint before what changed (the
edits, or any file the run read) and stops once the engine state equals
the previous run's (DESIGN.md §5.3), so a keystroke re-typesets a page or
two. The PDF (`DONE.pdf`) is the preview's: its streams are stored, not
compressed; `export` makes the compressed one.
One run: the client decides when to rerun (e.g. after `\label` changes:
`DONE.mode` `incremental` or `cold` after an `.aux` change, `unchanged`
when nothing changed).
**A `COMPILE` while one is running supersedes it**: the running compile
goes on (a page is never interrupted) without sending, its `DONE` says
`cancelled`, and the next compile sends what is current; a compile
superseded before it started only applies its edits. An `export` compile
is killed as in 3.0.

### 6.4 Replies

For one compile the host sends, in this order: `STARTED`; then, interleaved
as the engine produces them, `FONT`, `IMAGE`, `SOURCES`, `PAGE`, `FORM`,
`PAGES` and `DIAGNOSTIC`; then exactly one `DONE`. Pages arrive while the
engine is still typesetting later ones, **in page order**: the first page
the compile re-typesets (the edited one) first; pages it did not
re-typeset (before the restart point, or after convergence) are sent from
the host's cache in their place, unless the client holds them already
(`incremental`).

`STARTED`: `{"id", "pid", "argv", "output_dir", "mode", "keep",
"incremental"}`. `mode`: `resident` or `export`. `keep` (3.1): `true` when
the client's pages and resource ids from earlier compiles on this
connection stay valid; `false`: drop them first.

`PAGES` (3.1, incremental clients):
`{"id", "count", "complete", "current": [[first, last], ...], "stale": [[first, last], ...]}`
— which of the pages the client holds are current and which are left from
an earlier compile, still to be re-typeset (show them marked stale). Sent
after the first re-typeset page, after a `viewport` stop, and before
`DONE` (`complete: true`, all `count` pages current; a client drops pages
at or past `count`).

`DIAGNOSTIC`: `{"id", "severity": "error"|"warning", "message", "file"?, "line"?}`
— TeX errors (`file:line: message` or `! message`) and LaTeX/package
warnings from the engine's terminal output.

`DONE`:

```json
{"id": 12, "status": "ok", "exit_code": 0, "pages": 3, "bytes": 116038,
 "diagnostics": 2, "elapsed_ms": 244.1, "first_page_ms": 207.2,
 "pdf": "/…/main.pdf", "log": "/…/main.log"}
```

`status`: `ok` (exit 0), `error` (TeX reported errors; pages that shipped
out are valid), `cancelled`, `failed` (the engine died). For `export`,
`pdf` is the exported PDF, byte-identical to pdflatex's for the parity
fixtures (P-T2); for the resident engine, the preview's PDF. 3.1 adds, for
the resident engine: `mode` (`cold`, `incremental`, `unchanged`, `open`:
from a persisted S₀), `restart_page` (pages before the restart point),
`converged_at` (the page after which the previous run's pages were kept, or
null), `typeset_pages` (pages this compile shipped), `first_page_ms`
(`COMPILE` to the first re-typeset page on the socket), `viewport_ms`,
`run_ms`, `keep`, and `cold_reason` when a full run was needed.

### 6.5 `CANCEL`, `BYE`

`CANCEL {"id"}` stops that compile: an `export` process is killed; the
resident engine finishes the page it is on and the rest of the run without
sending (its checkpoints stay valid for the next compile). Its `DONE` says
`cancelled`. A `CANCEL` for an id that is not running is ignored. `BYE`
(or closing the socket) ends the connection and cancels a running compile.

### 6.6 Without the host

The engine writes the same frames when run directly:
`FLASHTEX_DISPLAY_LIST=file.dl3 pdftex -fmt=pdflatex main.tex` (or
`fd:N` for an inherited descriptor, which is how the host runs it), and
`FLASHTEX_DISPLAY_LIST_HAVE_FONTS=key,key` for held fonts. `dl3-dump
file.dl3` prints such a file as JSON lines.

## 7. Errors

- **Decoding fails closed.** A frame of bad length, a body shorter than its
  layout, an unknown item opcode, an item that names a path, matrix or
  unsupported entry that does not exist, a colour of other than 1, 3 or 4
  components: the reader rejects the message and treats the connection as
  broken (reconnect and recompile). It never draws a partial page it could
  not decode.
- **Unknown** message kinds, section tags and JSON keys are skipped.
- `ERROR` `{"id"?, "code", "message"}`, codes: `version` (major mismatch;
  the host closes), `protocol` (not a frame, or not `HELLO` first; the host
  closes), `request` (a `COMPILE` it refuses: missing or bad field, `root`
  not an absolute directory, `main` outside it, the engine cannot start;
  the connection stays usable).
- **TeX errors are not protocol errors**: they arrive as `DIAGNOSTIC`s and
  `DONE.status = "error"`; the pages shipped before and after the error are
  valid.
- A page the engine could not express fully is flagged INCOMPLETE (§4.7),
  never silently approximated.

## 8. Swift decoding sketch

What the app lane needs is small: a byte reader, the frame loop, the page
decoder, and a Unix socket. No engine code is involved. This sketch
type-checks with `swiftc` (Swift 6, macOS 26) and, driven by a five-line
loop over a file of frames, decodes the display lists of all 82 parity
fixtures to the same 175 pages, 102,151 items and 93,509 glyphs as the Rust
reference decoder.

```swift
import Foundation

struct DLReader {
    let data: Data
    var i = 0
    init(_ d: Data) { data = d }
    mutating func take(_ n: Int) throws -> Data {
        guard i + n <= data.count else { throw DLError.truncated }
        defer { i += n }
        return data.subdata(in: data.startIndex + i ..< data.startIndex + i + n)
    }
    mutating func u8() throws -> UInt8 { try take(1).first! }
    mutating func u16() throws -> UInt16 { try le(UInt16.self) }
    mutating func u32() throws -> UInt32 { try le(UInt32.self) }
    mutating func i32() throws -> Int32 { Int32(bitPattern: try u32()) }
    mutating func f64() throws -> Double { Double(bitPattern: try le(UInt64.self)) }
    private mutating func le<T: FixedWidthInteger>(_: T.Type) throws -> T {
        let d = try take(MemoryLayout<T>.size)
        return T(littleEndian: d.withUnsafeBytes { $0.loadUnaligned(as: T.self) })
    }
}

enum DLError: Error { case truncated, badOpcode(UInt8), badReference, badColor }

enum DLItem {
    case glyph(font: UInt16, code: UInt16, x: Int32, y: Int32, col: UInt16)
    case rule(kind: UInt8, x: Int32, y: Int32, w: Int32, h: Int32)
    case path(Int), clip(Int), image(id: UInt32, matrix: Int), form(id: UInt32, matrix: Int)
    case save, restore, fill([Double]), stroke([Double]), matrix(Int), span(UInt32)
    case textRender(UInt8), unsupported(Int)
}

struct DLPage {
    var index: UInt32 = 0, flags: UInt32 = 0, width: Int32 = 0, height: Int32 = 0
    var counts = [Int32](repeating: 0, count: 10)
    var box = [Double](repeating: 0, count: 4)
    var hash = Data()
    var matrices: [[Double]] = []          // 1-based on the wire; 0 = identity
    var items: [DLItem] = []
    // paths, links, dests, unsupported: same pattern (spec §4.4, §4.5)
}

func decodePage(_ body: Data) throws -> DLPage {
    var r = DLReader(body), p = DLPage()
    p.index = try r.u32(); p.flags = try r.u32()
    p.width = try r.i32(); p.height = try r.i32()
    for k in 0..<10 { p.counts[k] = try r.i32() }
    for k in 0..<4 { p.box[k] = try r.f64() }
    p.hash = try r.take(32)
    for _ in 0..<(try r.u32()) {
        let tag = try r.u32(), len = Int(try r.u32())
        var s = DLReader(try r.take(len))
        switch tag {
        case 1:  // MATRICES
            for _ in 0..<(try s.u32()) { p.matrices.append(try (0..<6).map { _ in try s.f64() }) }
        case 3:  // ITEMS
            while s.i < len {
                let op = try s.u8()
                switch op {
                case 0x01: p.items.append(.glyph(font: try s.u16(), code: try s.u16(),
                                                 x: try s.i32(), y: try s.i32(), col: try s.u16()))
                case 0x02: p.items.append(.rule(kind: try s.u8(), x: try s.i32(), y: try s.i32(),
                                                w: try s.i32(), h: try s.i32()))
                case 0x03: p.items.append(.path(Int(try s.u32())))
                case 0x04: p.items.append(.clip(Int(try s.u32())))
                case 0x05: p.items.append(.image(id: try s.u32(), matrix: Int(try s.u32())))
                case 0x06: p.items.append(.form(id: try s.u32(), matrix: Int(try s.u32())))
                case 0x07: p.items.append(.save)
                case 0x08: p.items.append(.restore)
                case 0x09, 0x0A:
                    let n = Int(try s.u8())
                    guard [1, 3, 4].contains(n) else { throw DLError.badColor }
                    let c = try (0..<n).map { _ in try s.f64() }
                    p.items.append(op == 0x09 ? .fill(c) : .stroke(c))
                case 0x0B: p.items.append(.matrix(Int(try s.u32())))
                case 0x0C: p.items.append(.span(try s.u32()))
                case 0x0D: p.items.append(.textRender(try s.u8()))
                case 0x0E: p.items.append(.unsupported(Int(try s.u32())))
                default: throw DLError.badOpcode(op)
                }
            }
        default: break  // PATHS, LINKS, DESTS, UNSUPPORTED as §4; unknown tags skipped
        }
    }
    return p
}

/// Frames from the host socket: 4-byte length, kind, body.
func readFrame(_ fd: Int32) throws -> (UInt8, Data)? {
    func readExactly(_ n: Int) throws -> Data? {
        var buf = Data(count: n), got = 0
        while got < n {
            let r = buf.withUnsafeMutableBytes { read(fd, $0.baseAddress! + got, n - got) }
            if r == 0 { if got == 0 { return nil }; throw DLError.truncated }
            if r < 0 { throw POSIXError(.init(rawValue: errno)!) }
            got += r
        }
        return buf
    }
    guard let head = try readExactly(4) else { return nil }
    let len = Int(head.withUnsafeBytes { UInt32(littleEndian: $0.loadUnaligned(as: UInt32.self)) })
    guard len >= 1, len <= 1 << 28, let rest = try readExactly(len) else { throw DLError.truncated }
    return (rest.first!, rest.dropFirst())
}

func connect(to path: String) throws -> Int32 {
    let fd = socket(AF_UNIX, SOCK_STREAM, 0)
    var addr = sockaddr_un(); addr.sun_family = sa_family_t(AF_UNIX)
    withUnsafeMutableBytes(of: &addr.sun_path) { raw in
        path.utf8CString.withUnsafeBytes { src in
            raw.copyMemory(from: UnsafeRawBufferPointer(rebasing: src.prefix(raw.count - 1)))
        }
    }
    let ok = withUnsafePointer(to: &addr) {
        $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
            Darwin.connect(fd, $0, socklen_t(MemoryLayout<sockaddr_un>.size))
        }
    }
    guard ok == 0 else { throw POSIXError(.init(rawValue: errno)!) }
    return fd
}
```

Drawing a decoded page into a `CGContext` whose space is top-left, y-down
points (as `GlyphRunRenderer` draws v2): flip once with
`ctx.concatenate(CGAffineTransform(a: 1, b: 0, c: 0, d: -1, tx: 0, ty: H))`
for stream-space items (paths, images, forms, glyph matrices), and place
glyph origins at `(x / 65781.76, y / 65781.76)` in the unflipped space; map
SAVE/RESTORE to `saveGState`/`restoreGState` and CLIP to `addPath` + `clip`.

## 9. What the app lane needs (checklist)

1. Start `flashtex-host --socket <per-document path> --s0-cache <app cache>/s0`
   (from the app bundle's helper directory, as a separate process) with
   `FLASHTEX_POOL` in its environment when a document opens; wait for its
   "listening" line; show `HELLO.texmf` (which TeX Live, whether the format
   is ready) in the app.
2. Say `[3, 1]` and compile with `"incremental": true`: on each keystroke
   (debounced as the app likes) send `COMPILE` with the changed file's
   `edits` (or its `buffers`) and the shown page as `viewport`; replace the
   pages that arrive, mark the `PAGES` stale ranges, drop pages past
   `count`, and apply re-declared spans from `SOURCES`.
3. Decode §2 frames and §4 pages (sketch above; the Rust crate is the
   reference, `crates/display-list-v3/src/page.rs`), resources (§5) and
   control messages; keep a font store keyed by `key`, send its keys as
   `have_fonts`.
4. Type 1 glyphs (DESIGN §6.2): Core Text cannot load Type 1 since
   Ventura, so the renderer converts each program once (to an in-memory
   CFF/OpenType font) or rasterises through FreeType, and draws
   `encoding[code]` with the glyph matrix; the gate is zero pixel
   difference against Core Graphics' rendering of `DONE.pdf`.
5. Rules by kind (§4.4), paths and clips, images from `file` (and PDF pages
   through `CGPDFDocument`), forms as cached sub-lists.
6. Pages flagged INCOMPLETE: render `DONE.pdf`'s page instead, or overlay.
7. Cache rasters by `hash` (§4.6), links from `LINKS`/`DESTS`, SyncTeX from
   `SOURCES` + SPAN/`col`.
8. Keep this behind a flag next to v2 until the preview parity gate passes.

## 10. Limits of version 3.1

- Extended graphics state (`gs`: transparency), shadings, patterns,
  separation colour spaces, inline images and text clipping are flagged
  INCOMPLETE, not expressed (2 of the 82 parity fixtures use `gs`).
- TrueType/OpenType and Type 3 (PK) fonts: the engine does not embed them
  yet, so they never reach a display list.
- DVI mode (`\pdfoutput=0`) and `\pdfdraftmode` pages carry no geometry.
- A `\pdfpageattr` that overrides `/MediaBox` is not reflected in `box`.
- One resident engine per host process, for one document at a time (the
  app runs a host per open document); an `export` runs its own process.
- The resident engine is not interrupted inside a page: a newer `COMPILE`
  waits for the running one's page (and, today, for the rest of its run,
  which the next compile then starts from).
- S₀ persisted with `--s0-cache` does not carry source spans: after a
  reopen, material made before `\begin{document}` (none that a page shows,
  in practice) has no span.
