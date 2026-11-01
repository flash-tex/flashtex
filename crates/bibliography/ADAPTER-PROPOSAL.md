# Proposal: compiler citation adapter over `flashtex-bibliography`

Status: **proposal, not agreed**. Owner for review: the compiler owner
(`crates/compiler`, FT-002 lineage) and Commander/orchestrator. This document
changes no code outside `crates/bibliography` and no shared contract. Nothing in
the compiler references this crate until the owner agrees; the crate's public
API is what the proposal consumes and is already exercised by its tests.

## Goal

Render `\cite{key}` as `[1]`/`[Knu84]` and `\bibliography{refs}` as a
`thebibliography` list using only original Rust code, with every diagnostic
pointing at the right byte of the right document, deterministic across runs
and independent of citation order for sorted styles.

## Inputs already available under runtime-v1

`compile` carries `documents: [{path, text}]` and `entry_path`. The `.bib`
file is just another document; the adapter looks it up by the name given in
`\bibliography{...}` (comma-separated stems, `.bib` appended when absent,
resolved relative to the entry document's directory, parent traversal already
rejected by the protocol layer). If the document is not supplied, the adapter
emits an `error` on the `\bibliography{...}` argument span with
`recovery: "bibliography omitted"` — the UI already supplies unsaved text
explicitly, so no filesystem access is added.

## Adapter steps (all inside the compiler, one function per step)

1. **Collect** during the existing parse, without new node kinds: record
   `\cite{a, b}` occurrences as `Citation { key, span }` where `span` is the
   trimmed key text inside the braces (one citation per key), `\nocite{...}`
   the same way (`*` allowed), `\bibliographystyle{plain|unsrt|alpha}` (default
   `plain`; unknown → `warning`, `recovery: "used plain"`), and
   `\bibliography{stems}`. Optional `[note]` on `\cite` is kept as trailing
   text inside the brackets.
2. **Load** each `.bib` document with `flashtex_bibliography::load(text)`.
   Its `diagnostics` are forwarded by a field-wise copy into the compiler's
   `Diagnostic`, converting each `bibliography::Span { start, end }` with
   `compiler::Span::in_document(DocumentId(bib_index), start, end)` where
   `bib_index` is the `.bib` document's position in the `documents` array
   (main as of `3ae7d9b` gives spans a `document` field and serialises the
   path through `to_json_with_paths`). The severity/message/recovery fields
   map one-to-one. One-level crossref and `@string` scope stay per file;
   multiple `.bib` files are concatenated only if the owner prefers that over
   per-file loading — the README lists the tradeoff.
3. **Resolve** with `resolve(&citations, &db, style)`. `missing` diagnostics
   already carry the `.tex` key span; they are forwarded with the citing
   document's `DocumentId`.
   `citations[i]` gives each `\cite` its item index; `items[j].label` the text
   to typeset.
4. **Typeset citations**: replace each `\cite{...}` with `[label]`, joining
   multiple keys with `, ` inside one bracket pair (`[1, 3]`), `[?]` for a
   missing key, note appended after `, `. Text items keep the `\cite`'s own
   source span so click-to-source lands on the citation.
5. **Typeset the list** where `\bibliography{...}` appeared:
   `format_bibliography(&db, &res)` → for each item a hanging-indent paragraph
   starting with `[label]` followed by the blocks joined by a space.
   `Run.style == Emphasis` selects the italic face; U+00A0 is unbreakable;
   text is already decoded (en dashes, accents), so the compiler's plain text
   path renders it without further LaTeX interpretation. Each bibliography
   text item's `source` points at the `.bib` entry's `span` (path = `.bib`),
   which gives click-to-source into the database for free.
6. **Status**: bibliography problems never turn `status` to `failed` on their
   own; missing keys and `.bib` warnings are `recovered`.

## Determinism and incremental notes

- Sorted styles depend only on database content; `unsrt` on first-citation
  order of the entry document. Both are pure functions of the `compile`
  payload, so revision replay is reproducible.
- `load` is the expensive part (linear in `.bib` size); it can be cached per
  `(path, text hash)` across revisions once the compiler has a cache layer.
  `resolve` and `format_bibliography` are cheap and rerun every compile.

## Interface impact

- runtime-v1: **no change**. `documents` already carries the `.bib`; text
  items and diagnostics already have the needed shape.
- Compiler README: add "bibliography (`unsrt`/`plain`/`alpha`, standard BibTeX
  entry types)" to the supported list and "biblatex, other `.bst`" to the
  unsupported list, mirroring this crate's README.
- Cargo: `crates/compiler/Cargo.toml` gains
  `flashtex-bibliography = { path = "../bibliography" }` (path dependency,
  still zero external crates). This is the only compiler file edit the proposal
  requires beyond the adapter module itself, and it is the compiler owner's to
  make.

## Open questions for the owner

1. Per-file vs concatenated `.bib` loading (affects `@string` scope).
2. Whether `\cite` labels should be their own text-item kind for hit-testing
   or plain text items with the `\cite` span (the proposal assumes the latter).
3. Whether `.bib` diagnostics should be suppressed when the `.bib` is not the
   document being edited, to keep the problems list focused (the proposal
   forwards all of them; the UI can filter by path).
