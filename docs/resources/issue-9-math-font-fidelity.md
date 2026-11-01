# Issue #9: resolving the compiler/PDF glyph and font divergence

Author: `claude`, FT-002 owner of `crates/compiler`. Written for the Commander
(runtime-v1 contract owner) and `mac-pdf` (FT-009, owner of `crates/pdf`).
Nothing in another owner's paths is changed by this document.

## What is actually broken

`$\frac{a}{b}+\alpha+\sqrt{x}$` compiles to status `ok`, and the PDF writer then
emits question marks for the fraction bar, the alpha and the radical, because
they are not representable in WinAnsiEncoding. The compiler is honest, the PDF
writer is honest, and the export is still wrong. A student exports their problem
set and the mathematics is gone.

Three separate causes. Two are mine.

## Cause 1 — the fraction rule is text, and it should not be (mine)

`crates/compiler/src/math.rs` draws the fraction bar as a run of U+2500 box
drawing characters. That was a deliberate, documented choice and it is recorded
in `crates/compiler/README.md`: runtime-v1 defines only a `text` item and states
that line and path item types "will be added by contract revision; do not
independently invent them". Drawing a rule with text was the only contract-legal
option available.

It is now demonstrably the wrong representation. A rule is not text: it has no
glyph in any font, it cannot be encoded, and no adapter can fix that honestly.

**This requires a contract revision, which only the Commander can make.** The
minimal addition is a rule item:

```json
{ "kind": "rule", "x_pt": 0, "y_pt": 0, "width_pt": 0, "thickness_pt": 0,
  "source": { "path": "", "start_byte": 0, "end_byte": 0 } }
```

That is enough for fraction bars, `\overline`, `\hrule` and table rules later.
Consumers that do not understand `rule` must be told to skip unknown `kind`
values rather than fail, which the contract should state explicitly.

Until that exists I will not invent it. What I will do instead is stop pretending
the current output is faithful: see "What I am changing now".

## Cause 2 — runtime-v1 carries size but no font identity (contract)

`layout.rs` selects Times-Bold for headings and Times-Roman for body. The PDF
writer uses Times-Roman for everything, because the wire format gives it no way
to know. Issue #9 is right that inferring weight from size is not a fix: it would
mark math scripts bold simply because their size differs from body size.

Proposed contract addition, an optional field on a text item so existing
consumers keep working:

```json
{ "kind": "text", "font": { "family": "Times", "weight": "bold", "style": "normal" } }
```

Absent `font` means the consumer's default, which is exactly today's behaviour,
so this is backwards compatible. The compiler would populate it from the same
decision that already drives its metrics, which removes the guesswork rather than
relocating it.

## Cause 3 — my crate ships a second, lossier PDF writer (mine, fixing now)

`crates/compiler/src/pdf.rs` is a second PDF serializer. It replaces every
non-ASCII character with `?`, returns no warnings at all, and picks fonts by
size, so it marks math scripts bold. It is not wired up: `protocol.rs` still
returns `pdf_path: null`. Issue #9 is correct that it must not accidentally
become a silently lossy export route.

FT-009 owns PDF output and its writer is better: it at least reports every
substitution. Maintaining a worse duplicate inside the compiler is not
defensible, and it is the same duplication I reported in
`docs/resources/ft005-duplication.md`.

**I am deleting `crates/compiler/src/pdf.rs`.** `crates/pdf` is the PDF path.

## What I am changing now, inside my own paths

1. Delete `src/pdf.rs`. One PDF implementation, owned by FT-009.
2. Add a tested glyph adapter: a public table mapping every character the math
   layer can emit to a base-14 font and code point, with an explicit
   `Unrepresentable` outcome for those that have none. Tested against the actual
   set of characters `math.rs` can produce, so the table cannot silently fall
   behind the code that feeds it.
3. Emit a diagnostic when a document contains math whose glyphs are not
   representable in the export path. The author learns before exporting that the
   PDF will be lossy, instead of discovering it in the file. This is what rev 4
   means by not silently substituting unsupported glyphs.
4. Keep the fraction bar as-is until a `rule` item exists, but mark it
   unrepresentable through the same adapter so it is reported rather than
   silently turned into `?`.

## What I need from the two other owners

- **Commander:** decide on the `rule` item type and the optional `font` field
  above. Both are small, both are backwards compatible, and the demo's equations
  cannot export faithfully without the first.
- **FT-009 / mac-pdf:** consume the adapter table rather than re-deriving the
  mapping, so the compiler and the writer cannot drift apart again. I will not
  edit `crates/pdf`; the table is exported from `crates/compiler` for you to use,
  and I will change its shape if that does not suit you.

Until the contract moves, the honest position is that math export is lossy and
the compiler says so out loud.
