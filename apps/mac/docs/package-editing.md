# Package and class files in the editor

Where the project's `.sty`/`.cls`/`.def`/`.clo` files enter editor
intelligence (lane pkg-editor): the engine's `metadata.packages` first, the
package inputs' text as the fallback.
User-facing behaviour is in [docs/user/gui.md › Packages and classes](../../../docs/user/gui.md#packages-and-classes).

## What the engine puts on the wire

Every `compile_result` for a project whose document loads a project
`.sty`/`.cls` carries `payload.metadata.packages`
([runtime-v1 › Optional `metadata` object](../../../docs/contracts/runtime-v1.md#optional-metadata-object)):
one record per file the expansion pass read, with its `\ProvidesPackage`
(name, date, version, description), the `\usepackage`/`\RequirePackage`
that loaded it, its `\DeclareOption`s and every definition it made at its
outermost level -- name, kind (`macro`, `environment`, `conditional`,
`counter`, `length`, `register`, `theorem`, `math_operator`), the defining
command, the parameter shape (`arity`, `optional_default`, `signature`)
and the byte span of the whole defining statement in that file. Both
producers write it: the compiler's own `compile_result` and
`flashtex-render` (`crates/render-pipeline`, `v1::metadata_json`), the
worker the app actually runs. The object is absent, never null, for a
project without package files.

The app decodes it as `RuntimeV1.CompileResult.metadata`
(`RuntimeV1.Metadata`, `RuntimeV1.PackageRecord`; `FastJSON` reads the
section on the hot path, it is not skipped as an unknown key) and
`Completion.Metadata.from(_:)` carries the records as `packages`.

- **Completion rows** -- `CompletionScheduler.Request.packageRecords`
  (the last `compileResult`'s records, whatever its revision: names and
  shapes do not move with document edits) reach
  `Completion.packageDeclarations(in:records:)`, which builds the
  "declared in mystyle.sty" rows from the records instead of scanning: a
  `macro`, `conditional`, `math_operator`, `length` or `register` is a
  command, an `environment` or `theorem` an environment, a `counter` names
  no control sequence and is skipped; the snippet's mandatory count is the
  engine's `arity` less the optional first parameter (`optional_default`,
  or an xparse `o`/`O{…}` first argument); the definer is the engine's.
  When the file's `\ProvidesPackage` has a description, the row's detail
  is `declared in mystyle.sty — <description>`.
- **Go to Definition / hover peek** -- `ShellModel.packageDefinition(ofCommand:environment:)`
  looks the name up in `result.metadata.packages` first and returns the
  engine's definition: `via` is the definer, the range is the statement's
  byte span converted to UTF-16 against the input's text, the line is
  counted from the same bytes. So a macro made through `\csname` or inside
  a conditional is found too. The offsets are trusted only while the
  input's text is the text the compiler read (`compiledDocuments`).
- The `\usepackage` line hint (`mystyle.sty: N problems`) is unchanged: it
  is built from the diagnostics' secondary labels, not from the records.

## The fallback: the package inputs' text

Without records -- no result yet, an older producer, a bare text view, a
name the engine did not record, a package input edited since the compile --
everything below works from the package inputs' *text* with the same
lexical scans the open documents use (`Completion.declarations`,
`EditorNavigation.definitions`). That is exact enough for
`\newcommand`-style definitions and wrong in the ways a lexical scan is
wrong (a macro defined through `\csname`, inside a conditional, or by a
package the package loads is not seen; `\let` copies are found by name
only), which is why the records come first.

## The three sources of package inputs

`ShellModel.packageInputs` (`ShellModel+PackageNavigation.swift`) is the
one list every editor feature reads, in the compile request's order:

1. **Open members** with a package extension (`ProjectManifest.isPackagePath`)
   — ordinary `ShellModel.documents`; excluded from `packageInputs` itself
   because they are already open, but `packageDocumentsForEditor()` adds
   the ones other than the active buffer.
2. **Manifest package inputs** — `ProjectManifest.packageInputs()`, the
   helper's `manifest` reply: the root's own `.sty`/`.cls` files and every
   file under a `[project] texinputs` directory, with their text. A row
   whose `origin` is set is a virtual `texinputs/<i>/…` mount of a
   directory outside the root; the `PackageInput.virtualSource` names the
   real file.
3. **Resolved packages** — `ProjectPackagesState.documents()`, delivered by
   `resolve_packages` at `packages/<name>/<file>`; always virtual, the
   source is the library or cache row (`ProjectManifest.Row.source`).

Nothing here re-reads TOML or walks directories: the helper reported the
texts, the compiler resolves `\usepackage` against the same paths.

## Where each feature reads them

| Feature | Entry point | What it reads |
|---|---|---|
| Colouring, `@` as a letter | `ShellModel.editorLanguage` → `SyntaxHighlighter.Language.package`; `\makeatletter` flips `atLetters` per line in any buffer | the active buffer only |
| Kernel vocabulary first, `@` in the typed token | `SourceEditorView.Coordinator.packageContext(at:in:)` → `CompletionScheduler.Request.packageMode` / `.atLetter` | the syntax model at the caret |
| `\usepackage{` / `\documentclass{` names | `ShellModel.projectPackageFiles` → `Request.projectPackageFiles` | paths of 1–3 |
| "declared in mystyle.sty" rows and `\begin{` names | `ShellModel.packageDocumentsForEditor()` → `Request.packageDocuments` → `Completion.packageDeclarations(in:records:)` (off-main, only for a command or environment token) | `metadata.packages` when the result has them, else texts of 1–3 |
| Go to Definition / hover peek | `ShellModel.packageDefinition(ofCommand:environment:)` after `definition(ofCommand:)` | `metadata.packages` (spans into the input's text) when the result has the name, else texts of 2–3 (open members are found first, the ordinary way) |
| Problems row → package file | `ShellModel.goToOccurrence` → `openPackageInput(at:)` | paths of 2–3 |
| `\usepackage` line hint | `EditorDiagnostics.packageHints` from the compiler's secondary labels | the compile result only |

Opening a virtual input goes through `ProjectDocuments.openVirtual`
(`Origin.virtual(source:)`): a member with no file, never saved
(`saveDocument` and `changeRefusal` refuse it), the text view
non-editable (`SourceEditorView.editable`), the `ReadOnlyBanner` above the
editor. The compile request carries it once, as the open member.

## What the engine could expose next

The records carry the load site (`loaded_by`) and the file's
`\ProvidesPackage`, which the editor does not use yet:

- the hint at the `\usepackage` line could name what the package brought
  in (`mystyle.sty: 4 macros, 2 environments`), not only how many
  problems it has;
- the `loaded here` label on a diagnostic raised inside a package names
  the direct loader; the records' `loaded_by` chain gives the whole
  nesting for a package loaded by a package.
- a definition's `overrides` (the name had a meaning before) could mark
  a `\renewcommand` of a kernel command in the package buffer.
