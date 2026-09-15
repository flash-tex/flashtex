# Kernel inventory audit (slice 1)

## Method

`crates/compiler/supported/canonical-latex.tsv` (header: `set\tkind\tname`, where `set` is the source package and `kind` is `command` or `environment`) was filtered to its 440 `kernel` rows (410 commands, 30 environments). For each name, `grep -rnwF -e 'NAME' crates/compiler/src/` (fixed-string, whole-word, recursive) was run from the repository root to test for any implementation reference, and `grep -rnwF -e 'NAME' crates/compiler/tests/` for integration-test coverage; inline `#[cfg(test)]` modules were separated from production code by treating every `#[cfg(test)] mod …` region (verified to run to end-of-file in each file) as test code. Short names that whole-word grep over-counts (single letters colliding with Rust identifiers, SI-unit symbols, roman numerals, column specifiers) and comment-only mentions were adjudicated by reading every match in context; matches confined to `vocabulary.rs` `KNOWN_UNIMPLEMENTED_*` lists, `//` comments, or unrelated tables (SI units, roman numerals, tabular alignment) were ruled *spurious*, not implementation. Cross-checks: `crates/compiler/src/supported.rs` (the inventory-as-data; `EXPANSION_COMMANDS` entries are executed by the `flashtex-tex-expansion` engine in `crates/tex-expansion/`, so they leave no string trace in `crates/compiler/src/`) and `crates/compiler/tests/supported_latex.rs::every_inventory_entry_compiles_without_an_unsupported_diagnostic` (run 2026-09-14: 1 passed, 0 failed — every inventory entry compiles without a "not supported" diagnostic in at least one context). Table 1A rows reproduce as empty output; Table 1B/2 rows give the discriminating evidence found.

## Table 1A — kernel names with ZERO matches in `crates/compiler/src/` (153)

| kind | name | reproducing grep (run from repo root) | result |
| ---- | ---- | ------------------------------------- | ------ |
| command | AtBeginDvi | `grep -rnwF -e 'AtBeginDvi' crates/compiler/src/` | (no output) |
| command | AtEndDvi | `grep -rnwF -e 'AtEndDvi' crates/compiler/src/` | (no output) |
| command | AtEndOfClass | `grep -rnwF -e 'AtEndOfClass' crates/compiler/src/` | (no output) |
| command | AtEndOfPackage | `grep -rnwF -e 'AtEndOfPackage' crates/compiler/src/` | (no output) |
| command | CheckCommand | `grep -rnwF -e 'CheckCommand' crates/compiler/src/` | (no output) |
| command | ClassError | `grep -rnwF -e 'ClassError' crates/compiler/src/` | (no output) |
| command | ClassInfo | `grep -rnwF -e 'ClassInfo' crates/compiler/src/` | (no output) |
| command | ClassWarning | `grep -rnwF -e 'ClassWarning' crates/compiler/src/` | (no output) |
| command | ClassWarningNoLine | `grep -rnwF -e 'ClassWarningNoLine' crates/compiler/src/` | (no output) |
| command | CurrentOption | `grep -rnwF -e 'CurrentOption' crates/compiler/src/` | (no output) |
| command | DeclareFontEncoding | `grep -rnwF -e 'DeclareFontEncoding' crates/compiler/src/` | (no output) |
| command | DeclareOption | `grep -rnwF -e 'DeclareOption' crates/compiler/src/` | (no output) |
| command | DeclareTextAccent | `grep -rnwF -e 'DeclareTextAccent' crates/compiler/src/` | (no output) |
| command | DeclareTextAccentDefault | `grep -rnwF -e 'DeclareTextAccentDefault' crates/compiler/src/` | (no output) |
| command | DeclareTextCommand | `grep -rnwF -e 'DeclareTextCommand' crates/compiler/src/` | (no output) |
| command | DeclareTextComposite | `grep -rnwF -e 'DeclareTextComposite' crates/compiler/src/` | (no output) |
| command | DeclareTextCompositeCommand | `grep -rnwF -e 'DeclareTextCompositeCommand' crates/compiler/src/` | (no output) |
| command | DeclareTextSymbol | `grep -rnwF -e 'DeclareTextSymbol' crates/compiler/src/` | (no output) |
| command | DeclareTextSymbolDefault | `grep -rnwF -e 'DeclareTextSymbolDefault' crates/compiler/src/` | (no output) |
| command | DocumentMetadata | `grep -rnwF -e 'DocumentMetadata' crates/compiler/src/` | (no output) |
| command | ExecuteOptions | `grep -rnwF -e 'ExecuteOptions' crates/compiler/src/` | (no output) |
| command | IfFileExists | `grep -rnwF -e 'IfFileExists' crates/compiler/src/` | (no output) |
| command | InputIfFileExists | `grep -rnwF -e 'InputIfFileExists' crates/compiler/src/` | (no output) |
| command | LastDeclaredEncoding | `grep -rnwF -e 'LastDeclaredEncoding' crates/compiler/src/` | (no output) |
| command | LoadClass | `grep -rnwF -e 'LoadClass' crates/compiler/src/` | (no output) |
| command | LoadClassWithOptions | `grep -rnwF -e 'LoadClassWithOptions' crates/compiler/src/` | (no output) |
| command | NeedsTeXFormat | `grep -rnwF -e 'NeedsTeXFormat' crates/compiler/src/` | (no output) |
| command | OptionNotUsed | `grep -rnwF -e 'OptionNotUsed' crates/compiler/src/` | (no output) |
| command | PackageError | `grep -rnwF -e 'PackageError' crates/compiler/src/` | (no output) |
| command | PackageInfo | `grep -rnwF -e 'PackageInfo' crates/compiler/src/` | (no output) |
| command | PackageWarning | `grep -rnwF -e 'PackageWarning' crates/compiler/src/` | (no output) |
| command | PackageWarningNoLine | `grep -rnwF -e 'PackageWarningNoLine' crates/compiler/src/` | (no output) |
| command | PassOptionsToClass | `grep -rnwF -e 'PassOptionsToClass' crates/compiler/src/` | (no output) |
| command | ProcessOptions | `grep -rnwF -e 'ProcessOptions' crates/compiler/src/` | (no output) |
| command | ProvideTextCommand | `grep -rnwF -e 'ProvideTextCommand' crates/compiler/src/` | (no output) |
| command | ProvideTextCommandDefault | `grep -rnwF -e 'ProvideTextCommandDefault' crates/compiler/src/` | (no output) |
| command | ProvidesClass | `grep -rnwF -e 'ProvidesClass' crates/compiler/src/` | (no output) |
| command | ProvidesFile | `grep -rnwF -e 'ProvidesFile' crates/compiler/src/` | (no output) |
| command | ProvidesPackage | `grep -rnwF -e 'ProvidesPackage' crates/compiler/src/` | (no output) |
| command | RequirePackageWithOptions | `grep -rnwF -e 'RequirePackageWithOptions' crates/compiler/src/` | (no output) |
| command | UseTextAccent | `grep -rnwF -e 'UseTextAccent' crates/compiler/src/` | (no output) |
| command | UseTextSymbol | `grep -rnwF -e 'UseTextSymbol' crates/compiler/src/` | (no output) |
| command | addtocontents | `grep -rnwF -e 'addtocontents' crates/compiler/src/` | (no output) |
| command | arraycolsep | `grep -rnwF -e 'arraycolsep' crates/compiler/src/` | (no output) |
| command | baselinestretch | `grep -rnwF -e 'baselinestretch' crates/compiler/src/` | (no output) |
| command | bigbreak | `grep -rnwF -e 'bigbreak' crates/compiler/src/` | (no output) |
| command | bottomfraction | `grep -rnwF -e 'bottomfraction' crates/compiler/src/` | (no output) |
| command | capitalacute | `grep -rnwF -e 'capitalacute' crates/compiler/src/` | (no output) |
| command | capitalbreve | `grep -rnwF -e 'capitalbreve' crates/compiler/src/` | (no output) |
| command | capitalcaron | `grep -rnwF -e 'capitalcaron' crates/compiler/src/` | (no output) |
| command | capitalcedilla | `grep -rnwF -e 'capitalcedilla' crates/compiler/src/` | (no output) |
| command | capitalcircumflex | `grep -rnwF -e 'capitalcircumflex' crates/compiler/src/` | (no output) |
| command | capitaldieresis | `grep -rnwF -e 'capitaldieresis' crates/compiler/src/` | (no output) |
| command | capitaldotaccent | `grep -rnwF -e 'capitaldotaccent' crates/compiler/src/` | (no output) |
| command | capitalgrave | `grep -rnwF -e 'capitalgrave' crates/compiler/src/` | (no output) |
| command | capitalhungarumlaut | `grep -rnwF -e 'capitalhungarumlaut' crates/compiler/src/` | (no output) |
| command | capitalmacron | `grep -rnwF -e 'capitalmacron' crates/compiler/src/` | (no output) |
| command | capitalnewtie | `grep -rnwF -e 'capitalnewtie' crates/compiler/src/` | (no output) |
| command | capitalogonek | `grep -rnwF -e 'capitalogonek' crates/compiler/src/` | (no output) |
| command | capitalring | `grep -rnwF -e 'capitalring' crates/compiler/src/` | (no output) |
| command | capitaltie | `grep -rnwF -e 'capitaltie' crates/compiler/src/` | (no output) |
| command | capitaltilde | `grep -rnwF -e 'capitaltilde' crates/compiler/src/` | (no output) |
| command | circle | `grep -rnwF -e 'circle' crates/compiler/src/` | (no output) |
| command | closein | `grep -rnwF -e 'closein' crates/compiler/src/` | (no output) |
| command | closeout | `grep -rnwF -e 'closeout' crates/compiler/src/` | (no output) |
| command | columnsep | `grep -rnwF -e 'columnsep' crates/compiler/src/` | (no output) |
| command | columnseprule | `grep -rnwF -e 'columnseprule' crates/compiler/src/` | (no output) |
| command | contentsline | `grep -rnwF -e 'contentsline' crates/compiler/src/` | (no output) |
| command | dashbox | `grep -rnwF -e 'dashbox' crates/compiler/src/` | (no output) |
| command | dotfill | `grep -rnwF -e 'dotfill' crates/compiler/src/` | (no output) |
| command | evensidemargin | `grep -rnwF -e 'evensidemargin' crates/compiler/src/` | (no output) |
| command | floatpagefraction | `grep -rnwF -e 'floatpagefraction' crates/compiler/src/` | (no output) |
| command | floatsep | `grep -rnwF -e 'floatsep' crates/compiler/src/` | (no output) |
| command | flushbottom | `grep -rnwF -e 'flushbottom' crates/compiler/src/` | (no output) |
| command | fontshape | `grep -rnwF -e 'fontshape' crates/compiler/src/` | (no output) |
| command | footskip | `grep -rnwF -e 'footskip' crates/compiler/src/` | (no output) |
| command | frenchspacing | `grep -rnwF -e 'frenchspacing' crates/compiler/src/` | (no output) |
| command | fussy | `grep -rnwF -e 'fussy' crates/compiler/src/` | (no output) |
| command | headheight | `grep -rnwF -e 'headheight' crates/compiler/src/` | (no output) |
| command | headsep | `grep -rnwF -e 'headsep' crates/compiler/src/` | (no output) |
| command | hrulefill | `grep -rnwF -e 'hrulefill' crates/compiler/src/` | (no output) |
| command | ignorespacesafterend | `grep -rnwF -e 'ignorespacesafterend' crates/compiler/src/` | (no output) |
| command | includeonly | `grep -rnwF -e 'includeonly' crates/compiler/src/` | (no output) |
| command | indexspace | `grep -rnwF -e 'indexspace' crates/compiler/src/` | (no output) |
| command | intextsep | `grep -rnwF -e 'intextsep' crates/compiler/src/` | (no output) |
| command | labelitemii | `grep -rnwF -e 'labelitemii' crates/compiler/src/` | (no output) |
| command | labelitemiii | `grep -rnwF -e 'labelitemiii' crates/compiler/src/` | (no output) |
| command | labelitemiv | `grep -rnwF -e 'labelitemiv' crates/compiler/src/` | (no output) |
| command | lefteqn | `grep -rnwF -e 'lefteqn' crates/compiler/src/` | (no output) |
| command | leftmarginii | `grep -rnwF -e 'leftmarginii' crates/compiler/src/` | (no output) |
| command | leftmarginiii | `grep -rnwF -e 'leftmarginiii' crates/compiler/src/` | (no output) |
| command | leftmarginv | `grep -rnwF -e 'leftmarginv' crates/compiler/src/` | (no output) |
| command | leftmarginvi | `grep -rnwF -e 'leftmarginvi' crates/compiler/src/` | (no output) |
| command | lineskip | `grep -rnwF -e 'lineskip' crates/compiler/src/` | (no output) |
| command | lineskiplimit | `grep -rnwF -e 'lineskiplimit' crates/compiler/src/` | (no output) |
| command | linethickness | `grep -rnwF -e 'linethickness' crates/compiler/src/` | (no output) |
| command | makeglossary | `grep -rnwF -e 'makeglossary' crates/compiler/src/` | (no output) |
| command | makeindex | `grep -rnwF -e 'makeindex' crates/compiler/src/` | (no output) |
| command | marginparpush | `grep -rnwF -e 'marginparpush' crates/compiler/src/` | (no output) |
| command | marginparwidth | `grep -rnwF -e 'marginparwidth' crates/compiler/src/` | (no output) |
| command | mathversion | `grep -rnwF -e 'mathversion' crates/compiler/src/` | (no output) |
| command | medbreak | `grep -rnwF -e 'medbreak' crates/compiler/src/` | (no output) |
| command | multiput | `grep -rnwF -e 'multiput' crates/compiler/src/` | (no output) |
| command | newfont | `grep -rnwF -e 'newfont' crates/compiler/src/` | (no output) |
| command | newsavebox | `grep -rnwF -e 'newsavebox' crates/compiler/src/` | (no output) |
| command | newtie | `grep -rnwF -e 'newtie' crates/compiler/src/` | (no output) |
| command | newwrite | `grep -rnwF -e 'newwrite' crates/compiler/src/` | (no output) |
| command | nocorrlist | `grep -rnwF -e 'nocorrlist' crates/compiler/src/` | (no output) |
| command | nofiles | `grep -rnwF -e 'nofiles' crates/compiler/src/` | (no output) |
| command | nonfrenchspacing | `grep -rnwF -e 'nonfrenchspacing' crates/compiler/src/` | (no output) |
| command | normalmarginpar | `grep -rnwF -e 'normalmarginpar' crates/compiler/src/` | (no output) |
| command | normalsfcodes | `grep -rnwF -e 'normalsfcodes' crates/compiler/src/` | (no output) |
| command | obeycr | `grep -rnwF -e 'obeycr' crates/compiler/src/` | (no output) |
| command | oddsidemargin | `grep -rnwF -e 'oddsidemargin' crates/compiler/src/` | (no output) |
| command | oldstylenums | `grep -rnwF -e 'oldstylenums' crates/compiler/src/` | (no output) |
| command | onecolumn | `grep -rnwF -e 'onecolumn' crates/compiler/src/` | (no output) |
| command | openin | `grep -rnwF -e 'openin' crates/compiler/src/` | (no output) |
| command | openout | `grep -rnwF -e 'openout' crates/compiler/src/` | (no output) |
| command | oval | `grep -rnwF -e 'oval' crates/compiler/src/` | (no output) |
| command | paperheight | `grep -rnwF -e 'paperheight' crates/compiler/src/` | (no output) |
| command | paperwidth | `grep -rnwF -e 'paperwidth' crates/compiler/src/` | (no output) |
| command | pdfpageheight | `grep -rnwF -e 'pdfpageheight' crates/compiler/src/` | (no output) |
| command | pdfpagewidth | `grep -rnwF -e 'pdfpagewidth' crates/compiler/src/` | (no output) |
| command | poptabs | `grep -rnwF -e 'poptabs' crates/compiler/src/` | (no output) |
| command | prevdepth | `grep -rnwF -e 'prevdepth' crates/compiler/src/` | (no output) |
| command | qbezier | `grep -rnwF -e 'qbezier' crates/compiler/src/` | (no output) |
| command | restorecr | `grep -rnwF -e 'restorecr' crates/compiler/src/` | (no output) |
| command | reversemarginpar | `grep -rnwF -e 'reversemarginpar' crates/compiler/src/` | (no output) |
| command | savebox | `grep -rnwF -e 'savebox' crates/compiler/src/` | (no output) |
| command | shortstack | `grep -rnwF -e 'shortstack' crates/compiler/src/` | (no output) |
| command | sloppy | `grep -rnwF -e 'sloppy' crates/compiler/src/` | (no output) |
| command | smallbreak | `grep -rnwF -e 'smallbreak' crates/compiler/src/` | (no output) |
| command | spacefactor | `grep -rnwF -e 'spacefactor' crates/compiler/src/` | (no output) |
| command | subitem | `grep -rnwF -e 'subitem' crates/compiler/src/` | (no output) |
| command | subsubitem | `grep -rnwF -e 'subsubitem' crates/compiler/src/` | (no output) |
| command | suppressfloats | `grep -rnwF -e 'suppressfloats' crates/compiler/src/` | (no output) |
| command | textfloatsep | `grep -rnwF -e 'textfloatsep' crates/compiler/src/` | (no output) |
| command | textfraction | `grep -rnwF -e 'textfraction' crates/compiler/src/` | (no output) |
| command | textheight | `grep -rnwF -e 'textheight' crates/compiler/src/` | (no output) |
| command | thepage | `grep -rnwF -e 'thepage' crates/compiler/src/` | (no output) |
| command | thicklines | `grep -rnwF -e 'thicklines' crates/compiler/src/` | (no output) |
| command | thinlines | `grep -rnwF -e 'thinlines' crates/compiler/src/` | (no output) |
| command | topfraction | `grep -rnwF -e 'topfraction' crates/compiler/src/` | (no output) |
| command | topskip | `grep -rnwF -e 'topskip' crates/compiler/src/` | (no output) |
| command | typein | `grep -rnwF -e 'typein' crates/compiler/src/` | (no output) |
| command | typeout | `grep -rnwF -e 'typeout' crates/compiler/src/` | (no output) |
| command | unboldmath | `grep -rnwF -e 'unboldmath' crates/compiler/src/` | (no output) |
| command | unitlength | `grep -rnwF -e 'unitlength' crates/compiler/src/` | (no output) |
| command | usebox | `grep -rnwF -e 'usebox' crates/compiler/src/` | (no output) |
| command | usecounter | `grep -rnwF -e 'usecounter' crates/compiler/src/` | (no output) |
| command | wlog | `grep -rnwF -e 'wlog' crates/compiler/src/` | (no output) |
| environment | filecontents* | `grep -rnwF -e 'filecontents*' crates/compiler/src/` | (no output) |
| environment | theindex | `grep -rnwF -e 'theindex' crates/compiler/src/` | (no output) |

## Table 1B — names with matches but NO genuine implementation (61)

The word grep below returns hits, but every hit was read in context and is spurious: `//` comments, `vocabulary.rs` `KNOWN_UNIMPLEMENTED_*` entries (the compiler's own not-implemented list), SI-unit symbol tables (`siunitx.rs`), the `roman()` numeral table (`parser.rs:5775`), tabular column-alignment words, or unrelated Rust identifiers. The "genuine-impl check" column gives the discriminating grep (empty output) proving no dispatch arm, builtin-table entry, or inventory claim exists.

| kind | name | word grep (run from repo root) | genuine-impl check (empty output) and what the word hits are |
| ---- | ---- | ------------------------------ | ------------------------------------------------------------ |
| command | b | `grep -rnwF -e 'b' crates/compiler/src/` | `grep -rn -e '"b"' crates/compiler/src/parser.rs crates/compiler/src/text_builtins.rs crates/compiler/src/vocabulary.rs` empty exc. test-region lines; hits are Rust identifiers/test data |
| command | c | `grep -rnwF -e 'c' crates/compiler/src/` | `grep -rn -e '"c"' crates/compiler/src/parser.rs crates/compiler/src/text_builtins.rs` empty exc. `roman()` table (`parser.rs:5780`), SI `centi`, test data |
| command | d | `grep -rnwF -e 'd' crates/compiler/src/` | `grep -rn -e '"d"' crates/compiler/src/parser.rs crates/compiler/src/text_builtins.rs` empty exc. `roman()` table (`parser.rs:5778`), SI `deci`/`day`, identifiers |
| command | H | `grep -rnwF -e 'H' crates/compiler/src/` | `grep -rn -e '"H"' crates/compiler/src/parser.rs crates/compiler/src/text_builtins.rs` empty; hits are SI `henry` (`siunitx.rs:1243`), test strings, math-table comments |
| command | k | `grep -rnwF -e 'k' crates/compiler/src/` | `grep -rn -e '"k"' crates/compiler/src/parser.rs crates/compiler/src/text_builtins.rs` empty exc. test-region lines; hits are SI `kilo`, `ColorSpace::Cmyk`, identifiers |
| command | r | `grep -rnwF -e 'r' crates/compiler/src/` | `grep -rn -e '"r"' crates/compiler/src/parser.rs crates/compiler/src/text_builtins.rs` empty exc. test-region lines; hits are SI `ronto`, `LongtableAlign::Right`, identifiers |
| command | t | `grep -rnwF -e 't' crates/compiler/src/` | `grep -rn -e '"t"' crates/compiler/src/parser.rs crates/compiler/src/text_builtins.rs` empty exc. test-region lines; hits are SI `tonne`, `VerticalPosition::Top`, identifiers |
| command | u | `grep -rnwF -e 'u' crates/compiler/src/` | `grep -rn -e '"u"' crates/compiler/src/parser.rs crates/compiler/src/text_builtins.rs` empty; hits are `\u{…}` Rust escapes, `math.rs:4772` test comment |
| command | v | `grep -rnwF -e 'v' crates/compiler/src/` | `grep -rn -e '"v"' crates/compiler/src/parser.rs crates/compiler/src/text_builtins.rs` empty exc. `roman()` table (`parser.rs:5786`), test-region lines, identifiers |
| command | put | `grep -rnwF -e 'put' crates/compiler/src/` | `grep -rn -e '"put"' crates/compiler/src/parser.rs crates/compiler/src/math.rs crates/compiler/src/expansion.rs` empty; hits are a `text_builtins.rs` logo closure variable and `//` comments |
| command | hss | `grep -rnwF -e 'hss' crates/compiler/src/` | `grep -rn -e '"hss"' crates/compiler/src/parser.rs crates/compiler/src/math.rs` empty; hits are a `//` comment (`parser/lists.rs:15`) and `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:43`) |
| command | DeclareTextCommandDefault | `grep -rnwF -e 'DeclareTextCommandDefault' crates/compiler/src/` | `grep -rn -e 'DeclareTextCommandDefault' crates/compiler/src/ --include='*.rs' | grep -v '^.*://'` empty; sole hit is a `//` comment (`text_builtins.rs:129`) |
| command | addcontentsline | `grep -rnwF -e 'addcontentsline' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`layout.rs:88`) |
| command | bigskipamount | `grep -rnwF -e 'bigskipamount' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`parser.rs:887`) |
| command | boldmath | `grep -rnwF -e 'boldmath' crates/compiler/src/` | non-comment grep empty; hits are `//` comments (`text_builtins.rs:262,367`) |
| command | fontencoding | `grep -rnwF -e 'fontencoding' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`text_builtins.rs:150`) |
| command | fontseries | `grep -rnwF -e 'fontseries' crates/compiler/src/` | non-comment grep empty; hits are `//` comments (`parser.rs:3270,3339`) |
| command | labelenumi | `grep -rnwF -e 'labelenumi' crates/compiler/src/` | non-comment grep empty; sole hit is a `//!` doc comment (`parser/lists.rs:17`) |
| command | labelenumii | `grep -rnwF -e 'labelenumii' crates/compiler/src/` | non-comment grep empty; sole hit is a `//!` doc comment (`parser/lists.rs:17`) |
| command | labelenumiii | `grep -rnwF -e 'labelenumiii' crates/compiler/src/` | non-comment grep empty; sole hit is a `//!` doc comment (`parser/lists.rs:18`) |
| command | labelenumiv | `grep -rnwF -e 'labelenumiv' crates/compiler/src/` | non-comment grep empty; sole hit is a `//!` doc comment (`parser/lists.rs:18`) |
| command | labelitemi | `grep -rnwF -e 'labelitemi' crates/compiler/src/` | non-comment grep empty; sole hit is a `//!` doc comment (`parser/lists.rs:19`) |
| command | leftmargini | `grep -rnwF -e 'leftmargini' crates/compiler/src/` | non-comment grep empty; hits are `///` doc comments (`layout.rs:46,48`) |
| command | leftmarginiv | `grep -rnwF -e 'leftmarginiv' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`layout.rs:48`) |
| command | makelabel | `grep -rnwF -e 'makelabel' crates/compiler/src/` | non-comment grep empty; hits are `//`/`//!` comments (`layout.rs:1006`, `parser/lists.rs:15,22`) |
| command | medskipamount | `grep -rnwF -e 'medskipamount' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`parser.rs:886`) |
| command | nobreakspace | `grep -rnwF -e 'nobreakspace' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`parser.rs:6014`) |
| command | numberline | `grep -rnwF -e 'numberline' crates/compiler/src/` | non-comment grep empty; hits are `///` doc comments (`layout.rs:75,1030`) |
| command | raggedbottom | `grep -rnwF -e 'raggedbottom' crates/compiler/src/` | non-comment grep empty; sole hit is a `//!` doc comment (`layout/footnotes.rs:27`) |
| command | refname | `grep -rnwF -e 'refname' crates/compiler/src/` | non-comment grep empty; sole hit is a `//` comment (`parser.rs:2958`) |
| command | sbox | `grep -rnwF -e 'sbox' crates/compiler/src/` | non-comment grep empty; sole hit is a `//` comment (`text_builtins.rs:338`) |
| command | shipout | `grep -rnwF -e 'shipout' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`color.rs:865`); no engine entry either |
| command | smallskipamount | `grep -rnwF -e 'smallskipamount' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`parser.rs:886`) |
| command | vector | `grep -rnwF -e 'vector' crates/compiler/src/` | non-comment grep empty; sole hit is a `///` doc comment (`math.rs:3334`, English word "vector") |
| command | vtop | `grep -rnwF -e 'vtop' crates/compiler/src/` | non-comment grep empty; sole hit is a `//!` doc comment (`tabular.rs:27`); no engine entry either |
| command | appendix | `grep -rnwF -e 'appendix' crates/compiler/src/` | non-comment grep empty exc. `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:38`); no dispatch, no engine entry |
| command | enlargethispage | `grep -rnwF -e 'enlargethispage' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:46`); no dispatch, no engine entry |
| command | fbox | `grep -rnwF -e 'fbox' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:44`); no dispatch, no engine entry |
| command | fontfamily | `grep -rnwF -e 'fontfamily' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:55`); no dispatch, no engine entry |
| command | fontsize | `grep -rnwF -e 'fontsize' crates/compiler/src/` | hits are a `//` comment (`text_builtins.rs:339`) and `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:54`) |
| command | framebox | `grep -rnwF -e 'framebox' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:44`); no dispatch, no engine entry |
| command | hyphenation | `grep -rnwF -e 'hyphenation' crates/compiler/src/` | hits are `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:62`) and a `//` comment stating it is not done (`parser.rs:1560`) |
| command | listoffigures | `grep -rnwF -e 'listoffigures' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:39`); no dispatch, no engine entry |
| command | listoftables | `grep -rnwF -e 'listoftables' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:40`); no dispatch, no engine entry |
| command | makebox | `grep -rnwF -e 'makebox' crates/compiler/src/` | hits are `///` comments (`tabular.rs:235,249`) and `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:44`) |
| command | marginpar | `grep -rnwF -e 'marginpar' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:40`); no dispatch, no engine entry |
| command | parbox | `grep -rnwF -e 'parbox' crates/compiler/src/` | hits are a `///` comment (`tabular.rs:329`) and `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:44`); `tests/` hits are incidental corpus text, not `\parbox` tests |
| command | raisebox | `grep -rnwF -e 'raisebox' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:44`); no dispatch, no engine entry |
| command | RequirePackage | `grep -rnwF -e 'RequirePackage' crates/compiler/src/` | hits are `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:62`) and `//`/`///` comments (`math.rs:488,547,553`); no parser arm, no engine entry — `\RequirePackage{…}` falls through to `unsupported()` |
| command | PassOptionsToPackage | `grep -rnwF -e 'PassOptionsToPackage' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:63`); no dispatch, no engine entry |
| command | samepage | `grep -rnwF -e 'samepage' crates/compiler/src/` | only hit is the unimplemented-environment list (`vocabulary.rs:113`); no dispatch, no engine entry |
| command | selectfont | `grep -rnwF -e 'selectfont' crates/compiler/src/` | hits are a `//` comment (`text_builtins.rs:339`) and `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:54`) |
| command | subparagraph | `grep -rnwF -e 'subparagraph' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:38`); no dispatch, no engine entry |
| command | underbar | `grep -rnwF -e 'underbar' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:50`); no dispatch, no engine entry |
| command | usefont | `grep -rnwF -e 'usefont' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_COMMANDS` (`vocabulary.rs:55`); no dispatch, no engine entry |
| environment | abstract | `grep -rnwF -e 'abstract' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_ENVIRONMENTS` (`vocabulary.rs:110`); no dispatch, no engine entry |
| environment | eqnarray | `grep -rnwF -e 'eqnarray' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_ENVIRONMENTS` (`vocabulary.rs:111`); no dispatch, no engine entry |
| environment | filecontents | `grep -rnwF -e 'filecontents' crates/compiler/src/` | only hit is the unimplemented-environment list (`vocabulary.rs:113`); no dispatch, no engine entry |
| environment | picture | `grep -rnwF -e 'picture' crates/compiler/src/` | only hit is `KNOWN_UNIMPLEMENTED_ENVIRONMENTS` (`vocabulary.rs:111`); no dispatch, no engine entry |
| environment | sloppypar | `grep -rnwF -e 'sloppypar' crates/compiler/src/` | only hit is the unimplemented-environment list (`vocabulary.rs:113`); no dispatch, no engine entry |
| environment | tabbing | `grep -rnwF -e 'tabbing' crates/compiler/src/` | only hit is the unimplemented-environment list (`vocabulary.rs:114`); `tests/recovery.rs:68-69` and `tests/diagnostic_codes.rs:65-66` explicitly assert it is *not* implemented |

## Table 2 — implemented but with ZERO name matches in tests/ or inline test modules (41)

"Implemented" means a genuine dispatch arm, builtin-table entry, or `flashtex-tex-expansion` engine primitive/prelude macro (site noted per row). Both test greps below return no output for every row: `grep -rnwF -e 'NAME' crates/compiler/tests/` and the inline check (matches of the same word grep inside `#[cfg(test)] mod …` regions of `src/`). Caveat: `tests/supported_latex.rs::every_inventory_entry_compiles_without_an_unsupported_diagnostic` compiles every *inventory-listed* name generically (verified passing), so inventory-listed rows below do have one-context smoke coverage — the same coverage `\pagestyle` had when its preamble bug shipped. Name-specific tests (especially second-context tests: preamble vs body, math vs text) are what is missing.

| kind | name | implementation evidence | test greps (both empty) |
| ---- | ---- | ----------------------- | ----------------------- |
| command | AtBeginDocument | engine prelude `crates/tex-expansion/src/prelude.rs:94` + inventory `src/supported.rs:154` | `grep -rnwF -e 'AtBeginDocument' crates/compiler/tests/` ; inline `#[cfg(test)]` grep empty |
| command | AtEndDocument | engine prelude `crates/tex-expansion/src/prelude.rs:95` + inventory `src/supported.rs:155` | `grep -rnwF -e 'AtEndDocument' crates/compiler/tests/` ; inline empty |
| command | DeclareRobustCommand | engine `src/expand.rs` `"DeclareRobustCommand"` + inventory `src/supported.rs:139` | `grep -rnwF -e 'DeclareRobustCommand' crates/compiler/tests/` ; inline empty |
| command | addtocounter | engine `src/expand.rs` `"addtocounter"` + inventory `src/supported.rs:144` | `grep -rnwF -e 'addtocounter' crates/compiler/tests/` ; inline empty |
| command | addtolength | engine prelude `\def\addtolength` (`crates/tex-expansion/src/prelude.rs`) | `grep -rnwF -e 'addtolength' crates/compiler/tests/` ; inline empty |
| command | bibliographystyle | parser arm `src/parser.rs:1784` (diagnosed no-effect) + `src/supported.rs:309` | `grep -rnwF -e 'bibliographystyle' crates/compiler/tests/` ; inline empty |
| command | bigskip | parser arm `src/parser.rs:2008,2010` + `src/supported.rs:247` | `grep -rnwF -e 'bigskip' crates/compiler/tests/` ; inline empty |
| command | columnwidth | `DimenUnit::ColumnWidth` (`src/text_builtins.rs:529`) + dimen list (`src/parser/tabular.rs:159`) | `grep -rnwF -e 'columnwidth' crates/compiler/tests/` ; inline empty |
| command | displaystyle | math arm `src/math.rs:1223` + `src/supported.rs:637` | `grep -rnwF -e 'displaystyle' crates/compiler/tests/` ; inline empty |
| command | fboxrule | parser arm `src/parser.rs:2322` (`self.fboxrule_pt = pt`) | `grep -rnwF -e 'fboxrule' crates/compiler/tests/` ; inline empty |
| command | hsize | engine `src/expand.rs` `"hsize"` + dimen list (`src/parser/tabular.rs:159`) | `grep -rnwF -e 'hsize' crates/compiler/tests/` ; inline empty |
| command | ignorespaces | engine `Primitive::Ignorespaces` (`src/expand.rs:269`) + `src/supported.rs:158` | `grep -rnwF -e 'ignorespaces' crates/compiler/tests/` ; inline empty |
| command | jobname | engine `Primitive::Jobname` (`src/expand.rs:250`) + `src/supported.rs:159` | `grep -rnwF -e 'jobname' crates/compiler/tests/` ; inline empty |
| command | makeatother | engine prelude `\def\makeatother` + inventory `src/supported.rs:156` | `grep -rnwF -e 'makeatother' crates/compiler/tests/` ; inline empty |
| command | mathnormal | math arm `src/math.rs:1185` + `src/supported.rs:544` | `grep -rnwF -e 'mathnormal' crates/compiler/tests/` ; inline empty |
| command | mdseries | parser arm `src/parser.rs:513` + `src/supported.rs:214` | `grep -rnwF -e 'mdseries' crates/compiler/tests/` ; inline empty |
| command | medskip | parser arm `src/parser.rs:2008,2011` + `src/supported.rs:248` | `grep -rnwF -e 'medskip' crates/compiler/tests/` ; inline empty |
| command | newcounter | engine `src/expand.rs` `"newcounter"` + counter model (`src/xref.rs:204-209`) + `src/supported.rs:142` | `grep -rnwF -e 'newcounter' crates/compiler/tests/` ; inline empty |
| command | newlength | engine `src/expand.rs` `"newlength"` + `src/supported.rs:150` | `grep -rnwF -e 'newlength' crates/compiler/tests/` ; inline empty |
| command | noindent | parser arm `src/parser.rs:1985` (accepted no-op) + `src/supported.rs:254` | `grep -rnwF -e 'noindent' crates/compiler/tests/` ; inline empty |
| command | protected | engine prelude `\def\protected` + inventory `src/supported.rs:137` | `grep -rnwF -e 'protected' crates/compiler/tests/` ; inline empty |
| command | providecommand | engine `src/expand.rs` `"providecommand"` + `src/supported.rs:138` | `grep -rnwF -e 'providecommand' crates/compiler/tests/` ; inline empty |
| command | refstepcounter | `src/xref.rs:306` + `src/supported.rs:146` | `grep -rnwF -e 'refstepcounter' crates/compiler/tests/` ; inline empty |
| command | renewenvironment | engine `src/expand.rs` `"renewenvironment"` + `src/supported.rs:141` | `grep -rnwF -e 'renewenvironment' crates/compiler/tests/` ; inline empty |
| command | rmfamily | parser arm `src/parser.rs:522` + `src/supported.rs:220` | `grep -rnwF -e 'rmfamily' crates/compiler/tests/` ; inline empty |
| command | scriptscriptstyle | math arm `src/math.rs:1223` + `src/supported.rs:640` | `grep -rnwF -e 'scriptscriptstyle' crates/compiler/tests/` ; inline empty |
| command | scriptstyle | math arm `src/math.rs:1223` + `src/supported.rs:639` | `grep -rnwF -e 'scriptstyle' crates/compiler/tests/` ; inline empty |
| command | settodepth | engine `src/expand.rs` `"settodepth"` + `src/supported.rs:153` | `grep -rnwF -e 'settodepth' crates/compiler/tests/` ; inline empty |
| command | settoheight | engine `src/expand.rs` `"settoheight"` + `src/supported.rs:152` | `grep -rnwF -e 'settoheight' crates/compiler/tests/` ; inline empty |
| command | settowidth | engine `src/expand.rs` `"settowidth"` + `src/supported.rs:151` | `grep -rnwF -e 'settowidth' crates/compiler/tests/` ; inline empty |
| command | sffamily | parser arm `src/parser.rs:523` + `src/supported.rs:221` | `grep -rnwF -e 'sffamily' crates/compiler/tests/` ; inline empty |
| command | slshape | parser arm `src/parser.rs:514` + `src/supported.rs:216` | `grep -rnwF -e 'slshape' crates/compiler/tests/` ; inline empty |
| command | smallskip | parser arm `src/parser.rs:2008` + `src/supported.rs:249` | `grep -rnwF -e 'smallskip' crates/compiler/tests/` ; inline empty |
| command | stackrel | math arm `src/math.rs:1395` + `src/supported.rs:460` | `grep -rnwF -e 'stackrel' crates/compiler/tests/` ; inline empty |
| command | stepcounter | engine `src/expand.rs` `"stepcounter"` + `src/supported.rs:145` | `grep -rnwF -e 'stepcounter' crates/compiler/tests/` ; inline empty |
| command | textmd | parser arm `src/parser.rs:513` + `src/supported.rs:203` | `grep -rnwF -e 'textmd' crates/compiler/tests/` ; inline empty |
| command | textnormal | parser arm `src/parser.rs:524` + `src/supported.rs:212` | `grep -rnwF -e 'textnormal' crates/compiler/tests/` ; inline empty |
| command | textstyle | math arm `src/math.rs:1223` + `src/supported.rs:638` | `grep -rnwF -e 'textstyle' crates/compiler/tests/` ; inline empty |
| command | upshape | parser arm `src/parser.rs:519` + `src/supported.rs:218` | `grep -rnwF -e 'upshape' crates/compiler/tests/` ; inline empty |
| environment | flushright | `ParagraphStyle::FlushRight` (`src/parser.rs:6062`) + `src/supported.rs:724` | `grep -rnwF -e 'flushright' crates/compiler/tests/` ; inline empty |
| environment | quotation | `ParagraphStyle::Quote` (`src/parser.rs:6064`) + `src/supported.rs:726` | `grep -rnwF -e 'quotation' crates/compiler/tests/` ; inline empty |

## Triage notes for the supervisor

- Table 1A rows `closein`, `closeout`, `openin`, `openout`, `lineskip`, `lineskiplimit`, `topskip`, `pdfpageheight`, `pdfpagewidth` have entries in the engine (`crates/tex-expansion/src/expand.rs` primitive/register tables) despite zero `crates/compiler/src/` matches — reproduce with `grep -rn -e '"openin"' crates/tex-expansion/src/expand.rs`. They may be partially handled engine-side; the rest of Table 1A has no implementation anywhere I could find.
- `\day`, `\month`, `\year`, `\space` (kernel rows with `src/` matches) are engine-implemented (`\day` etc. are `Count` registers at `expand.rs:4998`, `\space` is prelude-defined), so they appear in neither table.
- `\linespread` is recognised-but-diagnosed (`KNOWN_ARITY_UNIMPLEMENTED`, `parser.rs:5557-5559`) and name-tested (`parser.rs:7310`); it sits between the two tables and is listed in neither.
- Highest-value follow-ups in the author's judgment: the 9 text accents in Table 1B (`\b \c \d \H \k \r \t \u \v` — every `\c{c}`-style input falls to `unsupported()`), `\RequirePackage` (real documents use it; currently falls to `unsupported()`), and Table 2's engine-only rows (invisible to `src/` grep, e.g. `\AtEndDocument` in the preamble — the exact `\pagestyle` shape).
