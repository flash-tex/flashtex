# Packages, fonts and the project manifest — direction and slices

**Status:** direction set by the owner on 2026-09-19; slices below are being
dispatched. Rules on #905 (read `.cls`/`.sty` files, not an allow-list).
**Owner of this document:** mac-claude-a (until the Commander reassigns it).

The owner's words, verbatim, are the requirement:

> Looking forward, there should be an elegant way for the following:
>
> * packaging/library management, as well as creating your own libraries. It
>   should be backwards compatible with standard LaTeX distributions but it
>   should have modern packaging/build features such as .toml config,
>   automatic package resolution (such as typst when it sees @preview), etc.
> * fonts. support using any font on the computer, including for math. have
>   the same flexibility as typst.
>   * having fonts as packages just makes no sense. it should still be
>     backwards compatible, but it should just have a "good" font system like
>     typst, where you can e.g. set font settings globally, locally, etc
> * obviously, we fully support custom .sty and .cls files.

## 1. What is true today (measured)

- `\usepackage{x}` and `\documentclass{x}` never open a file. The compiler
  keeps a hard-coded model of ~24 packages and 4 classes (`parser.rs`
  `use_package`, `document_class`); everything else is "recognised but not
  implemented" and its commands are `unknown_command` errors. A project-local
  `mystyle.sty` next to `main.tex` is ignored (probe: `\usepackage{mystyle}`
  defining `\hello` → `\hello is not supported by this compiler version`).
- The expansion engine is already in front of the parser
  (`crates/compiler/src/expansion.rs` over `crates/tex-expansion`): `\def`,
  `\newcommand`, `\newenvironment`, conditionals, registers, counters,
  `\csname`, keyval, `\@ifnextchar`, `\input` through a file reader. That is
  exactly the machinery a `.sty`/`.cls` needs; what is missing is (a) the
  resolver that finds the file and (b) the LaTeX package/class kernel macros
  (`\ProvidesPackage`, `\DeclareOption`, `\ProcessOptions`, `\LoadClass`,
  `\RequirePackage`, `\PassOptionsToPackage`, `\@ifpackageloaded`,
  `\AtEndOfPackage`, `\CurrentOption`, `\OptionNotUsed`, …).
- Fonts: text is Latin Modern / Computer Modern via TFM metrics and bundled
  OTF/Type 1 outlines (`crates/font-resources`, `crates/font-engine`); math
  is CM/LM math through `math-layout`. `fontspec` is vocabulary only (#920).
  There is no system-font discovery and no `\setmainfont`.
- #905 and #837 measured the cost: 60 % of a 163-document corpus opens with a
  class we cannot read; the median document loads 12 unsupported packages.

## 2. Decisions

1. **Read the files.** `\documentclass`, `\usepackage`, `\RequirePackage`,
   `\LoadClass`, `\RequirePackageWithOptions`, `\LoadClassWithOptions` resolve
   a real `.cls`/`.sty` and execute it through the expansion engine. The
   built-in package models stay as **fast paths and typesetting semantics**
   (a `.sty` can define macros; it cannot teach the layout engine to draw a
   `tcolorbox`). Resolution order: the file the engine already models with
   typesetting support → project files → manifest `texinputs` → the package
   cache (§4) → "not found" diagnostic naming what was searched. A built-in
   model wins only while it is *complete* for that package; a project-local
   file of the same name always wins (that is how LaTeX behaves too).
2. **A project manifest, `flashtex.toml`**, optional. Absent manifest = the
   directory of the entry file, exactly like today. Present, it names the
   entry, extra search paths, the packages the project depends on (with
   versions/sources) and the output settings. It never replaces the
   preamble: `\usepackage` in the document is still the truth for what is
   loaded; the manifest says *where from*.
3. **Automatic package resolution** is a fetch of package *sources* into a
   per-user cache, triggered by an unresolved `\usepackage`, from a registry
   the user can point at (default: a CTAN mirror's `tex-archive`
   layout, later a FlashTeX index). Network access is explicit: the app asks
   once per project ("`\usepackage{siunitx}` is not in this project or the
   cache. Fetch from CTAN?") and remembers the answer in the manifest. Never
   in the engine crates — resolution lives in the project layer
   (`crates/project-files` / the app), and the engine only sees a resolved
   path. Offline and no-network builds keep working with what is cached.
4. **Fonts are a first-class setting, not a package.** The document (or the
   manifest, or the editor's Fonts sheet) sets `text`, `math`, `mono`,
   `sans` families — globally, or locally for a group/environment — from any
   font installed on the machine or shipped in the project. Backwards
   compatible: `\usepackage{lmodern}`, `fontenc`, `fontspec`'s
   `\setmainfont`/`\setmathfont` and `unicode-math` keep meaning what they
   mean, mapped onto the same setting. Math from OpenType MATH tables
   (Latin Modern Math, STIX Two Math, Libertinus Math, Fira Math, …), text
   from any OTF/TTF via the existing outline pipeline; TFM-metric fonts
   stay the default so pdflatex parity holds when nothing is set.

## 3. Slices (dispatchable, each verifiable on its own)

### S1 — read project-local `.sty` and `.cls` (engine-engineer, Opus)

- Resolver in `crates/compiler`: for `\usepackage{name}` / `\documentclass
  {name}` / `\RequirePackage` / `\LoadClass`, look for `name.sty`/`name.cls`
  next to the entry document, then in `texinputs` from the manifest (S2),
  via the same `SourceDocument` set the app already hands the compiler
  (the app must include `.sty`/`.cls` project files in that set).
- LaTeX kernel macros for packages/classes in `crates/tex-expansion`'s
  LaTeX layer: `\ProvidesPackage/Class/File`, `\NeedsTeXFormat`,
  `\DeclareOption`, `\DeclareOption*`, `\ProcessOptions(\relax|*)`,
  `\ExecuteOptions`, `\PassOptionsToPackage/Class`, `\CurrentOption`,
  `\OptionNotUsed`, `\@ifpackageloaded`, `\@ifclassloaded`,
  `\@ifpackagewith`, `\AtEndOfPackage/Class`, `\RequirePackage[opts]{}`,
  `\LoadClass[opts]{}`, `\@ifclasswith`, `\PackageWarning/Error/Info`,
  `\ClassWarning/Error/Info`, `\@onlypreamble`, `\@currname/@currext`,
  `\makeatletter` state across files (catcode of `@` is 11 inside a
  `.sty`/`.cls`, restored after). `\endinput`.
- A `.cls` that `\LoadClass{article}` (or report/book/beamer) gets the base
  class's geometry and page model from `class-geometry`; its own
  `\setlength`s apply on top. A class with no `\LoadClass` (a from-scratch
  thesis class) gets `article`'s model plus a diagnostic saying so.
- Built-in models keep winning for packages the typesetter implements
  (a `.sty` on disk for `amsmath` is *not* executed — the model is complete
  and the real file needs primitives we do not have). The set is explicit
  (`BUILT_IN_PACKAGES` with a `complete: bool`) and printed in the
  supported-LaTeX inventory.
- Acceptance: the `mystyle.sty` probe above renders `Hello from mystyle,
  **world**.`; a `myclass.cls` with `\LoadClass[11pt]{article}` +
  `\newcommand` + `\RequirePackage{mystyle}` renders with 11pt geometry;
  Jake's Resume's `resume.cls`-free `article` path keeps working; the
  corpus in #905 re-measured (how many documents now *open*).

### S2 — `flashtex.toml` manifest (Fable / general, app + project layer)

**Status (2026-09-19): shipped on `lane/manifest`** — `crates/project-manifest`
(parser, warnings, `locate`, template), the CLI (`build|check|watch` on a
directory, `texinputs` in the closure, `output`, `manifest init|show`),
`crates/project-files` (`Package`/`Class` kinds, `texinput_files`, the
helper's `manifest` operation) and the Mac app (open a folder, package rows,
package inputs in the compile request, Create flashtex.toml…, TOML
colouring). User doc: [docs/user/project-manifest.md](../user/project-manifest.md).
Delivery to S1: the `.sty`/`.cls` files are in the compiler's document set
by project-relative path, after the entry closure — the entry's own
directory first, then each `texinputs` directory in manifest order (an
outside directory at `texinputs/<i>/<file>`); no manifest → the entry's
own `.sty`/`.cls` files still arrive. `[fonts]`/`[packages]` are parsed
and shown; their effect is S4/S3.

```toml
[project]
entry = "main.tex"          # default: the file the user opened / the only .tex
texinputs = ["styles", "../shared-macros"]   # searched after the project dir
output = "build"            # where PDFs/aux go (default: .flashtex/)

[fonts]                     # §4 — optional, overrides nothing the document sets
text = "Libertinus Serif"
math = "Libertinus Math"
mono = "JetBrains Mono"

[packages]                  # §2.3 — resolution policy and pins
source = "ctan"             # or a URL / "none" (never fetch)
fetch = "ask"               # ask | always | never
pin = { siunitx = "3.3.24", tikz = "3.1.10" }
```

- Parsed by a small platform-free crate (`crates/project-manifest`) used by
  the CLI, the Mac app and the project-files helper. Unknown keys warn,
  never fail. `texinputs` feeds S1's resolver. The editor gets a "Create
  flashtex.toml" command that writes the defaults with comments.
- Acceptance: CLI `flashtex build` honours `entry`/`texinputs`; the app's
  project index includes `.sty`/`.cls` from `texinputs`; a missing manifest
  changes nothing (every existing fixture builds byte-identically).

### S3 — package resolution and cache (after S1 + S2)

**Status (2026-09-19): shipped on `lane/resolver`** — `crates/package-resolver`
(libraries → cache → source; `ask` describes, `resolve_with_consent`
fetches; CTAN via the JSON API record + the mirror listing; `.dtx`-only is
`NotAvailable("needs docstrip")`, docstrip itself is not implemented; a
registry URL is the same layout with a content-hash version; git URLs are
out of scope), the CLI (`--fetch`, `--write-pins`, `packages
list|fetch|clear`; nothing runs without a manifest or `--fetch`), the
project-files helper (`resolve_packages`, `set_packages`) and the Mac app
(one consent sheet per project, Fetch / Not Now / Never, *Remember* writes
`fetch = "always"`; resolved files under a Packages sidebar group on the
direct route). Docs: [docs/user/project-manifest.md#packages](../user/project-manifest.md#packages),
[crates/package-resolver/README.md](../../crates/package-resolver/README.md).
Delivery to S1: the files arrive as documents at `packages/<name>/<file>`
after the `texinputs` files.

- `crates/package-resolver` (project layer, may use the network): given an
  unresolved name, consult the cache `~/Library/Application Support/FlashTeX
  /packages/<name>/<version>/` (Linux: `$XDG_CACHE_HOME/flashtex`), then
  the registry named in the manifest. Fetch the package's `.sty`/`.cls`/
  `.def`/`.clo` files (CTAN `tex-archive/macros/latex/contrib/<name>/` or the
  TeX Live `texmf-dist` mirror layout), record the version in the manifest
  `pin` table, hand paths to S1. Consent gate in the app and the CLI
  (`--fetch=ask|always|never`). No execution of anything but TeX source;
  `.ins`/`.dtx` packages are unpacked with the engine's own `docstrip`
  subset or skipped with a diagnostic.
- "Creating your own libraries": a directory with a `flashtex.toml`
  `[library] name = "..."` and `.sty`/`.cls` files can be referenced from
  another project's `[packages] path = { mylib = "../mylib" }` or a git URL;
  same cache, same resolver.

### S4 — fonts (design lane first, then implementation; touches Daniel's crates)

- `crates/font-discovery` (platform dirs read from a small OS-specific list
  plus `FLASHTEX_FONT_DIRS`; the engine crates stay platform-free): scan
  OTF/TTF/TTC, index by family/style/weight, read the `MATH` table
  presence. The Mac app's Fonts sheet lists families with a live sample.
- Document-side API: `fontspec`-shaped commands are the compatible surface
  (`\setmainfont`, `\setsansfont`, `\setmonofont`, `\setmathfont`,
  `\newfontfamily`, `\fontspec{...}` locally) — real documents already write
  them (37/163 in #905). Unknown fonts fall back with one diagnostic naming
  what was searched. `unicode-math` → the same `math` setting.
- Text: `render-pipeline`'s `FontSet` resolves the family to the discovered
  file; metrics come from the OTF `hmtx`/`kern`/`GPOS` (font-engine already
  shapes OTF Latin Modern). Math: `math-layout` reads the OpenType `MATH`
  table (constants, variants, assembly) — this is the large piece and it is
  Daniel's crate; the design lane writes the contract, Daniel's side
  implements or delegates.
- Acceptance: `\setmainfont{Libertinus Serif}` sets a paragraph in that
  font with correct advances; `\setmathfont{Latin Modern Math}` reproduces
  the CM-math layout of `hw1` within tolerance (same glyph metrics, MATH
  constants); nothing changes for documents that set no font.

## 4. What this does *not* change

- pdflatex parity for the corpus is still the oracle for everything the
  engine models. Reading a `.sty` does not make an unmodelled typesetting
  command work; it makes the *macros* in it work and lets the engine say
  precisely which primitive it lacks.
- The engine crates stay free of the network and of platform paths.
- The four-classes-plus-beamer page models remain the geometry source.

## 5. Ownership and coordination

- S1: mac-claude-a lane (compiler + tex-expansion). Daniel's #950/#946
  touch `tex-expansion` too — rebase on their landing, not around them.
- S2: mac-claude-a lane (new crate + CLI + Mac app).
- S3: after S1/S2; needs a consent UI decision from the owner (default
  `ask`).
- S4: design lane → contract in `crates/math-layout/CONTRACT.md` style →
  Daniel's queue for the MATH-table half.
- Tracking: #905 (ruling recorded there), this document, and the #2 log.
