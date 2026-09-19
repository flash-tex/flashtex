# Package and class files in the editor

Where the project's `.sty`/`.cls`/`.def`/`.clo` files enter editor
intelligence (lane pkg-editor), and what the engine does not provide yet.
User-facing behaviour is in [docs/user/gui.md › Packages and classes](../../../docs/user/gui.md#packages-and-classes).

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
| "declared in mystyle.sty" rows and `\begin{` names | `ShellModel.packageDocumentsForEditor()` → `Request.packageDocuments` → `Completion.packageDeclarations(in:)` (off-main, only for a command or environment token) | texts of 1–3 |
| Go to Definition / hover peek | `ShellModel.packageDefinition(ofCommand:environment:)` after `definition(ofCommand:)` | texts of 2–3 (open members are found first, the ordinary way) |
| Problems row → package file | `ShellModel.goToOccurrence` → `openPackageInput(at:)` | paths of 2–3 |
| `\usepackage` line hint | `EditorDiagnostics.packageHints` from the compiler's secondary labels | the compile result only |

Opening a virtual input goes through `ProjectDocuments.openVirtual`
(`Origin.virtual(source:)`): a member with no file, never saved
(`saveDocument` and `changeRefusal` refuse it), the text view
non-editable (`SourceEditorView.editable`), the `ReadOnlyBanner` above the
editor. The compile request carries it once, as the open member.

## What the engine should expose next

Everything above works from the package inputs' *text* with the same
lexical scans the open documents use (`Completion.declarations`,
`EditorNavigation.definitions`). It is exact enough for `\newcommand`-style
definitions and wrong in the ways a lexical scan is wrong: a macro defined
through `\csname`, inside a conditional, or by a package the package loads
is not seen; `\let` copies are found by name only. For it to be exact, the
compiler (S1, `crates/compiler/src/packages.rs` / `tex-expansion`) should
put on the wire:

- per package input, the definitions it actually made while loading the
  file — name, kind (macro / environment / switch), the parameter shape it
  parsed, and the definition's byte span in that file (`documents` already
  carry the same paths, so `RuntimeV1.SourceRange` fits);
- for each definition, the load site (`\usepackage`/`\RequirePackage`
  span) so the hint at the loading line can name what a package brought
  in, not only how many problems it has;
- the `loaded here` label already emitted (`with_label`) on every
  diagnostic raised inside a package, including ones raised by a package
  the package loaded (today the label names the direct loader only).

With those, `packageDeclarations` and `packageDefinition` become lookups
into the result instead of scans, and a definition's span is the
compiler's, not a guess from the source text.
