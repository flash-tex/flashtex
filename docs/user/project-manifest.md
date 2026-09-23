# The project manifest: `flashtex.toml`

**Status:** shipped with slice S2 of
[packages, fonts and the project manifest](../proposals/packages-fonts-manifest.md)
(2026-09-19). Owner: mac-claude-a. `[fonts]` applies (slice S4, the same
day): the CLI, the app's direct route and the preview-controller route all
send it with every compile. `[packages]` applies (slice S3, the same day):
local libraries, the per-user package cache and consent-gated fetches from
CTAN — see [Packages](#packages).

A FlashTeX project is a folder with an entry document. That is all the
engine needs, and **without a manifest nothing described here happens**:
`flashtex build main.tex` and *File › Open* behave exactly as they always
have — the entry's folder is the project root, `\input`/`\include` resolve
under it, the PDF lands next to the entry. The manifest is for what a folder
cannot say by itself: which file is the entry when you open the folder,
extra directories of `.sty`/`.cls` files to search, where output goes, the
project's fonts, and the package policy.

The file is `flashtex.toml`, at the project root. Every key is optional.
Create one with `flashtex manifest init` or *File › Create flashtex.toml…*
in the app; both write this template, with your actual entry filled in:

```toml
# flashtex.toml — the FlashTeX project manifest (docs/user/project-manifest.md).
# Every key is optional. Without this file FlashTeX behaves exactly as the
# defaults below describe; a key it does not know is a warning, never an error.

[project]
entry = "main.tex"   # the document to compile (default: the file you open)
texinputs = []    # extra directories searched after the project directory for .sty/.cls/.tex/.bib/.def/.clo, e.g. ["styles", "../shared-macros"]
# output = "build"  # where the PDF and auxiliary files go, relative to this file (default: next to the entry)

[fonts]   # font families by role; overrides nothing the document itself sets (\setmainfont, \usepackage{lmodern}, …)
# text = "Latin Modern Roman"
# math = "Latin Modern Math"
# mono = "Latin Modern Mono"
# sans = "Latin Modern Sans"

[packages]
source = "ctan"   # where a package that is not in the project or the cache may be fetched from: "ctan", "none" (never fetch) or a registry URL
fetch = "ask"     # when \usepackage names such a package: "ask" (once per project), "always" or "never"
pin = {}          # exact versions, e.g. { siunitx = "3.3.24" }
path = {}         # local libraries, e.g. { mylib = "../mylib" } — a directory with its own [library] manifest

# [library]        # only when this directory *is* a library other projects reference
# name = "mylib"
```

## Where FlashTeX looks for it

Starting at the entry document's folder, FlashTeX looks for `flashtex.toml`
there, then in each parent folder, and stops at the first folder that
contains a `.git` entry (a repository is the outermost plausible project) or
at the filesystem root. The folder the manifest is found in is the
**project root** for the CLI: an entry in `paper/main.tex` still sees
`styles/` beside the manifest. A manifest above a `.git` boundary — a stray
`~/flashtex.toml` — never governs a checkout beneath it.

## Keys

| Key | Meaning | Default |
|---|---|---|
| `[project] entry` | The document to compile, relative to the manifest's folder. Used when you give FlashTeX a folder (`flashtex build .`, *Open… › a folder*); naming a `.tex` file always wins. | the file you open; for a folder without one, its only `.tex` file (two or none is an error, never a guess) |
| `[project] texinputs` | Directories searched after the project folder, in order, relative to the manifest's folder. Every `.sty`, `.cls`, `.tex`, `.bib`, `.def` and `.clo` file **directly** in each (no recursion, like `TEXINPUTS` without `//`) joins the document set the engine sees, after the entry's own include closure. A directory under the root keeps its real paths (`styles/mystyle.sty`); one you explicitly place outside it (`../shared-macros`) is presented at the stable virtual path `texinputs/<index>/<file>`, `<index>` being its position in this list. Absolute paths, `~`, backslashes and colons, and `"."` are ignored with a warning. | `[]` |
| `[project] output` | Where the PDF (and, later, auxiliary files) go, relative to the manifest's folder; created on demand. `-o` on the command line still wins. | next to the entry (`<entry>.pdf`) |
| `[fonts] text`, `math`, `mono`, `sans` | The installed font family each role defaults to — `text` is the body (`\rmdefault`), `sans` is `\textsf`/`\sffamily`, `mono` is `\texttt`/`\ttfamily`, `math` is the OpenType math font (a family with a `MATH` table — `flashtex-render --list-math-fonts` names the installed ones; Latin Modern Math is bundled) that lays every formula out with LuaTeX's OpenType rules; the document's `\setmathfont{…}` outranks it, `\usepackage{unicode-math}` alone means Latin Modern Math, and a family without a `MATH` table is a `math_font_unavailable` warning with TeX's metrics kept. Names are matched the way `\setmainfont{…}` matches them: case-insensitively, by family, full or PostScript name, over the fonts installed on the machine, `FLASHTEX_FONT_DIRS`, and the project's own `fonts/` folder. Precedence, highest first: a local `\fontspec` group, the document's `\setmainfont`/`\setsansfont`/`\setmonofont`, this table, the class default. An unknown family is a `font_family_unavailable` warning and the class font. Set it from *File › Project Fonts…* in the app or by hand. | unset (Latin Modern / Computer Modern) |
| `[packages] source` | Where a package that is neither in the project, a local library nor the cache may be fetched from: `"ctan"` (the CTAN JSON API and `mirrors.ctan.org`), `"none"` (never fetch, whatever `fetch` says) or a registry URL serving the CTAN archive layout (`<url>/macros/latex/contrib/<name>/`). See [Packages](#packages). | `"ctan"` |
| `[packages] fetch` | What happens when `\usepackage` names such a package: `"ask"` — the app shows one consent sheet per project, the CLI prompts once per package on a terminal and fetches nothing otherwise; `"always"` — fetch without asking (what the sheet writes when you tick *Remember*); `"never"` — use only what is cached (what *Never for this project* writes). | `"ask"` |
| `[packages] pin` | Exact versions, `{ siunitx = "3.3.24" }`: the cached copy of that version is used, and a fetch that would bring another version is refused naming both (CTAN keeps only the current version). Written by `flashtex build --write-pins`, never by an ordinary build. For a registry URL the version is the content hash the fetch reported. | `{}` |
| `[packages] path` | Local libraries, `{ mylib = "../mylib" }`: a directory (relative to this file, `..` allowed, no absolute path or `~`) whose own `flashtex.toml` says `[library] name = "mylib"`. Its `.sty`/`.cls`/`.def`/`.clo` files resolve before the cache and the network, at `packages/mylib/<file>`; a directory or file reached through a symbolic link is refused with a diagnostic. Git URLs are not supported: a library is a directory. | `{}` |
| `[library] name` | Only for a directory that *is* a library other projects reference; it must repeat the key those projects use in `path`. | absent |

Whatever the manifest says that this version of FlashTeX does not understand
— an unknown key or section, a known key of the wrong type, an unexpected
value for `source`/`fetch` — is a **warning** naming the key
(`fonts.serif: unknown key; ignored`), never an error: a manifest written
for a newer FlashTeX still builds on an older one. Only a file that is not
valid TOML stops the build, because nothing past a syntax error can be
trusted.

## Packages

`\usepackage{x}` is still the truth for what is loaded; `[packages]` says
*where from*. A package the compiler models is never resolved (its built-in
model wins); one the project supplies (next to the entry, in `texinputs`)
is read from there. For everything else, FlashTeX looks, in order:

1. in the **local libraries** of `[packages] path`;
2. in the **per-user package cache** — `~/Library/Application
   Support/FlashTeX/packages` on macOS, `${XDG_CACHE_HOME:-~/.cache}/flashtex/packages`
   on Linux, `%LOCALAPPDATA%\FlashTeX\packages` on Windows, or
   `FLASHTEX_PACKAGE_CACHE` — laid out as `<name>/<version>/<files>` plus a
   `manifest.json` recording the source URL, the fetch time and each file's
   SHA-256 (a copy that no longer matches is refused);
3. at the **source**, only as `fetch` allows: for CTAN, the package's
   record from `https://ctan.org/json/2.0/pkg/<name>` (its version and
   archive path) and the directory listing at
   `https://mirrors.ctan.org<path>/`, from which only the `.sty`, `.cls`,
   `.def`, `.clo` and `.cfg` files are fetched. Asking costs those two
   metadata requests; no package file moves before you say yes.

Resolved files reach the engine as documents at `packages/<name>/<file>`,
after the entry's closure and the `texinputs` files, so a project file of
the same name always wins. A package that ships only `.dtx`/`.ins` sources
(`lipsum`, `microtype`, `siunitx` — most of CTAN) has them fetched too and
unpacked with FlashTeX's own docstrip (an interpreter of the `.ins`
language, not a TeX run, so nothing fetched is executed); the generated
`.sty`/`.cls`/`.def`/`.cfg` are what gets cached, each marked with the
batch file and sources it came from, and `flashtex packages fetch` prints
them. A package whose `.dtx` installs itself without a `.ins`, or whose
batch file needs more TeX than docstrip's commands, is reported as *needs
docstrip* with the reason; run `tex name.ins` yourself and put the `.sty`
in the project. Offline, everything cached or in a library keeps working.
`flashtex packages list|fetch <name>|clear` inspects, fills and empties
the cache.

## What each tool does with it

**CLI.** `flashtex build`, `check` and `watch` take a `.tex` file, a
folder, or nothing (the current folder). With a manifest, `texinputs` files
appear in the `documents` list of `--json` and the `--v2` envelope after
the include closure; warnings are `manifest_unknown_key` /
`manifest_texinputs` diagnostics attributed to `flashtex.toml`; the report
carries `manifest` (the path, or `null`). `watch` also polls the manifest
and every `texinputs` file. `[fonts]` reaches the render; `--font
ROLE=NAME` (repeatable, roles `text`, `sans`, `mono`, `math`) outranks the
table for that role. With a manifest (or `--fetch`), the packages the
documents ask for that nothing supplies are resolved as [Packages](#packages)
says — `--fetch ask|always|never` outranks `fetch`; `ask` prompts once per
package on a terminal (`\usepackage{siunitx} is not in this project or the
cache. Fetch siunitx 3.3.24 from CTAN (https://…)? [y/N]`) and, when
stdin is not a terminal, fetches nothing and says so in a `package_fetch`
diagnostic; a package that cannot be resolved is a `package_unavailable`
warning naming the reason; the resolved files appear in `documents` as
`packages/<name>/<file>`. `build --write-pins` records each fetched version
in `[packages] pin` (a plain build never rewrites the manifest). Without a
manifest and without `--fetch` nothing is resolved. `flashtex manifest init [<entry>|<dir>]
[--force]` writes the template (refusing to overwrite without `--force`);
`flashtex manifest show [<entry>|<dir>] [--json]` prints the manifest that
governs the entry — the defaults when there is none — with each `texinputs`
entry classified (`inside`, `outside`, `invalid`) and the warnings. See
[Command-line tools](compiler.md#manifest).

**The Mac app.** *File › Open LaTeX File…* (⌘O) accepts
a folder: the entry is `[project] entry`, else the folder's only `.tex`
file. The `.sty`/`.cls`/`.def`/`.clo` files next to the entry and every file
under `texinputs` show in the sidebar's Project tree as dimmed rows with the
class/style icon and go out with every compile (direct route) so the engine
can read them; click one to open it as a project member. The app's project
root is the entry's folder, so a `texinputs` directory beside a manifest
*above* the entry (`proj/flashtex.toml`, `proj/styles/`, entry
`proj/paper/main.tex`) is read like an outside directory — compiled, listed,
not openable in place. *File › Create flashtex.toml…* writes the template
next to the entry and opens it; `.toml` buffers are coloured as plain text
with comments. *File › Project Fonts…* (also in the command palette) edits
the `[fonts]` table as four rows — Text, Math, Sans, Mono — each a
searchable picker over the families the engine's own index finds, with a
sample line in that family and *Class default* first; Apply writes the
table (creating the file from the template when there is none, and writing
nothing for *Class default* everywhere without one) and the preview
recompiles. Every compile request carries the table as `fonts`, on the
direct route and through the preview controller alike. The manifest is
read — and its `[fonts]` table rewritten — by the project-files helper
(one parser and one writer for the CLI, the helper and the app; the rest
of the file is kept byte for byte); without the helper a folder still opens
through its only `.tex` file and the status bar says the manifest was not
read. After each compile, the packages the compiler reports it could not
find are resolved through the same helper: what a local library or the
cache has joins the next compile at once; what would have to be fetched is
**one consent sheet per project** listing each package, its version, its
files and the URL, with *Fetch*, *Not Now* (asked again only through *File ›
Fetch Missing Packages…*) and *Never for This Project* (writes `fetch =
"never"`); ticking *Remember* on Fetch writes `fetch = "always"`. Resolved
files show under a dimmed *Packages* group in the sidebar. Nothing is ever
fetched without the sheet or a remembered `always`.

**The engine** never opens the manifest or the network: it receives the
documents the tools assembled and resolves `\usepackage{x}` /
`\documentclass{x}` among them by path — the entry's folder first, then the
`texinputs` files in manifest order, then the resolved packages at
`packages/<name>/<file>` — and takes the `[fonts]` table as the request's
optional `fonts` object (`{"text","math","mono","sans"}`); a request
without it renders exactly as before the field existed.
