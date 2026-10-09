# `display-list-v3`: the preview wire format and the engine-host protocol

- **Status:** version 3.3. 3.0: lane P3-DISPLAYLIST
  (2026-09-29); 3.1 (the resident, incremental host: §6): lane
  P3P4-HOST-UNIFY (2026-09-29); 3.2 (external tools: bibtex, biber,
  makeindex, §6.3–§6.4): lane P5-EXTERNAL-TOOLS (2026-09-30). Page sections
  `ORIGINS` and `RULE_GEOMETRY` (§4.2, §4.4; host capability
  `exact-geometry`): lane J1 P3-ZERO-TOLERANCE (2026-10-02), gated by that
  capability like `progress-v1`, so it takes no minor number (protocol
  owner's ruling, DESIGN.md §13, 2026-10-05). 3.3 (the Typst host's additions E1–E8, DESIGN.md §15.4:
  §11): lane TYPST-T0T1 (2026-10-04), drafted in `typst-host/` by #1303 and
  #1335. Producers: `crates/flashtex-engine` (`src/displaylist/`,
  `src/host/`) for LaTeX, `typst-host/` (`flashtex-typst-host`) for Typst.
  Reference decoder and client: `crates/display-list-v3` (Rust crate
  `flashtex-display-list`).
- **Licence:** this specification and the reference crate are **MIT**. A
  client needs nothing else: no engine code, no GPL code (DESIGN.md §3).
- **Governing design:** [DESIGN.md](../design/engine-v2/DESIGN.md) §3 (the
  licence boundary is a process boundary with a public, versioned protocol),
  §6.1 (display list), §6.2 (preview renderer).

The Mac app (MIT) never links the engine (GPL-2.0-or-later). It starts the
engine host, `flashtex-host`, and talks to it over a reliable, ordered
byte stream (§6.1: a Unix-domain socket on macOS/Linux). For each
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
- **Paths** are UTF-8 strings, on every OS. Absolute paths (`root`,
  `output_dir`, `IMAGE.file`, `FONT.file`, `SOURCES.files`, `DONE.pdf`) are
  in the host OS's native form. Relative paths (`main`, `buffers[].path`,
  `edits[].path`) use `/` as the only separator, never `\`, and are
  resolved against `root`. A file name that is not valid Unicode on its OS
  (non-UTF-8 bytes on Unix, an unpaired surrogate on Windows) cannot be
  carried by version 3.
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
| `0x05` | `RESOLVE` | client → host | JSON (§11.6; 3.3, host capability `resolve-v1`) |
| `0x06` | `LOCATE` | client → host | JSON (§11.6; 3.3, host capability `resolve-v1`) |
| `0x07` | `PROFILE` | client → host | JSON (§6.9; host capability `profile-v1`) |
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
| `0x4C` | `TOOL` | host → client | JSON (§6.4; 3.2) |
| `0x4D` | `IMAGE_DATA` | host → client | binary (§11.5; 3.3, `accept` `image-data`) |
| `0x4E` | `RESOLVED` | host → client | JSON (§11.6; 3.3) |
| `0x4F` | `LOCATED` | host → client | JSON (§11.6; 3.3) |
| `0x50` | `PACKAGE` | host → client | JSON (§11.8; Typst host, `accept` `packages-v1`) |
| `0x51` | `PROFILE` | host → client | JSON (§6.9; the reply to a client's `PROFILE`) |
| `0x60` | `DIAG` | host → client | JSON (§6.7; `diag-v1`, capability-gated) |
| `0x70` | `PROGRESS` | host → client | JSON (§6.8; `progress-v1`, capability-gated) |

## 3. Versioning

- The protocol name is `display-list-v3`; the version is `[major, minor]`,
  now `[3, 3]`. `flashtex-host` (LaTeX) implements 3.2 and answers
  `[3, 2]` to any 3.x client: 3.3 (§11) adds nothing a LaTeX host must send.
  The reference client says `[3, 3]`.
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
- **3.2** adds the `COMPILE` key `external_tools`, the `TOOL` message, the
  `STARTED`/`DONE` key `cause` and the host capability `external-tools`
  (§6.3, §6.4): the host runs bibtex, biber and makeindex when a document
  needs them and compiles again with what they made. `TOOL` goes only to a
  client that says `[3, 2]`; a follow-up compile (`"cause": "tools"`) only
  happens for a `COMPILE` that allowed tools, which a 3.1 client never sends.
- **`progress-v1`** (2026-10-03): the `PROGRESS` heartbeat (§6.8),
  capability-gated like `diag-v1`, so it needs no minor number: a client
  that does not accept it sees nothing new.
- **`profile-v1`** (2026-10-06, lane PERF-MODES): performance modes
  (§6.9), capability-gated like `progress-v1`, so it needs no minor number:
  the `HELLO` key `profile` in both directions and the `PROFILE` messages.
  A host without the capability ignores the client's `HELLO.profile` (an
  unknown key) and never receives a `PROFILE` from a client that checks the
  capability first.
- **3.3** (§11; DESIGN.md §15.4, E1–E8) adds, all of it for the Typst
  host and none of it required of the LaTeX host: `FONT.format`
  `opentype` with glyph ids and variation coordinates (§11.1), page
  sections 8 `PAGE_META` and 10 `COLORSPACES` (§11.2, §11.3), section 7
  `ORIGINS` on every Typst page (§11.2; the section is exact geometry's,
  below, and has one layout for both hosts), item opcodes
  `0x0F`–`0x13` for colour spaces, constant alpha and the text line state
  (§11.3, §11.4), images from bytes (`IMAGE_DATA`, §11.5), on-demand
  source mapping (`RESOLVE`/`LOCATE`, §11.6) and, for a client that
  accepts `packages-v1`, the `PACKAGE` message (§11.8), plus `DIAGNOSTIC`
  keys `column` and `hints`. Sections and JSON keys follow the minor rules
  above. **The new opcodes are sent only to a client that said `[3, 3]`
  and listed the feature in its `HELLO` `accept`** (§11.7): an item opcode
  a reader does not know is otherwise a major change. A 3.1 or 3.2 client
  of the Typst host gets its glyph pages INCOMPLETE and draws `DONE.pdf`.
- **Exact geometry** (J1, 2026-10-02; gated by the `exact-geometry`
  capability like `progress-v1`, so it takes no minor number: protocol
  owner's ruling, DESIGN.md §13, 2026-10-05) adds page sections 7
  `ORIGINS` and 9 `RULE_GEOMETRY` (§4.1, §4.2, §4.4) and the host capability
  `exact-geometry`, which says every `PAGE` and `FORM` carries both. Both
  directions are handled without negotiation: a reader that does not know
  the sections skips them (§4.1) and draws from the sp positions as before;
  a reader that knows them, given a page from a writer that does not send
  them, draws from the sp positions. A section with any other count than
  one entry per GLYPH (per RULE) is refused (§7).

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
| 7 | `ORIGINS` | exact geometry (§3), and every 3.3 Typst page (§11.2): `u32 n`, then n × `f64[2]`: each GLYPH's origin (X, Y) in stream space, in item order (§4.2); the Typst host's `ORIGINS_F64` (#1335) is this section |
| 8 | `PAGE_META` | 3.3: UTF-8 JSON, the page's metadata (§11.2) |
| 9 | `RULE_GEOMETRY` | exact geometry (§3): `u32 n`, then n × `f64[7]`: what the PDF draws each RULE with, in item order (§4.4) |
| 10 | `COLORSPACES` | 3.3: `u32 n`, then n colour spaces, referenced as 1..n (§11.3) |

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

**`ORIGINS`** (section 7) carries the same origins unrounded, one `f64[2]`
per GLYPH item in item order: (X, Y) in stream space (bp, y up), **as the
reference PDF viewer computes them in binary64**, which is what a
rasteriser needs to put a glyph on the same side of every pixel edge as
the viewer's rendering of the PDF (DESIGN.md §6.2, zero tolerance). Half a
scaled point is not enough for that: a glyph whose outline meets a pixel
edge within 7.6 × 10⁻⁶ bp draws differently. Nor is the double nearest to
the exact decimal: the viewer's own arithmetic differs from it by a few
ulps, and at an edge those decide a pixel. The evaluation is the viewer's
(Core Graphics', measured on the parity fixtures):

- a number with k fraction digits (trailing zeros dropped) is its digits as
  an integer m times the double nearest to 10⁻ᵏ (`237.283` is
  `237283 × 0.001`, one ulp above the double nearest to 237.283); a `TJ`
  adjustment and a `/Widths` entry are the double nearest to the decimal;
- `cm`: CTM ← M × CTM; `Td`, `TD`, `T*`: Tlm ← [1 0 0 1 tx ty] × Tlm with
  e′ = tx·a + ty·c + e, f′ = tx·b + ty·d + f; `Tm` sets both matrices;
- after each glyph, tx = ((W / 1000) · Tfs + Tc [+ Tw for code 32]) · Tz / 100,
  and after a `TJ` number n, tx = ((−n / 1000) · Tfs) · Tz / 100; then
  Tm ← [1 0 0 1 tx 0] × Tm as above; W is the `/Widths` entry (for a
  Type 3 font, times the font's `FontMatrix` a × 1000);
- the origin is (Ts·c + e, Ts·d + f) of Tm × CTM (products row by column,
  each sum left to right, no fused multiply-add).

Rounding X and H − Y to sp as above gives the GLYPH's own x and y (to
within the few ulps). A client draws the glyph at (X, Y) in stream space
when the page has `ORIGINS`, else at its sp position.

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
| `0x0F` | FILL_COLOR_CS | `u32 cs, u8 n, f64[n]` | 3.3 (`accept` `color-spaces`): the fill colour in colour space `cs` of `COLORSPACES` (§11.3) |
| `0x10` | STROKE_COLOR_CS | `u32 cs, u8 n, f64[n]` | 3.3: the same, for stroking |
| `0x11` | FILL_ALPHA | `f64 a` | 3.3 (`color-spaces`): constant fill alpha, the PDF's `ca` (§11.3) |
| `0x12` | STROKE_ALPHA | `f64 a` | 3.3 (`color-spaces`): constant stroke alpha, the PDF's `CA` |
| `0x13` | LINE_STATE | `f64 width, u8 cap, u8 join, f64 miter, u16 k, f64[k] dash, f64 phase` | 3.3 (`accept` `line-state`): the line state stroked glyphs use (text render modes 1 and 2; §11.4) |

The state at the start of every page and form: fill and stroke colour
DeviceGray 0 (black), text render mode 0, glyph matrix unset, span 0, the
clip the whole box; 3.3: fill and stroke alpha 1, the line state of a new
PDF graphics state (width 1, butt caps, miter joins, miter limit 10, no
dash). SAVE/RESTORE scope the colours, the text render mode and the clip,
as PDF's `q`/`Q` do, and in 3.3 the alphas and the line state; the glyph
matrix and the span are not graphics state and are not restored.

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

**`RULE_GEOMETRY`** (section 9) carries, per RULE item in item order, the
numbers the PDF draws it with, read and combined as for `ORIGINS` (§4.2):
`[e, f, x, y, w, h, 0]` for FILL (`x y w h re f` under a CTM whose
translation is (e, f)), `[e, f, x0, y0, x1, y1, lw]` for STROKE_H and
STROKE_V (`x0 y0 m x1 y1 l S`, line width lw). A client draws the rule as
the PDF does: translate by (e, f), then fill that rectangle or stroke that
line with butt caps. (Drawn from the sp rectangle instead, a 0.249 bp
stroked rule's edge row differs from the PDF's by one coverage level at
5.25, 6.5 and 6.75 px/pt: measured on proof-practice-21242.)

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
i32[4] rect     left, top, right, bottom (page space, sp): the ones pdfTeX
                writes for the kind (xyz left, top; fith, fitbh top; fitv,
                fitbv left; fitr all four), the others 0
i32    zoom     xyz zoom in thousandths, 0 = keep; 0 for the other kinds
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
for ORIGINS, then RULE_GEOMETRY, if the page has it:   u64 length, then the section data
3.3: for PAGE_META, then COLORSPACES, if the page has it: u64 length, then the section data
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
| `format` | `type1`: the program is a Type 1 font file (PFB if it starts with 0x80, else PFA); `none`: not embedded (a base-14 font the viewer supplies: draw with the named font); `truetype`: the program is the TrueType file (`.ttf`, or a `.ttc` collection whose first font pdfTeX uses); `opentype`: the program is the OpenType (CFF) file (`.otf`) (3.3: with `glyph_ids`, a Typst font instance whose codes are glyph ids, §11.1); `type3`: a bitmap (PK) font pdfTeX writes as Type 3, the program is its glyphs as bitmaps (§5.1.1). The last three are sent only to a client that lists them in `COMPILE.font_formats` (§6.3); to another the `FONT` comes with an empty program, as if held |
| `file` | the font file the engine read |
| `program_sha256`, `program_bytes` | of the complete program |
| `encoding` | 256 glyph names: code → glyph. From the font map's `.enc` file when the font is re-encoded, else the program's built-in `/Encoding` |
| `slant`, `extend` | the map entry's SlantFont/ExtendFont ×1000 (0 = none) |
| `font_matrix` | when slanted or extended: the `/FontMatrix` pdfTeX writes into the embedded font, as the PDF's text ("a b c d e f"); use it instead of the program's. For `type3`: the Type 3 font's `/FontMatrix` ("s 0 0 s 0 0"), which maps the bitmaps' pixels to text space |
| `dpi` | `type3`: the resolution of the PK file (`\pdfpkresolution`, scaled with the font's size) |
| `subfont`, `cmap` | `truetype` subfont entries (`name@sfd@`): the Unicode (or other) character code of each of the 256 codes (-1: none), and the `[platform, encoding]` of the font's `cmap` subtable that maps those codes to glyphs |
| `problem` | the glyphs cannot be drawn from the display list (e.g. `type3` without its PK file on a document's first compile, before mktexpk made it, or a `.pgc` Type 3 font): the pages using the font are flagged INCOMPLETE (§4.7) |

**Drawing a glyph:** `name = encoding[code]`; draw the program's charstring
of that name. The PDF embeds a *subset* of this program (writet1) whose
charstrings, for every glyph the document uses, are the program's own: the
outlines, and so the pixels, are the same. `truetype`/`opentype`: the glyph
named `encoding[code]` in the font (its `post` table or CFF charset; pdfTeX
also resolves names `uniXXXX` through the font's Unicode `cmap` and
`indexN` as glyph index N); a subfont: the glyph the `cmap` subtable maps
`subfont[code]` to; a TrueType font without `encoding` (whole, `<<`): the
PDF's TrueType rules (pdfTeX writes no `/Encoding`). `type3`: §5.1.1.

#### 5.1.1 `type3` programs: glyph bitmaps

pdfTeX writes a font without a map entry (or with a bitmap entry: no
PostScript name, no font file) as a Type 3 font whose glyph procedures
each draw one 1-bit image mask from the font's PK file (writet3.c). The
program carries those masks, bit for bit:

```
u8[4]  "T3B1"
u32    n                  glyphs, in ascending code
n × {  u8   code          the character code (the glyph items' `code`)
       i32  llx, lly      the mask's lower-left corner, in glyph space
       u32  width, height the mask's size in pixels (0, 0: no ink)
       u8[height × ⌈width/8⌉] rows, top row first, most significant bit
                          first; a 1 bit is ink }
```

Glyph space is the bitmap's pixel grid, y up; the mask fills the rectangle
from (`llx`, `lly`) to (`llx+width`, `lly+height`) (the PDF's `width 0 0
height llx lly cm` and `/ImageMask true /Decode [1 0]`). `font_matrix` maps
glyph space to text space; the glyph matrix of the item (§4.4) maps text
space to the page, as for any font. The Rust crate decodes these programs
(`resource::Type3Bitmaps`). `key` covers the program, the encoding and
`font_matrix`.

**Held programs:** `COMPILE.have_fonts` lists keys the client already has;
for those the host sends the `FONT` frame with an empty program
(`program_bytes` still gives the real length). Measured: 2.5 MB of fonts
for a first compile of `hyperref-toc`, 116 kB per compile after.

`key` = SHA-256(`"display-list-v3 font\0"`, format, `0x00`,
SHA-256(program), each of the 256 names followed by `0x00` (when there is an
encoding), `i32 slant`, `i32 extend`, and, where present,
`"\0matrix\0"` + `font_matrix` (`type3`), `"\0subfont\0"` + the 256
codes as `i32` + `pid`, `eid` as `i16`, `"\0problem\0"` + `problem`).

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

The IMAGE item's matrix is exactly the PDF's `cm` before `/Im Do`. For a
PNG, JPEG or JBIG2 image (an Image XObject) it maps the unit square to stream
space. For a PDF image (a Form XObject, as pdfTeX writes it) it maps the
form's space to stream space, and the form's space is **the included page's
own coordinates**, in bp: the form's /BBox is the selected page box as the
file has it, [`orig_x`, `orig_y`, `orig_x`+`width`, `orig_y`+`height`], and
the `cm` already carries the scaled −origin (−s·`orig_x`, −s·`orig_y`). A
page with `/Rotate` 90, 180 or 270 also has pdfTeX's form `/Matrix` (the page
turned clockwise about its box, the turned box keeping its lower-left
corner). So: apply the matrix, then the form matrix if rotated, clip to the
box, and draw the page's content stream as it is. All four values are bp.
*Note (2026-10-03, lane INFDESC-APP):* `flashtex-host` currently sends
`width`, `height`, `orig_x` and `orig_y` of a PDF image in scaled points
(pdfTeX's `bp2int`), not bp; the Mac client reads a box no PDF page can have
(beyond 14,400 bp) from the file instead, and the host is to send bp. The
engine copies PNG and JPEG data
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

The protocol needs nothing from its transport but **a reliable, ordered
byte stream (Unix-domain socket on macOS/Linux; AF_UNIX or a named pipe on
Windows)**: frames (§2) carry their own lengths, and no message depends on
descriptor passing, datagram boundaries or credentials. The reference host
below listens on a Unix-domain stream socket; a Windows host would listen
on AF_UNIX (Windows 10 1803+, stream sockets only) or a named pipe.

`flashtex-host --socket PATH [--engine PATH] [--format NAME]... [--no-warm]
[--s0-cache DIR] [--profile MODE] [--budget BYTES] [--timed SECONDS] [--external-tools off|auto]
[--tool-timeout SECONDS]` first finds the TeX
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
`flashtex-host: {"saved_s0": PATH, "bytes": N, "ms": T}`. `--profile` is the
performance mode the host starts in (§6.9; default `balanced`, or
`FLASHTEX_PROFILE`). `--budget` and `--timed` are
the checkpoints' memory budget (default 1 GiB, the Balanced mode's) and timed interval (default
0.02 s); given, they hold in every mode. `--engine` names the engine program that `export` compiles and
the format preparation run (default: `flashtex-host` itself, which runs as
the engine when invoked as `pdftex`).

### 6.2 `HELLO`

The client speaks first:

```json
{"protocol": "display-list-v3", "version": [3, 2], "client": "FlashTeX 1.2"}
```

The host answers with its own `HELLO`, or with `ERROR` `{"code":
"version"}` and closes if the major differs:

```json
{"protocol": "display-list-v3", "version": [3, 2], "server": "flashtex-host 0.1.0",
 "engine": "pdfTeX 3.141592653-2.6-1.40.29 (FlashTeX engine)",
 "capabilities": ["compile", "cancel", "diagnostics", "font-programs", "font-formats", "have-fonts",
                  "resident", "incremental", "buffers", "edits", "viewport",
                  "pages-status", "export", "external-tools", "exact-geometry", "halt-on-error", "includeonly", "diag-v1"],
 "texmf": {"texlive": "/Library/TeX/texbin (PATH) -> /usr/local/texlive/2026/bin/universal-darwin",
           "resolver": "kpathsea (/Library/TeX/texbin)",
           "bundle": null,
           "formats": [{"name": "pdflatex", "status": "ready", "ms": 93.8}],
           "tools": {"bibtex": "/Library/TeX/texbin/bibtex", "biber": "/Library/TeX/texbin/biber",
                     "makeindex": "/Library/TeX/texbin/makeindex"},
           "external_tools": "off"}}
```

A client may add `"accept": [...]` to its `HELLO`: the optional message
families it wants, of those the host lists in `capabilities` (`diag-v1`,
§6.7; `progress-v1`, §6.8; 3.3's features, §11.7). The host ignores names
it does not know.

`texmf.texlive` is null when no TeX Live was found (the resolver is then
the bundle, if one is configured); a format whose `status` is `failed`
carries `error`, and compiles with it will fail: the app says so before
the user compiles.

`texmf.bundle` is the configured bundle (DESIGN.md §4.4), null when none
is: `FLASHTEX_BUNDLE_URL` and `FLASHTEX_BUNDLE_DIGEST`, else the first
`flashtex-bundle.lock` found (`FLASHTEX_BUNDLE_LOCK`; the per-user
configuration directory, `~/Library/Application Support/FlashTeX/` on
macOS; beside the host; the app bundle's `Contents/Resources/engine/`):

```json
"bundle": {"digest": "f7ed93…", "url": "https://…/core.ttb", "origin": "environment",
           "offline": false, "active": true}
```

`active` says the engine reads it (then `texmf.resolver` is `bundle
<digest>`): there is no TeX Live, or `FLASHTEX_RESOLVER=bundle`. `origin` is
`environment` or the lock file's path. A lock file's bundle is `offline`
(nothing fetched; only the cache is read) unless `FLASHTEX_BUNDLE_ALLOW_FETCH`
is `1` or `<digest>@<url>` naming that bundle and the URL it is fetched
from, which the app passes only after its user agreed to the download (a
bare digest, or another URL, is refused); `FLASHTEX_BUNDLE_OFFLINE=1` makes any bundle offline.
A lock file that does not parse (format: docs/contracts/bundle-lock-vectors.json)
gives `{"error": "…"}`. While a bundle's files are fetched (its index and
core on a cold cache, before the start-up line; a file or small package on
demand later) the host prints progress lines
`flashtex-host: {"bundle_progress": {"what": "index"|"core"|"file", "name":
…, "done": BYTES, "total": BYTES}}`; a step ends with `done` = `total`.

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
| `halt_on_error` | no | (capability `halt-on-error`; an older host ignores the field) `true`: `-halt-on-error`, TeX stops at the first error (a client's strict mode); default `false`: nonstopmode, recovering as pdflatex does. Another job: the resident document is replaced |
| `includeonly` | no | (capability `includeonly`; an older host ignores the field) `["chapters/03", …]`: LaTeX's own `\includeonly`, a chapter focus. The first line becomes `\AtBeginDocument{\includeonly{chapters/03,…}}\input MAIN` (MAIN quoted when it holds a space): `\includeonly` runs in the `begindocument` hook, after the `.aux` is read and before any `\include`, which gives what it gives in the preamble (the same pages, content streams and `.aux` under pdflatex) while its file-name lookup (the chapter's `\pdffilesize`) stays out of S₀'s key, so an edit in the focused chapter restarts at S₀, not from the format; `\input` without a brace is the primitive, so MAIN is read as a bare first line reads it. Only the named `\include`s are typeset; the others' page numbers and references come from their `.aux` files in `output_dir`. Each name is as the document's `\include` writes it: relative, inside `root`, without `,` `{` `}` `\` `%` `#` `"` or a control character; an empty array is refused. Another job: the resident document is replaced, with its own stored S₀. Clients give a focused job its own `output_dir`, started from the whole document's auxiliary files, so the whole document's `.aux` is not rewritten by focused runs |
| `output_dir` | no | where the PDF, log and auxiliary files go (default: a per-connection temporary directory). A client creates the folders `\include`d files sit in (`chapters/` for `\include{chapters/03}`) here, as latexmk does: pdfTeX cannot create them |
| `jobname` | no | default: the main file's name |
| `have_fonts` | no | font keys (hex) the client holds (§5.1) |
| `font_formats` | no | host capability `font-formats`: the font formats beyond `type1` and `none` whose programs the client takes: any of `truetype`, `opentype`, `type3` (§5.1); default none |
| `incremental` | no | 3.1: `true` keeps the pages and resource ids of this connection's earlier compiles of the document: the host sends only pages that changed, and `PAGES` (default `false`: every page, every compile, as in 3.0) |
| `viewport` | no | 3.1: the page (0-based) the client shows; the run stops there first, says so (`PAGES`), then typesets the rest |
| `buffers` | no | 3.1: `[{"path", "text"}]`: files as the editor has them (path relative to `root`, `/`-separated, §1); the host writes each to its file, as saving would, before compiling |
| `edits` | no | 3.1: `[{"path", "offset", "delete", "insert"}]`: byte splices of files under `root` (path relative to `root`, `/`-separated, §1), applied in order, before compiling |
| `export` | no | `true`: a one-shot run of the engine as a child process instead of the resident engine: `DONE.pdf` is the compressed PDF pdflatex would write (P-T2), not the preview's |
| `external_tools` | no | 3.2: `auto`: after the compile, run bibtex, biber and makeindex from the user's TeX Live when latexmk would, then compile again (§6.4, "External tools"); `off`: never. Default: the host's `--external-tools` (`off` unless the host was started with `auto`). The app sends `auto` only for a **trusted** project (DESIGN.md §4.5): an untrusted project runs no external program |

The engine runs as pdflatex would:
`pdftex -fmt=FORMAT -interaction=nonstopmode -file-line-error -output-directory=DIR -jobname=JOB [shell flag] MAIN`
(with `includeonly`, `'\AtBeginDocument{\includeonly{NAMES}}\input MAIN'` in place of `MAIN`)
(without `-output-directory` when `output_dir` is `root` itself, as a
plain `pdflatex MAIN` or latexmk runs it),
in the resident engine: the first compile of a document is a full run; a
later one restarts from the last checkpoint before what changed (the
edits, or any file the run read) and stops once the engine state equals
the previous run's (DESIGN.md §5.3), so a keystroke re-typesets a page or
two. The PDF (`DONE.pdf`) is the preview's: its streams are stored, not
compressed; `export` makes the compressed one.
One run: the client decides when to rerun (e.g. after `\label` changes:
`DONE.mode` `incremental` or `cold` after an `.aux` change, `unchanged`
when nothing changed).
With `"external_tools": "auto"` the client needs no second `COMPILE` for
a bibliography or an index: see "External tools" in §6.4.
**A `COMPILE` while one is running supersedes it**: the running compile
goes on (a page is never interrupted) without sending, its `DONE` says
`cancelled`, and the next compile sends what is current; a compile
superseded before it started only applies its edits. One exception keeps
fast typing visible: an incremental compile is not stopped before its first
changed page has shipped (or three pages have, none changed), and it sends
that page (with the forms it draws) even when superseded, so every
keystroke's edit reaches the screen however fast the next one comes (lane
LIVE-30MS). That `PAGE` comes after the newer `COMPILE` was sent and before
the superseded compile's `DONE`; a client takes it like any other page. An `export` compile
is killed as in 3.0.

### 6.4 Replies

For one compile the host sends, in this order: `STARTED`; then, interleaved
as the engine produces them, `FONT`, `IMAGE`, `SOURCES`, `PAGE`, `FORM`,
`PAGES` and `DIAGNOSTIC`; then exactly one `DONE`. Pages arrive while the
engine is still typesetting later ones, **in page order**: the first page
the compile re-typesets (the edited one) first; pages it did not
re-typeset (before the restart point, or after convergence) are sent from
the host's cache in their place, unless the client holds them already
(`incremental`). When the run's `.aux` (or a file it reads again) changed, the
compile runs further passes (DESIGN.md §5.5) before `DONE`; each pass sends
the pages it typesets again, in page order, from where it restarts, which may
be before pages already sent: **a page that arrives again replaces the earlier
one** (the pages of the last pass are the document's).

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
at or past `count`). Also sent, without a page of its own, when the compile
finds nothing new against a run newer work stopped and continues that run
as its own (fast typing: a letter typed, deleted and typed again): the
pages the stopped run had shipped are this compile's at once, so the host
sends those the client lacks from its cache and then `PAGES` with them
current, before the run ships its next page.

`DIAGNOSTIC`: `{"id", "severity": "error"|"warning", "message", "file"?, "line"?}`
(3.3 adds `column` and `hints`, §11.7)
— TeX errors (`file:line: message` or `! message`) and LaTeX/package
warnings from the engine's terminal output. A client that accepted
`diag-v1` gets `DIAG`s (§6.7) instead: the column, the byte range, the
macro trace, the help and a stable code.

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

The resident engine's `DONE` may also carry `stages`, an object of
**optional diagnostics** for benchmarks (`tools/incr-bench`): a client must
not depend on any of its keys, which may change without a protocol version.
Times are in ms (`queue`, `apply`, `move_spans`, `find`, `key`, `changes`, `restore`,
`first_page`, `first_page_cpu`, `first_page_dl`, `first_page_send`,
`edited_wall`, `edited_cpu`, `test`, `dl`, `send`, `cpu`; `tests` and
`edited_page` are counts and a page index). `queue_by` splits `queue` (the
request's wait for the engine thread) by what the engine thread did
meanwhile, in ms per part (`restore`, `typeset`, `test`, `jump`, `paused`,
`done`, `prepare`, `request`, `idle`, `other`). When the compile found a
run that newer work had stopped (typing: the previous compile's background
work), `paused_how` says what it did with it first (`continued`,
`settled`, `abandoned`) and `paused` (ms) and `paused_instr_k` what that
took, inside `key` and `find`. `old_kept` and
`old_rewound` count, since the host started, the old checkpoints' chunks
the convergence comparisons took from their cache and rewound. On macOS and
Linux the engine thread's counter readings are added, in thousands
(`os::thread_counts`: macOS's fixed counters; Linux's `perf_event_open`,
user space only; absent where the system does not give them): `instr_k` and
`cycles_k` for the whole compile, `first_page_instr_k` to the first page,
`restore_instr_k` for the restore, `edited_instr_k` from just before the
restore to the edited page's shipout, `typeset_instr_k` and
`typeset_cycles_k` from the engine's resumption after the restore to the
edited page's shipout (the typesetting alone), `test_instr_k` for the
convergence tests, and two absolute cycle marks of the engine thread,
`arrival_mark_kc` (when the `COMPILE` arrived; also on a `DONE` cancelled
before its compile started) and `first_page_mark_kc` (its first page): a
keystroke's latency in engine cycles is the second mark of the compile that
painted it less the first of its own (`dl3-keys --interval-ms`,
`docs/evidence/live-30ms-2026-10-04/scripts/interval.py`).

**External tools (3.2).** For a `COMPILE` with `"external_tools":
"auto"`, once its `DONE` is out (never before: the edited page is not
delayed), the host decides as latexmk 4.87 does
(`rdb_set_latex_deps`, `parse_aux`, `parse_bcf`) which programs the
document needs, and runs them from the user's TeX Live (`HELLO.texmf.tools`)
on a worker thread, one at a time, each with a timeout (`--tool-timeout`,
default 120 s):

| program | when | sources compared with its last run |
|---|---|---|
| `biber JOB.bcf` | the run wrote `JOB.bcf` (biblatex) | the `.bcf`, the data sources it names |
| `bibtex BASE` | otherwise, for each `BASE.bbl` the run read or looked for whose `BASE.aux` the run wrote and names `\bibdata` | the `.aux` lines bibtex reads (`\citation`, `\bibdata`, `\bibstyle`, `\@input` and the `.aux` files it inputs), the `.bib` files, the `.bst` |
| `makeindex -o X.ind X.idx` | for each `X.idx` the run wrote | the `.idx` |

A program runs when its sources differ from its last run's, or when its
output is missing or not what that run made; bibtex and biber do not run
while a `.bib` file they need is missing (latexmk's default: the document
keeps the `.bbl` it has). bibtex and makeindex run in the directory of the
`.aux`/`.idx` with `BIBINPUTS` and `BSTINPUTS` starting with the project
and output directories; biber with `--input-directory` the project. Their
outputs (`.bbl` and `.blg`, `.ind` and `.ilg`) are written into the output
directory atomically, and only when they changed. When a `.bbl` or `.ind`
changed, the host compiles again **by itself**: a follow-up compile with
the same `id`, reported like any compile (`STARTED`, pages, `PAGES`,
`DIAGNOSTIC`s, `DONE`) with `"cause": "tools"`; the resident engine
restarts before the first read of the changed file, and its `.aux` passes
follow (DESIGN.md §5.3, §5.5). Then the host asks again, up to 5 rounds.
A newer `COMPILE` from the client supersedes the cycle (it reads what the
tools made, and asks again when it is done).

`TOOL` (3.2): `{"id", "event", ...}` about the compile `id`'s tools:

- `"event": "run"`: `{"tool", "file", "reason"}` — a program starts
  (`file`: its source, relative to the output directory).
- `"event": "done"`: `{"tool", "file", "status", "exit_code", "ms",
  "changed", "warnings", "errors", "log", "message"?}` — `status`: `ok`,
  `warnings`, `errors`, `error` (exit status, nothing in the log), `timeout`
  (killed; its outputs are not used) or `failed` (could not start);
  `changed`: its output differed; `log`: the `.blg`/`.ilg`. Its warnings and
  errors also arrive as `DIAGNOSTIC`s with `"source": "bibtex"` (`biber`,
  `makeindex`), with `file` and `line` when the log names them (a `.bib`
  syntax error); a client that accepted `diag-v1` gets them as `DIAG`s
  instead (§6.7).
- `"event": "skip"`: `{"tool", "file", "reason"}` — a program that would run
  does not: the compile has `external_tools` `off` ("…external tools are
  off for this project": the app can offer to trust it), a `.bib` file is
  missing, or TeX Live has no such program. Said once per state of its
  sources.
- `"event": "settled"`: `{"ran", "rounds", "limit"?}` — the client's
  compile and its follow-ups are done as far as tools go (sent once per
  cycle, also when no tool was needed): `ran`, whether any program ran;
  `rounds`, the follow-up compiles; `limit`, stopped after 5 rounds.

### 6.5 `CANCEL`, `BYE`

`CANCEL {"id"}` stops that compile: an `export` process is killed; the
resident engine finishes the page it is on and the rest of the run without
sending (its checkpoints stay valid for the next compile). Its `DONE` says
`cancelled`. A `CANCEL` for an id that is not running is ignored. `BYE`
(or closing the socket) ends the connection and cancels a running compile.

### 6.6 Without the host

The engine writes the same frames when run directly:
`FLASHTEX_DISPLAY_LIST=file.dl3 pdftex -fmt=pdflatex main.tex`, and
`FLASHTEX_DISPLAY_LIST_HAVE_FONTS=key,key` for held fonts. `dl3-dump
file.dl3` prints such a file as JSON lines. `FLASHTEX_DISPLAY_LIST` takes:

| value | the engine writes to |
|---|---|
| `fd:N` | inherited descriptor `N` (Unix; how the host runs an `export`) |
| `socket:PATH` | a Unix-domain stream socket listening at `PATH`, which the engine connects to (macOS/Linux) |
| `pipe:NAME` | the named pipe `\\.\pipe\NAME` (Windows; elsewhere the engine reports it unsupported and writes nothing) |
| anything else | a file at that path, created or truncated (write `./fd:x` for a file whose name starts with a prefix above) |

The named forms exist because Windows has neither `socketpair` nor
numbered-descriptor inheritance: a launcher there listens on a name and
passes the name. One parser, `flashtex_display_list::endpoint`, defines
this grammar for the engine and for launchers.

### 6.7 `DIAG`: structured diagnostics (`diag-v1`)

A separate, capability-gated message family (lane P5-DIAGNOSTICS),
independent of the 3.x page protocol's minor versions: it never changes a
page item, a section or a 3.1 message, and its kinds have a range of their
own (`0x60`..`0x6F`; `0x60` is `DIAG`), so a later minor version's page
messages (DESIGN.md §15's Typst proposals) and this family cannot meet.

**Negotiation.** The host lists `"diag-v1"` in `HELLO.capabilities`. A
client that wants it says so in its `HELLO`:

```json
{"protocol": "display-list-v3", "version": [3, 1], "client": "FlashTeX 1.3",
 "accept": ["diag-v1"]}
```

Such a client gets, for every compile, one `DIAG` per error or warning
**instead of** the `DIAGNOSTIC`s of §6.4 (never both), in the order the
engine reported them, after the compile's last `PAGE`/`PAGES` and before
`DONE`; `DONE.diagnostics` counts them. A client that does not accept
`diag-v1` (every 3.0 and 3.1 client) sees exactly what it saw before.

**What a `DIAG` knows.** The engine records, at the moment TeX reports
something, what TeX itself knows then (`changes/diagnostics.ch`,
`src/diag.rs`): TeX's `error`, `pdf_warning`, the overfull/underfull box
reports of `hpack`/`vpackage`, `\write`s to the terminal (LaTeX's, packages'
and classes' warnings are `\immediate\write`s), `\def` (definition
sites), and a source line too long for TeX's buffer (texmfmp.c's
`input_line`, which prints to stderr, not the terminal: an `error` at the
file and line being read, with no `col`). The hooks only read TeX's
variables and never print: the terminal and the log are byte-identical with and without them (P-T1). The record
travels with the engine's checkpoints, so an incremental compile reports
every diagnostic of the document — those of pages it kept too — exactly as
a run from scratch does, and a persisted S₀ carries the preamble's.

```json
{"id": 12, "seq": 1, "severity": "error",
 "code": "tex/undefined-control-sequence", "origin": "tex",
 "message": "Undefined control sequence.",
 "file": "/Users/me/paper/main.tex", "line": 6, "col": 19, "range": [10, 19],
 "offset": 133, "span": 593,
 "trace": [
   {"kind": "argument", "text": ["x \\undefinedthing ", ""]},
   {"kind": "macro", "name": "\\textbf",
    "text": ["#1->\\ifmmode \\nfss@text {\\bfseries #1}...", "\\check@icr ..."]},
   {"kind": "macro", "name": "\\mycmd", "text": ["#1->\\textbf {#1 \\undefinedthing }", ""],
    "def": {"file": "/Users/me/paper/main.tex", "line": 2}},
   {"kind": "file", "file": "/Users/me/paper/main.tex", "line": 6, "col": 19,
    "text": ["Some text \\mycmd{x}", " more."]}],
 "help": ["The control sequence at the end of the top line",
          "of your error message was never \\def'ed. ..."],
 "exact": true}
```

| key | always | meaning |
|---|---|---|
| `id`, `seq` | yes | the compile's id; 0-based order within the compile |
| `severity` | yes | `error`, `warning`, `info` (a `\show`; a tight or loose box) |
| `code` | yes | stable, below |
| `origin` | yes | `tex`, `latex`, `latex3`, `package`, `class`, `pdftex`; `bibtex`, `biber`, `makeindex` (external tools, below) |
| `package` | no | the package or class that reported it (`origin` `package`/`class`) |
| `message` | yes | the report's first line as TeX printed it (`Undefined control sequence.`, `LaTeX Error: File `x.sty' not found.`, `LaTeX Warning: Reference `a' on page 1 undefined on input line 8.`, `Overfull \hbox (3.2pt too wide) in paragraph at lines 5--7`) |
| `detail` | no | the rest of the message (LaTeX's "See the LaTeX manual…"; a box report's second line) |
| `file` | no | absolute path of the file TeX was reading (the innermost *file* level of the input stack: the level TeX's own context display ends with) |
| `line` | no | 1-based line: TeX's `l.<n>` |
| `col` | no | 0-based **byte** column in `line` where TeX's context display splits the line (`l.6 Some text \mycmd{x}` / `more.`): what TeX had read |
| `range` | no | byte columns `[from, to)` in `line`, `to` = `col`: the command before the split with its arguments (`\mycmd{x}`, `\ref{a}`), else the token before it (`\foo`, `^`, a UTF-8 character) |
| `offset` | no | byte offset of `col` in the file as the host read it after the compile |
| `span` | no | the display-list span (§5.3) of (`file`, `line`), declared in a `SOURCES` before the `DIAG` if the client lacks it; it moves with its line across edits like the pages' spans |
| `end` | no | `{"file","line","col","span"}`: where the material ends (box reports: the box's last character) |
| `lines` | no | TeX's line range (box reports: "at lines a--b"; "detected at line n" is `[n, n]`) |
| `trace` | no | the input stack when TeX reported it, **innermost level first**, every level up to 24 (the innermost 23 and the file level; TeX shows only `\errorcontextlines` of them, and LaTeX sets that to −1): `kind` (`macro`, `argument`, `template`, `backed_up`, `recently_read`, `inserted`, `output`, `everypar`, `everymath`, `everydisplay`, `everyhbox`, `everyvbox`, `everyjob`, `everycr`, `mark`, `everyeof`, `write`, `file`, `scantokens`, `terminal`, `insert`, `read`; a client shows an unknown kind as its name), `name` (a macro's), `text` (`[read, still to read]`, as TeX shows the level; each side at most 240 bytes), a file level's `file`/`line`/`col`, a macro's `def` (`file`, `line`: where this run defined it, when it saw the definition; macros of the format have none). After the compile's first 1,000 reports, a report's `trace` keeps only its file level |
| `help` | no | TeX's help lines (the log has them; the terminal does not), or the `\errhelp` text of `\errmessage` (LaTeX's `\PackageError` help) |
| `fatal` | no | `true`: TeX stopped (emergency stop, capacity exceeded, `==> Fatal error occurred`) |
| `output` | no | `true`: reported while `\output` was active (a box report then has no line range) |
| `exact` | yes | `true`: from the engine's record; `false`: read from the terminal text only (§6.4's rules: `file`/`line` at best) — what pdfTeX's C parts print, and every `DIAG` of an `export` compile (another process) |

**External tools (3.2).** A tool's warnings and errors (§6.4, `TOOL`) reach
a `diag-v1` client as `DIAG`s too, never as `DIAGNOSTIC`s. They come after
the compile's `DONE`, before that program's `TOOL` `done`, with the
compile's `id`; `seq` counts within that program's run; `origin` is the
program, `code` is `<program>/<slug>` (the slug as in **Codes** below),
`message` is the line as the program wrote it, `file`/`line` are set when its
log names them, and `exact` is `false`. `DONE.diagnostics` does not count
them (the `TOOL` `done` has `warnings` and `errors`).

**Where a box report points.** TeX says only "in paragraph at lines a--b".
The display list's side table knows where each character came from (§5.3),
so a box report's `file`/`line`/`col`/`span` are its **first character's**
and `end` its last's; `lines` keeps TeX's range. Without a display list the
place is line `a` of the file TeX was reading.

**Codes.** `origin/slug` or `origin/package/slug`: `tex/…`
(`undefined-control-sequence`, `missing-dollar`, `missing-left-brace`,
`missing-right-brace`, `extra-right-brace-or-forgotten-dollar`,
`extra-right-brace-or-forgotten-endgroup`, `too-many-right-braces`,
`display-math-should-end-with-dollars`, `missing-number`, `illegal-unit`,
`paragraph-ended-before-argument-complete`, `file-ended-while-scanning`,
`emergency-stop`, `capacity-exceeded`, `file-not-found`,
`cannot-use-in-this-mode`, `misplaced-alignment-tab`, `extra-alignment-tab`,
`double-superscript`, `double-subscript`, `missing-right-delimiter`,
`extra-right-delimiter`, `missing-character`, `fatal-error-no-output`, `show`, `overfull-hbox`,
`underfull-hbox`, `tight-hbox`, `loose-hbox`, the same for `vbox`),
`latex/…` (`file-not-found`, `environment-undefined`,
`environment-mismatch`, `missing-begin-document`, `missing-item`,
`lonely-item`, `verb-ended-by-end-of-line`, `option-clash`,
`unknown-option`, `command-already-defined`, `undefined-reference`,
`undefined-citation`, `multiply-defined-label`, `rerun`, `float-too-large`,
`unicode-not-set-up`, `caption-outside-float`, `no-file`),
`latex-font/font-shape-undefined`, `package/<name>/…`, `class/<name>/…`,
`pdftex/<category>` (`pdftex/dest`). Every other message's slug is its text
with quoted names (`` `x' ``), control sequence names, arguments in braces,
numbers and "on input line N" left out, lower case, words joined by `-`
(`Undefined color `x'.` → `undefined-color`), at most 60 characters. A code
names the kind of problem, never its instance.

*Added 2026-10-05 (lane DIAG-PARITY, #1592).*
- **Two codes are new reports** that a `diag-v1` client did not get before. Both are
  read from the terminal (`exact: false`), and neither has a place:
  - `latex/no-file`: "No file X.tex.", LaTeX's `\typeout` for an `\include` or
    `\InputIfFileExists` of a missing `.tex` file. The `.aux` and `.toc` files of a
    first run are not reported.
  - `tex/missing-character`: "Missing character: There is no X in font Y!", shown
    on the terminal when `\tracinglostchars` > 1.
- **Five codes replace slugs** the rule above used to produce:

  | Before | Now |
  |---|---|
  | `tex/missing-inserted` | `tex/missing-right-delimiter` |
  | `tex/extra` | `tex/extra-right-delimiter` |
  | `latex/unicode-character-u` | `latex/unicode-not-set-up` |
  | `latex/outside-float` | `latex/caption-outside-float` |
  | `latex/float-too-large-for-page-by-pt` | `latex/float-too-large` |

- **Why the renames need no alias or gate:**
  - Two of the old codes broke the rule that a code names a kind. `tex/extra` was
    also the slug of "Extra \else", "Extra \fi" and "Extra \or". `tex/missing-inserted`
    was also the slug of "Missing \endcsname inserted".
  - `diag-v1` is capability-gated, and codes are not listed in HELLO.
  - The only consumers of codes are:
    - the app (`EngineV3Explain`, `EngineV3Fixes`, `EngineV3ErrorPolicy`,
      `EngineV3DiagPresent`);
    - the engine's `tests/diagnostics.rs`;
    - `tools/diag-oracle`.
  - On #1592's tree none of them names an old code: a search for them in `apps/`, `crates/` and
    `tools/` finds nothing.
  - A client that keyed on an old slug falls back, as for any unknown code, to the
    message.

**Precision** (measured, lane P5-DIAGNOSTICS: `crates/flashtex-engine/tests/diagnostics/`,
docs/evidence/p5-diagnostics-2026-09-30/): on an 80-document corpus of
common errors, `line` and `col` equal the position pdflatex's own error
context shows (`l.<n>` and the split) for every report that has one; for
LaTeX and package warnings, which pdflatex prints without context, `col` is
checked against the split of an oracle run that turns `\GenericWarning` into
an error.

**Decoding.** `crates/display-list-v3/src/diag.rs` (`flashtex_display_list::diag::Diag`,
MIT) is the reference decoder; `client::Event::Diag` carries it. In Swift:

```swift
struct DiagLoc: Decodable { var file: String?; var line: Int?; var col: Int?; var span: Int? }
struct DiagFrame: Decodable {
    var kind: String; var name: String?
    var file: String?; var line: Int?; var col: Int?
    var text: [String]?; var def: DiagLoc?
}
struct Diag: Decodable {
    var id: Int; var seq: Int; var severity: String; var code: String; var origin: String
    var package: String?; var message: String; var detail: String?
    var file: String?; var line: Int?; var col: Int?; var range: [Int]?; var offset: Int?
    var span: Int?; var end: DiagLoc?; var lines: [Int]?
    var trace: [DiagFrame]?; var help: [String]?
    var fatal: Bool?; var output: Bool?; var exact: Bool
}
// kind 0x60: let d = try JSONDecoder().decode(Diag.self, from: body)
```

A Problems panel row: `severity` icon, `message`, `file:line:col+1`; the
underline is `range` on `line` (byte columns: convert to the editor's
string index), or `line` alone; the macro rows of `trace`
(`kind == "macro"`: "in `\name`", with `def` as a link) and `help` as
expandable sub-rows; `span` keeps the row on its line across edits
(`SOURCES` re-declarations).

### 6.8 `PROGRESS`: a typesetting heartbeat (`progress-v1`)

An additive, capability-gated message, like `diag-v1` (§6.7): it changes no
page item, section or earlier message, and the protocol version stays as it
is. Its kinds have a range of their own (`0x70`..`0x7F`; `0x70` is
`PROGRESS`). A client that does not accept it never receives it, so an older
app sees no difference.

**Why.** A compile can be silent for a long time while it works: a later
`.aux` pass re-typesets every page but sends only the pages that changed
(§6.4), and on a long document such a pass is silent for tens of seconds.
A client that bounds a compile that never finishes (the app's stall bound:
an endless macro loop before the first page holds the engine thread, and a
newer compile only preempts it at a page or segment checkpoint) needs to
tell such a pass from a loop.

**Negotiation.** The host lists `"progress-v1"` in `HELLO.capabilities`; a
client adds `"progress-v1"` to its `HELLO.accept`.

**Message** (`0x70`, JSON): `{"id", "pass", "page", "file"?}`. `id` is the compile;
`pass` is the run's pass (1, then 2, ... for further `.aux` passes); `page`
is the number of pages that run has shipped; `file` (added 2026-10-06, for
`flashtex-v3`'s progress line; absent at the terminal level) is the
innermost file TeX is reading, as TeX opened it (`./chapters/a.tex`, a
TeX Live path), the name `-file-line-error` gives. A client that does not
know a key ignores it. The host sends one at the
first page or segment checkpoint of each pass and then at most every 250 ms
while the pass reaches checkpoints, in every run (cold or incremental, the
first pass and the `.aux` passes behind it), whether or not those pages are
sent. Nothing is sent between checkpoints: an endless loop that reaches
none sends none, which is what a client's bound detects. A client treats
any `PROGRESS` as the compile making progress and otherwise ignores it.

### 6.9 `PROFILE`: performance modes (`profile-v1`)

An additive, capability-gated feature like `progress-v1` (§6.8): no page,
section or earlier message changes, and the protocol version stays as it
is. DESIGN.md §1.2 ("Performance modes"), lane PERF-MODES.

**What a mode is.** A named set of the host's knobs: how much memory the
checkpoints may hold, how densely they are kept near the cursor, how long
the engine stays warm after a compile, whether it works out the next
restore while it waits, and how soon it gives freed memory back. **A mode
never changes what a compile produces**: every page, the PDF and the log
are identical in every mode; only latency and memory differ.

| mode | for |
|---|---|
| `low-memory` | the least memory: a small checkpoint budget, few checkpoints far from the cursor, memory given back soon after typing stops; edits far from the last one take longer |
| `balanced` | the default: the latency and memory targets of DESIGN.md §1.2 and §5.2 |
| `high-performance` | the lowest latency: a large checkpoint budget (a quarter of the machine's memory, 1–4 GiB), checkpoints dense near the cursor, a longer keep-warm window, nothing trimmed |

**Negotiation.** The host lists `"profile-v1"` in `HELLO.capabilities`. A
client may add `"profile": MODE` to its `HELLO`; a mode the host does not
know is ignored. Either way the host's `HELLO` carries `profile`, the knobs
in effect for this connection's compiles:

```json
"profile": {"mode": "balanced", "budget": 1073741824, "dense": 16,
            "segment_ms": 0.5, "timed_ms": 20.0, "keep_warm_ms": 2000,
            "prepare": true, "trim_after_ms": 2000}
```

`budget` is bytes; `segment_ms` and `trim_after_ms` are null when off
(High Performance never trims). The keys are informative: a client shows or
logs them and does not depend on any one. A knob the host's command line
fixed (`--budget`, `--timed`, `--keep-warm`) keeps its value in every mode.

**Message** (`0x07`, client → host, JSON): `{"profile": MODE}` switches the
mode live. The host applies it between compiles, in the order requests
arrive (a running compile finishes under the old mode), and answers with
`PROFILE` (`0x51`, host → client, JSON) `{"profile": {...}}`, the knobs now
in effect, as in `HELLO`. A smaller budget or dense window thins the
resident document's checkpoints at once and gives the freed memory back
before the reply. An unknown mode gets `ERROR` (`request`) and changes
nothing. The mode is the host's: with one host per document, it is that
document's.

## 7. Errors

- **Decoding fails closed.** A frame of bad length, a body shorter than its
  layout, an unknown item opcode, an item that names a path, matrix,
  unsupported entry or colour space that does not exist, a colour of other
  than 1, 3 or 4 components (or, 3.3, of other than its colour space's
  count): the reader rejects the message and treats the connection as
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
2. Say `[3, 2]` and compile with `"incremental": true` (and, for a
   trusted project, `"external_tools": "auto"`; show `TOOL` `run`/`done` as
   a status line and treat `settled` as "bibliography and index current"): on each keystroke
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
9. Fonts beyond Type 1 (lane P3-FONTS-2): send `"font_formats": ["type3",
   "truetype", "opentype"]` in `COMPILE` once each is drawn. `type3`:
   decode §5.1.1 (`resource::Type3Bitmaps` in Rust) and draw each glyph's
   mask as an image mask in the current fill colour with the matrix
   [`width` 0 0 `height` `llx` `lly`] × `font_matrix` × the glyph matrix;
   `truetype`/`opentype`: Core Text loads both from the program's bytes
   (`CTFontManagerCreateFontDescriptorFromData`); draw the glyph named
   `encoding[code]` (`CGFontGetGlyphWithGlyphName`), or for a subfont the
   glyph `cmap` gives `subfont[code]`. Pages whose fonts carry `problem`
   are INCOMPLETE: draw them from `DONE.pdf`.
10. Accept `diag-v1` (§6.7) and feed the Problems panel from `DIAG`s:
    `file`/`line`/`col`/`range` for the underline, `trace` and `help` as
    sub-rows, `span` to keep a row on its line across edits.

## 10. Limits of version 3.3

- Extended graphics state (`gs`: transparency), shadings, patterns,
  separation colour spaces, inline images and text clipping are flagged
  INCOMPLETE, not expressed (2 of the 82 parity fixtures use `gs`).
- TrueType, OpenType and Type 3 (PK) fonts reach a display list (§5.1,
  §5.1.1) for a client that asks for them; a Type 3 font from a `.pgc`
  file, and a PK font on the compile that first makes its PK file, are
  flagged INCOMPLETE (`problem`).
- DVI mode (`\pdfoutput=0`) and `\pdfdraftmode` pages carry no geometry.
- A `\pdfpageattr` that overrides `/MediaBox` is not reflected in `box`.
- One resident engine per host process, for one document at a time (the
  app runs a host per open document); an `export` runs its own process.
- The resident engine is not interrupted inside a page: a newer `COMPILE`
  waits for the running one's page (and, today, for the rest of its run,
  which the next compile then starts from).
- External tools (3.2): bibtex, biber and makeindex only (latexmk's
  defaults); makeglossaries, xindy, splitindex and custom latexmk rules are
  not run. What a tool made is remembered per host process: a new host runs
  each needed tool once more (its unchanged output is not re-installed, so
  no recompile follows).
- S₀ persisted with `--s0-cache` does not carry source spans: after a
  reopen, material made before `\begin{document}` (none that a page shows,
  in practice) has no span.
- Typst (3.3, §11): the Typst host produces E1–E4 and E7 (E3, E4 for a
  client that accepts them); images and islands (E5, E6) are flagged
  INCOMPLETE and `RESOLVE`/`LOCATE` (E8) is not answered yet.

## 11. Version 3.3: the Typst host's additions

3.3 carries what a Typst document draws and v3.2 cannot express (DESIGN.md
§15.1, §15.4: E1–E8). Its producer is `flashtex-typst-host` (`typst-host/`,
MIT, its own workspace), which speaks this protocol (§6) for one open
`.typ` document per process. The LaTeX host is never required to send any
of it. Everything here is additive: JSON keys, section tags and message
kinds a 3.2 reader skips (§3), and item opcodes sent only to a client that
asked for them (§11.7).

What the Typst host sends today is marked **produced**; the rest is
specified, decoded by the reference crate, and not yet produced. Produced
or not, a class of items needs its 2×/3× pixel gate row in DESIGN.md §15.5
(the app's renderer against typst-pdf's PDF) before a page using it is
drawn rather than flagged INCOMPLETE: until then the Typst host sends the
items **and** flags the page INCOMPLETE, with an UNSUPPORTED entry
"…: pixel gate row pending (DESIGN.md §15.5)", so the client shows
`DONE.pdf`. The host's `--draw-ungated` drops that flag, for measuring the
rows only. Pending today: ICCBased and Separation colours
(`FILL_COLOR_CS`, `STROKE_COLOR_CS`), alpha other than 1, stroked glyphs.

For a Typst document **the PDF** of §4.2 and §4.4 is typst-pdf's export of
the same compile (`DONE.pdf`; the host's per-page positions come from a
one-page export of the page, whose content stream draws the same).
Positions, colours and paths are the numbers that PDF draws with, read as
§4.2 says.

### 11.1 `FONT.format` `opentype` with glyph ids (E1; produced)

A Typst font resource is one **font instance**: a font file, a face in it,
and variation coordinates. Its `FONT` has `format` `opentype` (whatever the
outlines: `outlines` says which) and these JSON keys instead of the TeX
ones:

| key | meaning |
|---|---|
| `glyph_ids` | `true`: a GLYPH's `code` is the **glyph id** in the face (no `encoding`) |
| `face_index` | the face in a collection (`.ttc`/`.otc`), else 0 |
| `units_per_em` | the face's units per em |
| `font_matrix` | `"1/upm 0 0 1/upm 0 0"`: glyph space to text space |
| `outlines` | `cff` (CFF or CFF2) or `truetype` (`glyf`) |
| `variations` | `[[tag, value], ...]`: the variation coordinates (`wght` 700, …), in the font's axis units; `[]` for the default instance |
| `family`, `ps_name` | the family and PostScript name, for display and font-list checks |
| `file` | the font file the host read (absolute path) |
| `program_sha256`, `program_bytes` | of the whole file |
| `program_from` | `accept` `font-program-refs`: the id of an earlier `FONT` on this connection that carried this program (another instance of the same file); the program is then empty |

The program is the **whole font file** (a collection included): Core Text
and FreeType load it from the bytes and select `face_index`, then apply
`variations`. The PDF embeds a CFF or TrueType subset of the same outlines
(an instanced one for a variable font) whose hints and outlines are the
file's (DESIGN.md §15.5), so the pixels are the same.

The program is sent once per connection: in the first `FONT` that takes it,
and, to a client that accepts `font-program-refs`, never again
(`program_from`); to any other client it is sent whole in every `FONT`,
bounded per compile by the host (`--font-program-budget`; past it the
compile fails with a diagnostic rather than silently dropping a font). A
client that accepts `font-files` may receive an empty program with `file`
and `program_sha256` for any font: it reads the file itself and must refuse
it (and reconnect without `font-files`) when the file's SHA-256 differs.
This keeps 20 MB CJK collections off the socket.

`key` = SHA-256(`"display-list-v3 font\0"`, `"opentype"`, `0x00`,
SHA-256(file), `"\0face\0"`, `u32 face_index`, then for each variation
coordinate in order `"\0var\0"`, the tag's four bytes, `f32 value`)
(`resource::opentype_font_key`). It depends on neither the id, the path
nor the font's names.

A 3.1 or 3.2 client, or one that does not list `opentype` in
`COMPILE.font_formats`, gets every Typst page with glyphs INCOMPLETE (the
fonts' programs empty) and draws `DONE.pdf`.

### 11.2 `ORIGINS` and `PAGE_META` for Typst pages (E2, E7; produced)

**`ORIGINS`** (section 7, §4.2) carries every GLYPH's origin as the
reference viewer computes it from the PDF's content stream (typst-pdf's,
§11): one `f64[2]` per GLYPH, in item order. A Typst page always has it.
Typst's own frame positions are not enough: typst-pdf (krilla) writes
positions in f32, so the frame positions miss the PDF's by up to 6 × 10⁻⁵
bp, which moves 31–5,324 pixels at 2× and 3× (DESIGN.md §15.5). The GLYPH's
own x and y are those origins rounded to sp (§4.2), its glyph matrix (MATRIX)
is the linear part of the PDF's text rendering matrix as the viewer computes
it, and every PATH and CLIP of a Typst page carries the PDF's own path: its
CTM, segments and line state as the content stream writes them (§4.4), not
Typst's frame numbers (the frame's rules miss the PDF's by up to 4.8 × 10⁻⁵
bp, enough for 408 pixels at 3×: `docs/evidence/typst-t0-2026-10-04/`).

**`PAGE_META`** (section 8) is a UTF-8 JSON object:

| key | meaning |
|---|---|
| `engine` | `"typst"` (a LaTeX page has no `PAGE_META`) |
| `number` | the page's logical number (Typst's page counter at the page) |
| `label` | the page label the PDF gives the page (`/PageLabels`), when there is one |
| `bleed` | `[left, top, right, bottom]` in bp: the page's bleed; `box` (the MediaBox) includes it |
| `trim` | `[x0, y0, x1, y1]` in bp, stream space: the TrimBox, when the page has bleed |

`counts` stay TeX's and are zero on a Typst page.

### 11.3 Colour spaces and constant alpha (E3; produced)

Every page of a plain Typst document paints in **ICCBased** colour spaces
(sRGB, and a grey profile), and Typst also has spot colours and constant
alpha. A client that accepts `color-spaces` may receive:

**`COLORSPACES`** (section 10): `u32 n`, then n colour spaces, numbered
1..n in order:

```
u8 kind
kind 1, ICCBased:   u8 n (components: 1, 3 or 4); u32 len; u8[len] profile
                    (the ICC profile bytes as the PDF's stream decodes them)
kind 2, Separation: u16 len; u8[len] colorant (the name, decoded from the PDF name);
                    u32 alternate: 0x80000001 DeviceGray, 0x80000003 DeviceRGB,
                       0x80000004 DeviceCMYK, else the number of an
                       earlier ICCBased entry of this section;
                    u8 m; f64[m] c0; f64[m] c1; f64 e
                    (the tint transform, a PDF Type 2 function over [0, 1]:
                     alternate = c0 + t^e × (c1 − c0))
```

**`FILL_COLOR_CS`** / **`STROKE_COLOR_CS`** (`0x0F`, `0x10`): `u32 cs` (1..n
of `COLORSPACES`), `u8 n`, then the n components the PDF's `scn`/`SCN`
writes (for a Separation, n = 1: the tint), read as §4.2 reads a number. A
component count other than the space's is a corrupt page (§7).
`FILL_COLOR`/`STROKE_COLOR` (`0x09`, `0x0A`) remain the Device spaces.

**`FILL_ALPHA`** / **`STROKE_ALPHA`** (`0x11`, `0x12`): `f64`, the PDF's
`ca` / `CA` from the `ExtGState` the content stream selects with `gs`. Both
are graphics state: SAVE/RESTORE scope them. Of an `ExtGState`, v3.3
carries `ca`, `CA` and `LW` only (`/SMask /None`, `/BM /Normal` or
`/Compatible` and `/AIS false` are their defaults): a glyph, path or image
painted while any other key is in effect (a soft mask, a blend mode,
overprint, a transfer function, ...) makes the page INCOMPLETE.

**Colours are the PDF's** (spec §4.2's reading of each `scn`/`SCN`, `g`,
`rg`, `k` operand, and each `ca`/`CA`), for every glyph and path, whichever
space carries them: the Typst host reads them from typst-pdf's export with
the positions (§11.2). ICCBased spaces carry the PDF's profile bytes.

A Separation's tint transform is read as PDF 32000-1 §7.10.3 defines a
Type 2 function: `C0` defaults to `[0]` and `C1` to `[1]`, `/Domain` must
be `[0 1]`, and `C0`/`C1` must have the alternate space's component count;
anything else fails the page's derivation (INCOMPLETE).

Without `color-spaces` the Typst host draws ICCBased sRGB and grey as
DeviceRGB and DeviceGray with the same components and flags a page that
uses alpha or a spot colour INCOMPLETE; it sends no `COLORSPACES` and none
of `0x0F`–`0x13`. (Whether that is pixel-exact for non-black colour is a
gate row, DESIGN.md §15.5.)

### 11.4 Stroked glyphs: `LINE_STATE` (E4; produced)

**`LINE_STATE`** (`0x13`, `accept` `line-state`): the line width, cap, join,
miter limit, dash array and phase (the same encoding and meaning as a
path's stroke, §4.4) for glyphs drawn with text render mode 1 (stroke) or 2
(fill, then stroke), as the PDF's `w`, `J`, `j`, `M`, `d` set them before
the text object. It is graphics state (SAVE/RESTORE scope it). Without
`line-state` a page with stroked text is INCOMPLETE.

**For glyphs the width, dash array and phase are in stream space**, because
a GLYPH carries only the glyph matrix, not the CTM the PDF strokes under.
The PDF's `w` and `d` are user space; under a CTM whose linear part is a
similarity, `[a b −b a]` or `[a b b −a]`, a circular pen of width `w` in
user space is one of width `w × s` in stream space, with
`s = sqrt(a·a + b·b)` in binary64 (each product, the sum and the square root
correctly rounded). The Typst host sends the width, the dashes and the phase
multiplied by `s`; the cap, join and miter limit are unchanged. A client
strokes the glyph's outline, mapped by the glyph matrix, with that pen in
stream space. Under a CTM that is not a similarity (a non-uniform scale, a
skew) the pen is no circle and has no one width: the page is INCOMPLETE.

### 11.5 Images from bytes and PDF islands (E5, E6; specified)

Typst images come from bytes as often as from files (`image(bytes)`,
packages), and typst-pdf re-encodes them. For a client that accepts
`image-data`, an `IMAGE` (§5.2) may say `"data": true` instead of naming a
`file`: its bytes follow in an **`IMAGE_DATA`** frame (`0x4D`) sent right
after it:

```
u32 id                 the IMAGE's id
u32 n                  parts, then n × { u32 len; u8[len] }:
                       part 0: the image data;
                       part 1 (when the IMAGE says "smask": true): the soft mask
                         samples, 8 bits per pixel, rows top first;
                       part 2 (when the IMAGE says "icc": true): the ICC profile
```

3.3 `IMAGE` keys:

| key | meaning |
|---|---|
| `data` | `true`: the bytes are in `IMAGE_DATA`, not in `file` |
| `type` | besides §5.2's: `gif`, `webp` (the file's bytes), and `raw`: the samples of the PDF's image XObject |
| `components`, `bits` | `raw`: components per pixel (1, 3, 4) and bits per component |
| `interpolate` | the PDF's `/Interpolate` (Typst's `smooth` scaling) |
| `smask`, `icc` | parts 1 and 2 are present |
| `island` | `true`: a **PDF island** (E5), below |

For parity a client draws **the pixels the PDF has**: the host sends `raw`
samples (what typst-pdf wrote, after its decoding and colour conversion)
unless the PDF carries the file's own bytes (a JPEG it passes through), when
`type` is `jpeg` and the data is the file.

**PDF islands (E5).** What v3 cannot draw item by item — gradients
(conic included), tilings, SVG images, colour glyphs, gradient-filled text
— the Typst host may send as a one-page PDF that typst-pdf exports for the
construct's bounding box (`page_ranges`, untagged): an `IMAGE` with
`"type": "pdf"`, `"island": true`, `"data": true`, `page` 1, `page_box`
`media`, its `width`/`height` and `orig_x`/`orig_y` in bp, drawn by an IMAGE
item whose matrix maps the island's page space to stream space (§5.2). A
page that would need an island but whose client does not accept
`image-data` is INCOMPLETE. Parity by construction is a **belief** until its
gate row passes (DESIGN.md §15.4).

### 11.6 On-demand source mapping: `RESOLVE`, `LOCATE` (E8; specified)

Re-declaring every span of a long Typst document after each edit costs
about 0.5 s at 300 pages (DESIGN.md §15.4), so a host that lists
`resolve-v1` in its `HELLO` capabilities also answers queries:

- `RESOLVE` (`0x05`): `{"id", "compile", "page", "x", "y"}` — click →
  source: the point (x, y) in page space (sp) of page `page` (0-based) of
  compile `compile`. Reply `RESOLVED` (`0x4E`): `{"id", "file", "line",
  "column", "byte"}` (1-based line, 0-based byte column, byte offset in the
  file), or `{"id", "none": true}`.
- `LOCATE` (`0x06`): `{"id", "compile", "file", "byte"}` — source → page:
  reply `LOCATED` (`0x4F`): `{"id", "positions": [[page, x, y], ...]}` (page
  space, sp), empty when nothing is drawn from there.

`compile` names the compile whose pages the client shows; a host that has
compiled since answers `{"id", "stale": true}` and the client asks again.
The ids are the client's, separate from compile ids. The replies may
interleave with a compile's frames.

### 11.7 Negotiation

The client says `[3, 3]` in `HELLO` and lists in `accept` (§6.2) what it
draws: `color-spaces` (§11.3), `line-state` (§11.4), `image-data` (§11.5),
`font-program-refs` and `font-files` (§11.1). The host lists in
`capabilities` what it can send: the Typst host says `opentype-glyphs`,
`origins-f64`, `page-meta`, `font-program-refs`, `color-spaces` and
`line-state` today, and `resolve-v1` once it answers §11.6. An item opcode or message the client did not accept
is never sent: the host flags the page INCOMPLETE instead (§4.7). A host may
send sections 8 and 10 to any 3.x client; a reader that does not know them
skips them.

`DIAGNOSTIC` (§6.4) gains, in 3.3, `column` (the 0-based byte column of the
diagnostic's start, with `line`) and `hints` (an array of strings).

### 11.8 Packages and the project's lock (Typst host; produced)

The Typst host resolves `#import "@ns/name:version"` from, in order, the
project (`typst-packages/<ns>/<name>/<version>/`, vendored), the host's
read-only package paths (`--package-path`), its package cache
(`--package-cache`) and, for `@preview` only, the Universe or a mirror
(`--package-mirror`). It reads a package's files only inside that package's
canonical root, as it reads the project's only inside the project root;
it opens each file by walking from that root with `O_NOFOLLOW`, so a
symlink swapped in after the check is not followed. A mirror must be
`https://` (redirects only to https) or a local `file://` directory;
`http://` is refused.
**It never fetches unless the compile allows it**: offline is the default,
and `--offline` overrides any client. The app allows it after its first-use
consent sheet (DESIGN.md §15.2). Each project's **lock**,
`flashtex-typst.lock` in `root` (a TOML subset the host writes, meant to be
committed), records the SHA-256 of each fetched package's tarball and of
each font file the document's text uses. The host writes it under an
exclusive `flock` on the project root, re-reading it first (so two hosts
of one project lose nothing), to a new file renamed over it (never through
a symlink); tables and keys of a later version are kept verbatim. Its
first line is a stamp, the SHA-256 of the rest as the host wrote it: a lock
changed outside FlashTeX since (no stamp, or a stamp that does not match) is
never overwritten — a `DIAGNOSTIC` with `"kind": "lock"` says so — until a
`COMPILE` says `"lock": "update"`. A project whose directory or lock is not
writable, or whose root another host keeps locked for more than 2 s (the
host never waits longer), is not written: what the host records is kept in
memory for the session, checked from there and merged with the file when
it is re-read, and one `DIAGNOSTIC` with `"kind": "lock"` says so.

`COMPILE` keys:

| key | meaning |
|---|---|
| `packages` | `offline` (default): use only what is on disk; `online`: this compile may fetch a missing `@preview` package |
| `lock` | `record` (default): check the lock and add what it lacks; `update`: also re-record the fonts as the document uses them now (accepting changed fonts; a package's hash is never replaced); `off`: record nothing and write nothing, report nothing about fonts, but still refuse a package tarball whose hash the lock knows otherwise |

A missing package never makes a compile wait for the network: the host
starts the fetch on its own thread; the compile that started it waits at
most 200 ms, any other compile not at all, and otherwise fails with a
located `DIAGNOSTIC` at the import ("downloading @preview/x:1.2.3 …"). A
fetched tarball, or a cached package's tarball, whose SHA-256 differs from
the lock's entry is refused with a located error and never unpacked or
used; the lock is never changed to accept it. A tarball is unpacked only
if every entry is a regular file or directory inside the package (no `..`,
no absolute path, no link or device) and there are at most 20,000 entries
and 256 MiB in all; otherwise it is refused with a located error, not kept
and not recorded. A cached package's unpacked
files are checked against its tarball once per host process (same files,
same bytes, nothing more); a tree that differs is replaced by a fresh
unpack of the tarball before any file is read. A failed fetch is retried
after 5 s, doubling up to 5 minutes; until then a compile that needs the
package reports the failure without fetching.

**`PACKAGE`** (`0x50`, host → client, JSON; only to a client whose `HELLO`
`accept` lists `packages-v1`, a capability the Typst host lists): what
happened to a package, outside any compile's frames (between compiles, or
interleaved with one like a diagnostic):

```json
{"package": "@preview/cetz:0.3.1", "event": "ready", "sha256": "…", "bytes": 123456}
```

`event`: `needed` (a compile needed it offline: the app may ask the user
for consent, then compile with `"packages": "online"`), `fetching`, `ready`
(compile again), `failed` (with `message`). The reference decoder reads it
as `Event::Package`. The fetch uses the system `curl` (an absolute path,
without the user's `.curlrc`) with a FlashTeX User-Agent, restricted to the
mirror's URL scheme and to 64 MiB, counted while reading.

**Font notes.** After each compile's `DONE` (never before its pages: it may
hash font files) the host records new fonts and checks the lock's fonts
against the installed ones; what it finds goes with **the next compile**
(a `DIAGNOSTIC` after `DONE` would fall outside the compile's frames), and
with every compile after until it is fixed, as a
`DIAGNOSTIC` `{"severity": "warning", "kind": "font", "file": <the lock>,
"message"}` for a recorded font that is missing, replaced by another
variant, or a different file (another SHA-256): an unknown family is only a
quiet warning in Typst, and a different file reflows the document. `kind`
`lock` reports a lock that cannot be read or written (a symlink, a newer
version); such a lock is never overwritten. `HELLO.typst.packages`
describes the host's setup: `{"lock", "vendor", "cache", "mirror",
"offline"}`.

### 11.9 The Typst host's seeded compiles and their check

For an incremental `COMPILE` (not `export`), the Typst host runs Typst's
layout loop seeded with the previous compile's introspection (DESIGN.md
§15.3): usually one layout iteration instead of about four. `DONE` says
how each compile ran:

| key | meaning |
|---|---|
| `seeded` | `true`: the seeded loop produced the pages; `false`: Typst's standard compile did (the first compile, an export, an error, or the seeded loop declined) |
| `iterations` | the seeded loop's layout iterations (0 when `seeded` is false) |
| `verified` | `true`/`false`: the pages were checked against the standard compile and were equal/different; `null`: not checked yet |
| `verify_ms` | what the check cost |

The host checks a seeded compile against the standard one, either before
sending its pages (`--verify every`: the standard pages are sent when
they differ) or **when idle** (the default: after a second without a
message from the client). When the idle check finds different pages, the
host compiles again by itself, as for external tools (§6.4): a follow-up
compile with the last compile's `id` and `"cause": "verify"` (`STARTED`,
the pages that differ, `PAGES`, `DONE`), which a client treats like any
compile. A newer `COMPILE` comes first: the check runs only while the
client is quiet.

### 11.10 The Typst host's watchdog

A Typst compile cannot be cancelled (a WASM plugin runs without a fuel or
memory limit; a `for` over a huge range is unbounded), so the Typst host
watches its own compiles (the first of two layers; the app, the second,
restarts a host on any exit): when **the compile itself** (not the socket
writes, the export, font hashing or eviction after it, so that a slow
client never gets a healthy host killed) runs longer than its wall-time budget
(`--watchdog-secs`, default 10 s; `--watchdog-cold-secs`, default 180 s, for
the first compile of a document and for the idle check of §11.9, both standard
compiles; a 722-page document took 73 s cold to its first page on a loaded
machine, so the app should raise it from a document's last cold time), or the
process's resident memory passes
its ceiling (`--rss-ceiling-mb`, default 4096; 0: none), it writes one line
to stderr, `flashtex-typst-host: {"watchdog": "wall"|"rss", "id", ...}`
(with `over_ms`, how long after the budget ended, or `since_under_ms`, how
long after memory was last seen under the ceiling), kills its children
(package downloads), removes its temporary directory and **`_exit`s with
status 86**; a host starting up removes the temporary directories of hosts
that no longer run. The client sees the socket close during a compile
(no `DONE`): it starts a new host, marks the pages it shows stale and
compiles cold; a client may also kill a host it cannot reach, by `pid`.
`HELLO.watchdog` is `{"wall_ms", "wall_cold_ms", "rss_mb", "exit_code"}`.
