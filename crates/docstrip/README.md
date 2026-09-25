# flashtex-docstrip

A purpose-built interpreter for docstrip batch files (`.ins`) and the
guarded sources (`.dtx`) they strip, so a package that ships only its
sources — `lipsum`, `microtype`, `siunitx`, most of CTAN's
`macros/latex/contrib` — becomes the `.sty`/`.cls`/`.def`/`.cfg` files a
TeX distribution installs, byte for byte. Slice S3 of
[packages, fonts and the project manifest](../../docs/proposals/packages-fonts-manifest.md)
("`.ins`/`.dtx` packages are unpacked with the engine's own docstrip subset
or skipped with a diagnostic"). Platform-free, no dependencies, no network,
no TeX: the reference is `docstrip.dtx` v2.6c (TeX Live, `source/latex/base/`)
and the comments cite its line numbers.

```sh
cargo test --manifest-path crates/docstrip/Cargo.toml                       # unit tests + the TeX Live oracle when a texmf-dist is found
FLASHTEX_DOCSTRIP_SWEEP=1 cargo test --release --manifest-path crates/docstrip/Cargo.toml --test texlive -- --nocapture   # every .ins in TeX Live
cargo run --manifest-path crates/docstrip/Cargo.toml --example run -- path/to/pkg.ins out/   # write what a batch file generates
```

## API

`run(batch_name, batch_bytes, &sources) -> Outcome` (or `run_with` and
`Options { today }` for `\AddGenerationDate` headers). `Sources` answers
`\from{name}`, `\input name` and `\batchinput{name}`: a `BTreeMap<String,
Vec<u8>>` in memory, or `DirSources(dir)` for a directory (relative paths
below it only). `Outcome` has the generated files (`name`, `bytes`, the
`sources` read, the `\usedir` label), the diagnostics (`file:line:
message`), the terminal `messages` (`\Msg` and docstrip's own progress
lines) and `completed`. Nothing fetched is *executed*: the interpreter
only ever writes bytes copied from the sources or the batch file's own
preamble text, and a batch file that programs TeX beyond the commands
below gets a note per unknown command and still yields what `\generate`
names. Runs are bounded (tokens read, tokens pending, input depth).

## What is interpreted

Batch file (`batch.rs`): `\input docstrip`/`l3docstrip` (before it, `\input`
reads a file as TeX does — fontspec inputs its own `.dtx`; after it, `\input`
is docstrip's no-op), `\generate{\file{out}{\from{in}{opts} …}}` with any
number of `\file`s and `\from`s, repeated sources uniquized the way docstrip
does and read in the global first-appearance order, `\needed`, `\checkorder`;
`\preamble`/`\postamble`, `\declarepreamble`/`\declarepostamble`,
`\usepreamble`/`\usepostamble`, `\nopreamble`/`\nopostamble`; `\generateFile`,
`\include`/`\processFile`; `\batchinput`, `\ifToplevel`, `\endbatchfile`,
`\endinput`, `\end`/`\@@end`; `\Msg`, `\typeout`, `\message` (terminal
output, recorded); `\usedir` (recorded), `\BaseDirectory`/
`\UseTDS`/`\DeclareDir` (noted, ignored: without a `docstrip.cfg` docstrip
writes next to the batch file too); `\askforoverwritetrue`/`false`,
`\askonceonly`, `\keepsilent`/`\showprogress`, `\AddGenerationDate`. Plus the
macro layer real batch files rely on: `\def`/`\gdef`/`\edef`/`\xdef` with
delimited and undelimited parameters, `\let`, `\long`/`\global`, groups,
`\catcode` (a real category-code table the lazy lexer reads under),
active-character macros, `\obeyspaces`, `\string`, `\noexpand`, and the
expandable conditionals `\iffalse`/`\iftrue`/`\ifx`/`\if`/`\ifnum`/`\ifcase`
over constants (`\ifeof`, `\ifdim` and other unevaluable ones take their
true branch with a note). Pre- and postambles are what docstrip makes them:
macros `\edef`ed at declaration time with `\outFileName`, `\inFileName`,
`\ReferenceLines` (and a `\let…\relax`ed `\MetaPrefix`) left for write time,
`\checkeoln` dropping the first line end, the line end before `\endpreamble`
being part of the delimiter (`\declarepreambleX#1#2^^M\endpreamble`,
docstrip.dtx line 3531). A batch file may redefine `\declarepreambleX`, as
microtype does to splice `\firstpreamblepart`/`\lastpreamblepart`.

Sources (`strip.rs`, `guard.rs`): `%<*expr>`…`%</expr>` blocks with per-output
off-counters (a nested guard inside an off block is parsed but not
evaluated), `%<expr>`, `%<+expr>`, `%<-expr>`, `%<<TAG` verbatim mode,
`%<@@=module>` replacement in docstrip's order (`@@@@` protected, then
`__@@`, `_@@`, `@@` → `__module`), `%%` meta-comments through `\MetaPrefix`,
other `%` lines dropped, at most one of a run of empty lines, a line that is
exactly `\endinput`, tabs as blanks (dropped at a line start, one space
mid-line), trailing spaces trimmed as TeX's reader does. Guard expressions:
`|` and `,` (or), `&` (and), `!` (not), parentheses; a terminal is any run
of other characters and is true iff `,term,` occurs in `,options,` literally.

Variants (`batch.rs` loader, `strip.rs` guards): `\input ydocstrip`
arms `%<=NAME>text` (define), `%<=+NAME>text` (append),
`%<=*NAME>`…`%<=/NAME>` (multi-line capture) and `%<!NAME>` (insert);
`\input scrdocstrip.tex` arms `%!NAME` (KOMA variable), `\KOMAdefVariable`/
`\KOMAuseVariable`/`\KOMAifVariable`, `\@@input` (the primitive input
docstrip saved, which still reads after docstrip loaded) and the
"extended by scrdocstrip" heading; `\input ctxdocstrip.tex` reads
standard guards with a note (its Lua encoding conversion and `.id`
substitution need an engine). A `%?...` line stays a comment, as under
the real `scrdocstrip` (its `?` arm lives in `\KprocessLineX`, which no
shipped source activates — checked against `tex`). `mwe`, `currfile`,
`standalone`, `adjustbox`, `filehook` and KOMA-Script's `scrmain.ins`
(46 files) generate byte-identical copies of the installed files.

Not interpreted: `\csname`, `\expandafter`, `\the`, registers, `\loop`,
`\read`/`\write`/`\openin`/`\openout`, `\newif`, `\ifdim`/`\ifeof`,
LaTeX's `filecontents`, `ctxdocstrip`'s Lua conversion, beta detection
(`\ifbeta`) in `scrdocstrip`'s heading, and `docstrip.cfg` directory
mapping. Each is a diagnostic naming the command and the line.

## The oracle

`tests/texlive.rs` runs each package's `.ins` over the `.dtx` sources TeX
Live ships in `source/latex/<pkg>/` and compares every generated file byte
for byte with the installed copy under `tex/latex/<pkg>/` (skipped loudly
without a TeX Live tree; `FLASHTEX_TEXMF_DIST` overrides the search). With
MacTeX 2026 the following are identical: lipsum, booktabs, siunitx (344 KB
from 15 sources, some read three times), float, microtype (all 32 files:
`microtype.sty`, three engine `.def`s, `letterspace.sty`, `microtype-show.sty`,
`microtype.cfg` and every `mt-*.cfg`), xcolor (whose `.ins` generates
`xcolor.lox` and `\batchinput`s it), fancyhdr, cleveref, fontspec, amsmath,
graphics, tools and base's `docstrip.ins` (`docstrip.tex`, `doc.sty`,
`shortvrb.sty`, `ltxdoc.cls`, `ltxdoc.cfg`); caption is report-only because
TeX Live's installed copies were generated from an older revision of the
sources it ships. The opt-in sweep over all 1492 batch files in TeX Live
reports 4647 generated files identical, 643 differing, 1380 not installed
(so not comparable); the differing ones are dominated by installed files
generated from other source revisions (bidi, caption, xepersian, abc…),
docstrip variants (measured before variant support; `ydocstrip` users and
KOMA-Script now match — see above), and maintainers' TeX writing 8-bit
bytes as `^^xx`.
Two rules were settled by running the real `tex` on minimal batch files:
the `\endpreamble` delimiter (above) and trailing tabs (blanks, not
trimmed — TeX Live 2026 turns `a}<tab>` into `a} `).
