# The project manifest: `flashtex.toml`

**Status:** shipped with slice S2 of
[packages, fonts and the project manifest](../proposals/packages-fonts-manifest.md)
(2026-09-19). Owner: mac-claude-a. The `[fonts]` and `[packages]` sections
are parsed and shown today; the fonts (S4) and package resolution (S3)
slices give them their effect.

A FlashTeX project is a folder with an entry document. That is all the
engine needs, and **without a manifest nothing described here happens**:
`flashtex build main.tex` and *File › Open* behave exactly as they always
have — the entry's folder is the project root, `\input`/`\include` resolve
under it, the PDF lands next to the entry. The manifest is for what a folder
cannot say by itself: which file is the entry when you open the folder,
extra directories of `.sty`/`.cls` files to search, where output goes, and
(later) fonts and package policy.

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
| `[fonts] text`, `math`, `mono`, `sans` | Font families by role, for the fonts slice (S4). Parsed and shown now; no effect yet. The document's own `\setmainfont` etc. will always override them. | unset (Latin Modern / Computer Modern) |
| `[packages] source` | Where a package that is neither in the project nor in the cache may be fetched from: `"ctan"`, `"none"` (never fetch) or a registry URL. Parsed now; the fetch is the package-resolution slice (S3). | `"ctan"` |
| `[packages] fetch` | `"ask"` (once per project), `"always"` or `"never"`. | `"ask"` |
| `[packages] pin` | Exact versions, `{ siunitx = "3.3.24" }`. | `{}` |
| `[packages] path` | Local libraries, `{ mylib = "../mylib" }`: a directory with its own `[library]` manifest. | `{}` |
| `[library] name` | Only for a directory that *is* a library other projects reference. | absent |

Whatever the manifest says that this version of FlashTeX does not understand
— an unknown key or section, a known key of the wrong type, an unexpected
value for `source`/`fetch` — is a **warning** naming the key
(`fonts.serif: unknown key; ignored`), never an error: a manifest written
for a newer FlashTeX still builds on an older one. Only a file that is not
valid TOML stops the build, because nothing past a syntax error can be
trusted.

## What each tool does with it

**CLI.** `flashtex build`, `check` and `watch` take a `.tex` file, a
folder, or nothing (the current folder). With a manifest, `texinputs` files
appear in the `documents` list of `--json` and the `--v2` envelope after
the include closure; warnings are `manifest_unknown_key` /
`manifest_texinputs` diagnostics attributed to `flashtex.toml`; the report
carries `manifest` (the path, or `null`). `watch` also polls the manifest
and every `texinputs` file. `flashtex manifest init [<entry>|<dir>]
[--force]` writes the template (refusing to overwrite without `--force`);
`flashtex manifest show [<entry>|<dir>] [--json]` prints the manifest that
governs the entry — the defaults when there is none — with each `texinputs`
entry classified (`inside`, `outside`, `invalid`) and the warnings. See
[Command-line tools](compiler.md#manifest).

**The Mac app.** *File › Open LaTeX File or Project Folder…* (⌘O) accepts
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
with comments. The manifest is read by the project-files helper (one parser
for the CLI, the helper and the app); without the helper a folder still
opens through its only `.tex` file and the status bar says the manifest was
not read.

**The engine** never opens the manifest or the network: it receives the
documents the tools assembled and resolves `\usepackage{x}` /
`\documentclass{x}` among them by path — the entry's folder first, then the
`texinputs` files in manifest order.
