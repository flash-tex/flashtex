# flashtex-bibliography

Original Rust BibTeX data layer for FlashTeX (issue #2). No existing BibTeX or
TeX engine is invoked, linked, or consulted at run time; zero external crates;
edition 2024. It parses `.bib` databases, resolves `\cite` keys, orders and
labels the bibliography like the standard `unsrt`/`plain`/`alpha` styles, and
emits each entry as styled runs for a layout consumer. It does not typeset.

```sh
cd crates/bibliography
cargo test                                   # 44 tests
cargo clippy --all-targets -- -D warnings
```

```rust
use flashtex_bibliography::{Citation, Span, Style, format_bibliography, load, resolve};

let db = load(bib_text);                     // Database { entries, macros, preambles, diagnostics }
let cites = [Citation::new("knuth84", Span::new(start, end))];   // span of the key in the .tex
let res = resolve(&cites, &db, Style::Plain); // items in final order, labels, missing-key warnings
let items = format_bibliography(&db, &res);   // blocks of styled runs per item
```

Diagnostics use the `docs/contracts/runtime-v1.md` shape
(`severity`, `message`, `source{path,start_byte,end_byte}`, `recovery`) with
zero-based, end-exclusive UTF-8 byte offsets into the text that was parsed;
`Diagnostic::to_json(path)` serialises one. The `.bib` path is supplied by the
caller, exactly as the compiler does for `.tex` diagnostics.

## Supported subset

### Syntax (`parser.rs`)

- `@type{key, field = value, ...}` and `@type(key, ...)`; trailing comma allowed.
- Values: `{balanced braces}`, `"quoted, with {balanced "braces"} inside"`,
  bare integers, macro names, joined with `#`. Whitespace runs inside a value
  collapse to one space and the ends are trimmed; braces and control sequences
  are stored raw so BibTeX's brace semantics survive.
- `@string{name = value}` (concatenation allowed; later definitions override),
  `@preamble{value}`, `@comment{...}` (balanced group) or `@comment` followed by
  the rest of the line. Text outside items is ignored, as BibTeX does, so `%`
  line comments between entries work; `%` inside an entry is not a comment.
- Entry types, field names, and macro names are case-insensitive and stored
  lower-cased. Cite keys keep their spelling but compare case-insensitively
  (BibTeX warns about case-mismatched keys; here the second is a duplicate).
- The twelve month macros `jan`..`dec` are predefined as the standard styles
  define them (`"January"`, ...). A file's `@string{jan = ...}` overrides.
- Spans: every `Entry` has `span` (from `@` through the closing delimiter),
  `type_span`, `key_span`; every `Field` has `span`, `name_span`, `value_span`
  (first part through last part, delimiters included). All are byte-exact on
  multi-byte input (tested).
- Recovery: on a syntax error one `error` diagnostic is reported at the
  offending byte (an empty span at end of input); the item is dropped and
  parsing resumes at the next `@`. The `recovery` text names the dropped item's
  start byte and the resume byte. Undefined macros are a `warning` with the
  macro's span and substitute the empty string. A repeated field in one entry
  is a `warning`; the first occurrence wins.

### Model and validation (`model.rs`)

- `Entry { key, key_span, entry_type, type_span, fields, span }`; `Fields` is
  insertion-ordered with case-insensitive lookup.
- `Database { entries, macros, preambles, diagnostics }` with `get`,
  `index_of`, and `effective_field` (one level of `crossref` inheritance).
- `STANDARD_ENTRY_TYPES`: the fourteen types of btxdoc §3.1 with required
  (alternatives such as `author` or `editor`, `chapter` and/or `pages`) and
  optional fields. `conference` is an alias of `inproceedings`.
- `validate`: duplicate keys are an `error` and the later entry is dropped;
  unknown entry types are a `warning` and formatted as `misc`; missing
  required fields are a `warning` per field (BibTeX reports them as warnings);
  a `crossref` naming a missing entry is a `warning`.

### LaTeX text (`latex.rs`)

`decode` turns accents and letter commands into Unicode and strips braces;
`typeset` adds `--`→`–`, `---`→`—`, ``` `` ```/`''`→`“ ”`, `` ` ``/`'`→`‘ ’`.
`~` becomes U+00A0. Unknown control words are dropped and their brace
arguments kept as text (`\emph{x}` → `x`). Special escapes: `\& \% \$ \# \_
\{ \}`, `\ ` (space), `\,` (U+2009), `\textendash`, `\textemdash`, `\ldots`,
`\dots`, `\textquote*`, `\TeX`, `\LaTeX`, `\BibTeX`, `\S`, `\P`, `\dag`,
`\ddag`, `\pounds`, `\copyright`, `\textregistered`, `\texttrademark`, and a
few more listed in `text_command`.

#### Accent table

Accent commands take their argument as a brace group, another control
sequence (`\'{\i}`), or the next character. `\i`/`\j` are treated as `i`/`j`
bases. Pairs in the table compose to the precomposed character; any other
base gets the combining mark instead (`\'{\ae}` → `æ` + U+0301).

| Command | Mark | Precomposed bases |
|---|---|---|
| `\'` | U+0301 acute | a e i o u y A E I O U Y c C n N s S z Z l L r R g G |
| `` \` `` | U+0300 grave | a e i o u A E I O U n N |
| `\^` | U+0302 circumflex | a e i o u A E I O U c C g G h H j J s S w W y Y |
| `\"` | U+0308 diaeresis | a e i o u y A E I O U Y |
| `\~` | U+0303 tilde | a n o A N O i I u U |
| `\=` | U+0304 macron | a e i o u A E I O U |
| `\.` | U+0307 dot above | c C e E g G z Z I |
| `\u` | U+0306 breve | a A g G u U e E i I o O |
| `\v` | U+030C caron | c C d D e E n N r R s S t T z Z a A i I o O u U |
| `\H` | U+030B double acute | o O u U |
| `\c` | U+0327 cedilla | c C s S t T g G k K l L n N r R |
| `\k` | U+0328 ogonek | a A e E i I u U |
| `\r` | U+030A ring | a A u U |
| `\d` | U+0323 dot below | a A e E i I o O u U |
| `\b` | U+0331 macron below | (combining only) |
| `\t` | U+0361 double inverted breve | (combining only) |

Letter commands: `\ss`→ß `\SS`→SS `\o`→ø `\O`→Ø `\aa`→å `\AA`→Å `\ae`→æ
`\AE`→Æ `\oe`→œ `\OE`→Œ `\l`→ł `\L`→Ł `\i`→ı `\j`→ȷ `\dh`→ð `\DH`→Ð
`\th`→þ `\TH`→Þ `\ng`→ŋ `\NG`→Ŋ `\dj`→đ `\DJ`→Đ.

BibTeX built-ins, implemented on the raw text: `purify` (`purify$`: keep
alphanumerics and whitespace, `-`/`~` become spaces, inside a special
character the control sequence is dropped unless it is a letter command, so
`{\'e}`→`e`, `{\ss}`→`ss`, `{\TeX}`→empty), `change_case` (`change.case$`
`t`/`l`/`u`: brace groups untouched, special characters converted including
`\AE`↔`\ae`, title mode keeps the first character and the first after
`: `), `text_length`/`text_prefix` (counted on decoded text, so a special
character is one character), `add_period` (`add.period$`).

### Names (`names.rs`)

- `split_names`: ` and ` (case-insensitive, whitespace on both sides) at brace
  depth 0; `{Barnes and Noble}` stays one name.
- `parse_name`: tokens are separated by whitespace and `~` at depth 0;
  depth-0 commas select the form: `First von Last`, `von Last, First`,
  `von Last, Jr, First` (further commas join First). The von rule is BibTeX's
  `von_token_found`: the first letter at depth 0 decides by case; digits and
  other non-letters are skipped; a leading non-special brace group makes the
  token non-von (`{de} Gaulle`); a special character decides by a known letter
  command's case (`\o` von, `\O` not) or the first letter after the command.
  In the no-comma form the final token is never von and Last runs from the
  token after the last von token; in the comma forms von runs from the first
  token through the last lower-case token before the final one.
- `Name::is_others()` marks `others`.
- `format_name(name, template)` implements the `format.name$` template
  language for the templates the standard styles use (`{ff~}{vv~}{ll}{, jj}`,
  `{f.~}{vv~}{ll}{, jj}`, `{vv{ } }{ll{ }}{  ff{ }}{  jj{ }}`, `{v{}}{l{}}`,
  `{ll}`). Rules: a piece is silent when its part is empty; `f` abbreviates
  each token to its first character (a leading special character counts as
  one; `Jean-Baptiste` → `J.-B.`); with the default separator every
  abbreviated token but the last gets a `.`; the default inter-token
  separator is a tie when the token just written is shorter than three
  characters or the next token is the last of the part, otherwise a space;
  a single trailing `~` is a tie when the part text is shorter than three
  characters, else a space; `~~` forces a tie. These rules are reconstructed
  from btxhak and observed `.bbl` output (`Donald~E. Knuth`, `D.~E. Knuth`,
  `L.~Lamport`, `C.~A.~R. Hoare`); they were not checked against the WEB
  source, see "Provenance".

### Resolution (`resolve.rs`)

`resolve(&[Citation { key, span }], &db, style)` returns
`Resolution { items, citations, missing }`:

- `items`: `BibItem { key, entry, label, cited_by }` in final order.
- `citations[i]`: the item index the i-th citation resolved to, or `None`.
- `missing`: one `warning` per unresolved citation with that citation's span
  and `recovery: rendered the citation as [?] ...`.
- The key `*` (`\nocite{*}`) appends every uncited entry in database order.
- `Style::Unsrt`: first-citation order, labels `1..n`.
- `Style::Plain`: `plain.bst` `presort`, then labels `1..n`. The sort key is
  `names "    " year "    " title` where names come from author (book/inbook:
  author or editor; proceedings: editor or organization; manual: author or
  organization; else the `key` field), each name as
  `{vv{ } }{ll{ }}{  ff{ }}{  jj{ }}` joined by three spaces, `others` as
  `et al`; the title drops a leading `A `, `An `, `The ` (case-sensitive, as
  `chop.word`); everything is `purify$`-ed and lower-cased. Ties keep
  first-citation order (stable sort).
- `Style::Alpha`: `alpha.bst` `calc.label`: one author → von+last initials
  (`vN`), or the first three characters of the last name when that is shorter
  than two (`Knu`); 2–4 authors → initials (`GMS`); more → first three plus
  `+`; a final `others` → `+`; no names → the `key` field, organization
  (without `The `), or the cite key, cut to three characters; then the last
  two characters of the purified year. Sorted by the purified lower-cased
  label, then the plain key; runs of equal labels get `a`, `b`, ... in that
  order. Labels are decoded to Unicode and returned without brackets.

### Layout data (`format.rs`)

`format_entry(&db, &entry, label)` → `FormattedEntry { key, label, blocks,
warnings }`; `blocks` are `\newblock` units of `Run { text, latex, style }`
with `style ∈ {Plain, Emphasis, Bold}`. `text` is the decoded, typeset-ready
string (en dashes, no-break spaces, composed accents); `latex` is the raw
text `plain.bst` would write. `FormattedEntry::text()` joins blocks with a
space; `to_bbl(style)` renders the `\bibitem` as a `.bbl` would show it, for
comparison against real BibTeX output. Consumers typeset `Emphasis` in italic
and treat U+00A0 as unbreakable; `Bold` is reserved and unused by `plain`.

Entry functions follow `plain.bst` for all thirteen types plus `conference`
(= `inproceedings`) and the `misc` default for unknown types: author/editor
formatting with `{ff~}{vv~}{ll}{, jj}` and `, and`/`et~al.` joining, `t`-case
article titles, emphasised book titles/journals/booktitles/series, `In
Editor, editor, Booktitle`, `volume~5 of Series`, `Number 3 in Series`,
`27(2):97--111`, `pages 1--10`/`page 5`, `chapter~3`, `second edition`,
`Master's thesis`/`PhD thesis`/`type`, `Technical Report TR-7`, `Month Year`,
`tie.or.space.connect`, `n.dashify`, and the `output.state` machine
(`, ` mid-sentence, `. ` after a sentence, `.\newblock` after a block,
`add.period$` at the end). Style warnings `plain.bst` prints
(`can't use both volume and number fields`, `there's a number but no
volume`/`series`, `there's a month but no year`, empty `misc`) are returned in
`warnings` with the entry's span.

## Provenance of style conventions

- Entry-type field tables: *BibTeXing* (btxdoc) §3.1 as reproduced by
  `plain.bst`'s `ENTRY`/`output.check` calls.
- Formatting, ordering, and `output.state`: transliterated from the `plain.bst`
  functions of the same names (`article` ... `unpublished`, `format.names`,
  `format.editors`, `format.title`, `format.btitle`, `format.bvolume`,
  `format.number.series`, `format.edition`, `format.date`, `format.pages`,
  `format.vol.num.pages`, `format.chapter.pages`, `format.in.ed.booktitle`,
  `format.thesis.type`, `format.tr.number`, `new.block`, `new.sentence`,
  `output.nonnull`, `fin.entry`, `presort`, `sort.format.names`,
  `sort.format.title`, `chop.word`) and from `alpha.bst` (`calc.label`,
  `format.lab.names`, `author.key.label` and siblings, `forward.pass`).
  `unsrt.bst` is `plain.bst` without `SORT`. These were written from the
  author's knowledge of those files; the crate is not derived from their text.
- Built-in functions (`purify$`, `change.case$`, `text.prefix$`,
  `add.period$`, `format.name$`, `von_token_found`): behaviour as documented in
  btxhak ("Designing BibTeX styles") and Tame the BeaST, plus known `.bbl`
  output. The `format.name$` tie rules are a reconstruction (see "Names").
- Goldens in `tests/format.rs` are what this crate produces and match the
  author's expectation of `plain.bst` output for those entries. They have not
  been diffed against a real BibTeX run in this repository; doing so is a
  worthwhile verification step for whoever has a TeX installation (the
  product never runs one).

## Unsupported

- biblatex: `@online`, `date`, `journaltitle`, name-list `and others`
  extensions, `\DeclareNameFormat` — none of it.
- `crossref` beyond one level; `plain.bst`'s crossref-specific formatting
  (`In \cite{parent}` and `format.*.crossref`) — inherited fields are formatted
  as if they were the entry's own.
- `@string` across files: `load` takes one source; a caller that merges files
  must define macros in the file that uses them or concatenate texts first
  (spans then refer to the concatenation).
- Styles other than `unsrt`, `plain`, `alpha`; no `abbrv` (its `{f.~}`
  template is implemented in `format_name` but not wired), no author-year.
- `.bst` interpretation in general: the conventions are hard-coded.
- BibTeX's 8-bit `lex_class` tables: this crate uses Unicode classes, so raw
  UTF-8 letters are letters for case/von decisions (BibTeX would treat them as
  non-letters).
- `%` comments inside entries; `@comment` semantics beyond the two forms above.
- `text.prefix$`/`text.length$` on raw brace structure: measured on decoded
  text instead (differs only for pathological brace nesting).
- Per-file `Database::merge`; multi-path diagnostics; label output for
  `\cite` typesetting itself (the adapter proposal covers that).

## Compiler citation adapter

See `ADAPTER-PROPOSAL.md`. It is a proposal awaiting compiler-owner agreement,
not a contract, and nothing in the compiler references this crate yet.
