# Proposal: `display-list-v2-images` — image items for `display-list-v2`

Status: **PROPOSAL ONLY** (FT-063, agent `kabir-claude`, branch
`agent/kabir-claude/graphics-floats`). Additive and opt-in. It changes no
frozen contract: `protocol/rendering-v2.schema.json`,
`docs/contracts/runtime-v1*.md` and the `display_list` line of a request that
does not ask for this capability stay byte-for-byte as they are. Needs a
consumer co-sign (Mac painter + PDF export, `mac-claude-a`) before the Mac
side relies on it.

Consumer: Mac co-signed 2026-09-13, base main fe864d90 (lane `mac-images`,
branch `agent/mac-images/v2-consumer`): §2 request (`display-list-v2-images`
alongside `display-list-v2`, `payload.project_root`), §3 item decoding and
validation, §5.1–5.3 (rooted symlink-refusing read, `byte_length` + SHA-256
verified before decoding, cache by `sha256`, `image` treated as a required
feature) and §5.4 for the CoreGraphics `Export PDF (v2)…` route are
implemented in `apps/mac` (`V2ImageStore.swift`, `V2ImageTests`). Not yet
co-signed: §5.4 for `flashtex-pdf-exact from-v2` (`crates/pdf` refuses image
items). Live round trip verified against `flashtex-render` at this SHA.

## 1. Summary

1. New layout capability string `display-list-v2-images`. It is accepted only
   when the same request also lists `display-list-v2`; unknown to old
   producers (never echoed), never sent by old consumers.
2. When accepted, the `display_list` sibling line may contain items with
   `"kind": "image"`, and `required_features` gains `"image"` whenever at
   least one image item is present. When not accepted, image items are
   **omitted from the line** (the layout is identical — floats keep their
   space) and `required_features` is unchanged.
3. Image bytes are **not** in the line. Each item names the file by
   project-relative `path` plus `sha256` and `byte_length`; the consumer
   reads the bytes through project-files (rooted, no symlinks) and must refuse
   to paint an image whose bytes do not hash to `sha256` (draw nothing, keep
   the frame; optionally surface a stale-asset notice).
4. The producer reads image headers for sizing from an optional absolute
   `payload.project_root` in the compile request (or the worker's
   `--project-root DIR`). Without a root, an `image_unavailable` error
   diagnostic names the file; nothing is guessed.
5. The runtime-v1 `compile_result` has no image item and is unchanged.

## 2. Request

```
{"protocol_version":1,"id":"…","type":"compile","payload":{
  "project_id":"…","revision":N,"entry_path":"main.tex","documents":[…],
  "layout_capabilities":["display-list-v2","display-list-v2-images"],
  "project_root":"/Users/me/Projects/paper"          -- OPTIONAL, absolute; ignored otherwise
}}
```

The echoed `accepted` list names `display-list-v2-images` only when it was
honoured (same rule as the other layout capabilities).

**PROPOSAL addition (FT-063, helper route, `kabir-claude`):** on the
preview-controller helper route the client does not build compile requests.
A file-backed helper (startup config `project_root`) forwards its canonical
project directory itself: the producer is launched with
`--project-root <dir>` and every compile request carries the same
`payload.project_root` (per request, authoritative; survives `restart`). The
directory is the one `FileProject` opened, after checks that it is absolute,
existing, a directory, UTF-8 and already canonical (a symlinked or `..`
spelling is refused, matching the rooted symlink-refusing reads). Store-backed
helpers send neither; their launch argv and request bytes are unchanged.
Producers that do not know the flag or field ignore them. No new field is
needed from the Mac client on this route.

## 3. Item shape

Coordinates follow the frozen list: `bp_2pow20` integer ticks, y down from
the page's top-left.

```
{"kind":"image",
 "x":<ticks>,"top":<ticks>,"width":<ticks>,"height":<ticks>,   -- bounding box on the page
 "transform":[a,b,c,d,e,f],                                   -- PDF points, see below
 "image":{
   "image_id":"<sha256 hex>",        -- = sha256; the resource identity
   "sha256":"<hex64>","byte_length":<int>,
   "format":"png"|"jpeg"|"pdf",
   "path":"figures/plot.pdf",       -- project-relative, as resolved (extension search applied)
   "pixel_width":<int>,"pixel_height":<int>,      -- png/jpeg only
   "pdf_page":<int>,"pdf_box":[llx,lly,urx,ury],"pdf_rotate":0|90|180|270   -- pdf only
 },
 "sources":[{"path":"main.tex","start_byte":…,"end_byte":…}]  -- the \includegraphics command
}
```

`transform` maps the image's unit square — `u` to the right, `v` up, `(0,0)`
at the image's lower-left corner — to page points with y down:

```
page_x = e + a·u + c·v
page_y = f + b·u + d·v
```

An unrotated image at bounding box `(x, top, w, h)` (points) has
`[w, 0, 0, -h, x, top + h]`. `angle=90` gives `[0, -w', h', 0, …]`-style
quarter-turn matrices; arbitrary angles are allowed (the bounding box then
encloses the rotated square). Values are rounded to 1/1000 pt; the tick box
is the exact geometry and the painter should clip to it.

For `format: "pdf"` the unit square is the named page's `pdf_box` (CropBox
clipped to MediaBox, graphicx's default `pagebox`), after applying
`pdf_rotate` clockwise as a PDF viewer would.

Paint order: image items appear in the page's `items` array like rules and
glyph runs; paint in array order. Text selection/hit testing ignores images;
`sources` allows source navigation from a click on the bounding box.

## 4. Producer semantics (implemented)

- Natural size follows pdfTeX: PNG `pHYs` (metre unit, dpi rounded to an
  integer like `writepng.c`), else 72 dpi; JPEG JFIF density (dpi or dpcm),
  else 72 dpi; PDF page box in points with `/Rotate`. PDFs whose page tree is
  only in compressed object streams are reported (`image_unavailable`), not
  sized by guesswork.
- graphicx keys: `width`, `height`, `totalheight`, `scale`, `angle`,
  `keepaspectratio`, `page`, with graphicx's order rule (size keys before the
  first `angle` size the unrotated image; after it they rescale the rotated
  box). Units `pt bp in cm mm pc dd cc sp em ex` and factors of `\textwidth`,
  `\linewidth`, `\columnwidth`, `\textheight`, `\paperwidth`, `\paperheight`.
  `trim`/`clip`/`viewport`/`bb`/`origin` are reported (`graphics_option`)
  and not honoured yet.
- Extension search for names without one: `.pdf .png .jpg .jpeg .PDF .PNG .JPG`.
- Scope today: `\includegraphics` inside `figure`/`table` floats (see
  `crates/render-pipeline/src/typeset/floatpage.rs`). An `\includegraphics`
  in running text keeps the compiler's existing "unsupported" warning.

## 5. Consumer obligations (for the co-sign)

1. Advertise `display-list-v2-images` only if the painter can draw PNG, JPEG
   and page 1..N of a PDF into an affine-transformed rectangle.
2. Read bytes with project-files' rooted read of `path`; verify `byte_length`
   and `sha256` before decoding; cache by `sha256`.
3. Treat `"image"` in `required_features` like any other required feature:
   a consumer that did not negotiate the capability never receives it.
4. PDF export: embed the same bytes (PNG/JPEG as image XObjects, PDF pages as
   form XObjects) with `transform` as the `cm` operator after converting to
   PDF's y-up page space (`[a, -b, c, -d, e, H - f]` for page height `H`).

## 6. What the Mac side still needs (not in this task's paths)

- `apps/…` preview: request the capability, pass `project_root` (the open
  project's directory), implement the image painter and asset cache, and
  refuse on hash mismatch.
- PDF export route (`crates/pdf` / Mac export): consume image items as in §5.4.
- Neither is implemented here; FT-063 cannot edit `apps/` or `crates/pdf`.

## 7. Evidence

`crates/render-pipeline/tests/floats_oracle.rs` checks 10 fixtures
(`crates/render-pipeline/fixtures/floats/`) against pdfLaTeX references and
the capability gate (frozen line has no image item; negotiated line lists
`image`). Numbers: `crates/render-pipeline/docs/evidence/floats/README.md`.
