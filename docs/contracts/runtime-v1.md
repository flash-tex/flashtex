# FlashTeX runtime protocol v1

Owner: Commander (FT-001). Status: initial interface contract, not implemented
compiler behavior. JSON examples live in `protocol/fixtures/`.

Mac UI and Rust worker communicate via UTF-8 JSON Lines over standard input/output.
One complete JSON object per line; diagnostic logs go to stderr. Every envelope
has `protocol_version: 1`, a unique string `id`, `type`, and object `payload`.
Replies preserve request `id`. Unknown versions/types return `error`, never silent
success. Set implementation message-size limits and reject oversized payloads.

## Compilation

`compile` payload: `project_id`, integer `revision`, `entry_path`, and `documents`
array of `{path, text}`. Paths are project-relative; reject parent traversal and
absolute paths. Unsaved text is supplied explicitly. UI must never block waiting
for compilation and must not replace a newer preview with an older revision.

Optional absolute `project_root` (FT-063, `display-list-v2-image.md` §2; the
worker's `--project-root DIR` is the same directory, and the per-request value
wins). It is where the producer reads the project from, so `documents` is the
set of buffers the client has open rather than the whole project: the producer
completes the `\input`/`\include` closure from the root through project-files'
rooted, symlink-refusing discovery, with `documents` overlaid so an unsaved
buffer always beats the file on disk and is what is scanned for further
includes. Resolution is TeX's — every include path is relative to the job's
root, never to the directory of the file naming it.

A path escaping the root is **never read**. Which diagnostic says so depends on
what stopped it, and the producer reports the accurate one: an escape that
would otherwise have resolved to a readable file — `..`, or a symlink whose
target is a regular file outside the root — is refused with `invalid_path` or
`path_escapes_root`; a path that does not name an includable file at all (a
symlink to a FIFO, socket, device or directory, or a dangling one) never
becomes a candidate, so it is reported as the compiler's `recovered_input`
"included file not found", with the names that were tried. Containment is the
same either way; only the explanation differs. A genuinely missing include
keeps the same `recovered_input` diagnostic.

The read is bounded, not just the reply: discovery is costed before it runs and
refused whole if it would read more than 256 files, more than 8 MiB in any one
file, or more than 32 MiB in total. Refused means the compile proceeds on
exactly the `documents` the request sent, plus an error diagnostic
`closure_budget_exceeded` naming the limit and the file that reached it — never
a partial closure, which would typeset a document quietly missing some of its
includes. Documents the request carries are served from the request and cost
nothing, so a client that sends its whole project is never refused.

Without `project_root` the producer reads nothing
from disk and `documents` is the entire project, exactly as before.

`compile_result` payload: same `project_id` and `revision`; `status` is `ok`,
`recovered`, or `failed`; `pages`, `diagnostics`, and optional `pdf_path` (null
until a real artifact exists). `pages` contain `number` (1-based), `width_pt`,
`height_pt`, and `items`. Coordinates use points, origin top-left. A text item has
`kind: text`, `text`, `x_pt`, `baseline_y_pt`, `font_size_pt`, and `source`.

`source`: `{path, start_byte, end_byte}` with zero-based, end-exclusive UTF-8 byte
offsets into the specified input revision. Swift must convert explicitly to its
editing coordinates. Image/line/path item types will be added by contract revision;
do not independently invent them in separate apps. Initial fixtures exercise text.

Diagnostics: `{severity, message, source, recovery}`. Severity is `error` or
`warning`; source may be null where mapping is unknown. Recovery is null or a
short description of provisional rendering. A fixture is not a generated PDF or
evidence of compiler completeness.

Additive optional diagnostic fields (issue #76; absent, never null, when unset —
consumers must tolerate their absence and unknown `code` values):
`code` is one of `unknown_command` (no known LaTeX layer defines the command —
usually a typo), `unsupported_feature` (real LaTeX this compiler does not
implement), `syntax_error`, `export_limitation`, `fidelity_note`, or
`recovered_input` (author-fixable input such as a missing include or undefined
reference). `suggestion` is replacement text for the diagnostic's source range,
currently a did-you-mean command such as `\alpha` on `unknown_command`.
Classify by `code`, not by `message` wording.

`help` carries a human `message` and an optional `replacement`: the mechanical
edit behind Fix… / Tab-to-apply. Its range is a nested `source` object, the same
`{path, start_byte, end_byte}` shape as `labels[].source`, so a fix can name a
file other than the diagnostic's own:

```json
"help": {"message": "did you mean \alpha?",
         "replacement": {"source": {"path": "main.tex", "start_byte": 5, "end_byte": 11},
                         "text": "\alpha"}}
```

Offsets are UTF-8, zero-based and end-exclusive. A flat `start_byte`/`end_byte`
pair directly on `replacement` is also accepted and takes precedence, and a flat
`path` overrides `source.path`; producers should emit the nested form. A
consumer that cannot find a range in either place must reject the replacement
alone, never the enclosing frame.

## Capture and insertion

`capture_submit`: `capture_id` (stable unique ID), `destination_id` (Mac-pinned
anchor), `base_revision`, `image` with `mime_type` and `data_base64`, and `instructions`.
Initial accepted MIME types: `image/png`, `image/jpeg`; reject malformed/oversized
images. The fixture image is a one-pixel transport example, not a handwriting test.

`capture_received` acknowledges only durable receipt of `capture_id`.
`capture_proposal` returns `capture_id`, `latex`, `ambiguities` array, and
`required_dependencies` array. It does not insert automatically. Mac presents a
review and applies one undoable edit after approval. If the destination was deleted
or cannot be rebased safely, require reselection. Repeated capture IDs must not
produce duplicate insertions. Internet requests and project context assembly live
in the Rust application layer; companion sends images/context notes to Mac.

Both capture methods must use this same payload. Keep physical transfer framing
and authentication separate from the semantic message; v1 examples do not establish
a secure network implementation.

## Immediate consumers

- FT-003: native shell reads `compile-result.json`; show fixture status explicitly
  until attached to a real Rust compiler. Source navigation uses the mapped range.
- FT-004: drawing and camera produce a `capture_submit` message. Real photographs
  and Pencil input are required for acceptance; generated fixtures are insufficient.
- Commander: owns v1 changes until an interface owner is explicitly reassigned.

## Optional request date

`payload.date` (a `YYYY-MM-DD` civil date) is specified in
[protocol/proposals/runtime-v1-request-date.md](../../protocol/proposals/runtime-v1-request-date.md)
(implemented, despite the filename): `\today` prints the real date, and the only
way to do that without breaking the determinism rule above is for the caller to
read the clock and send the answer as an ordinary request input. `apps/mac`
already sends it on every compile request (`RuntimeV1.localDate()` in
`ShellModel.swift`); `crates/render-pipeline` threads it to the compiler by
default (`request-date` is a default Cargo feature). An absent field means the
Unix epoch, so every request written against this document stays valid and
byte-identical.

## Optional negotiated layout extension

See [runtime-v1-layout-capabilities.md](runtime-v1-layout-capabilities.md) for per-request
`rules-v1` and `font-hints-v1`. They do not change output for unnegotiated clients.
Unknown primitives must be explicitly rejected, never silently omitted.

## Optional `metadata` object

`compile_result.payload.metadata` carries sections a client uses for editor
intelligence, not for drawing pages. It is absent (never null) when no section
has anything to say, so a reply for a project without project package files is
byte-identical to one written before the object existed; consumers must
tolerate its absence and ignore unknown sections. Every span in it is
`{path, start, end}`: the project-relative document path and zero-based,
end-exclusive UTF-8 byte offsets into the request's revision.

Both producers emit it: the compiler's own `compile_result`
(`crates/compiler/src/protocol.rs`) and `flashtex-render`
(`crates/render-pipeline`, `v1::metadata_json`), which serialises the same
records with the vendored compiler's `package_definitions::to_json`, so the
section reads identically whichever worker a client runs. In sorted key
order it sits between `layout_capabilities` and `pages`.

### `metadata.packages`

One entry per project `.sty`/`.cls` file the expansion pass read
(`\usepackage`, `\RequirePackage`, `\documentclass`, `\LoadClass` resolved to a
document of the request), in loading order:

```json
{"path": "mystyle.sty", "kind": "package",
 "provides": {"name": "mystyle", "date": "2024/01/02", "version": "v1.3",
              "description": "my macros", "span": {"path": "mystyle.sty", "start": 24, "end": 73}},
 "loaded_by": {"path": "main.tex", "start": 24, "end": 35},
 "options_declared": [{"name": "draft", "span": {"path": "mystyle.sty", "start": 74, "end": 112}}],
 "definitions": [
   {"name": "emphx", "kind": "macro", "definer": "newcommand", "arity": 1,
    "optional_default": null, "signature": "[1]", "overrides": false,
    "span": {"path": "mystyle.sty", "start": 113, "end": 149}}]}
```

- `kind` is `package` (`.sty`) or `class` (`.cls`).
- `provides` is the file's `\ProvidesPackage`/`\ProvidesClass`, or null before
  one is seen. The bracket is split as `YYYY/MM/DD vX.Y description` when it
  follows that layout; a part that does not is null.
- `loaded_by` is the command that loaded the file. For a nested load
  (`a.sty` `\RequirePackage`s `b.sty`) it lies in the loading package's
  document, so following `loaded_by` from entry to entry gives the load chain
  back to the entry document. Diagnostics raised inside a package carry the
  same chain as `labels` (`b.sty is loaded here` in `a.sty`, `a.sty is loaded
  here` in `main.tex`).
- `options_declared` lists the file's `\DeclareOption` names in order, `*`
  for `\DeclareOption*`, each with the span of the whole declaration.
- `definitions` lists what the file defined at its outermost level -- the
  defining command was read from the file's own text, not from a macro
  body, an option's code run by `\ProcessOptions`, or an `\AtEndOfPackage`
  hook -- in order, including repeated definitions of one name. `kind` is
  `macro` (`\newcommand` & co., `\def` & co., `\let`,
  `\DeclareRobustCommand`, `\NewDocumentCommand` & co.), `environment`,
  `conditional` (each of the three commands a `\newif` or `\newboolean`
  creates), `counter`, `length`, `register` (`\newcount` & co.), `theorem`
  or `math_operator`; `definer` is the defining command without its
  backslash. `arity` counts the parameters (a `\let` copy reports the copied
  macro's); `optional_default` is the `[default]` of a LaTeX definer's
  optional first parameter; `signature` is the parameter shape as written
  (`[2][x]`; a `\def` parameter text such as `#1\stop`; an xparse argument
  specification such as `O{x} m`). `span` is the whole defining statement,
  `\global`/`\long` prefixes included, trailing spaces and comments
  excluded; `overrides` says the name had a meaning before (a
  `\renewcommand`, a `\def` over a taken name, a `\let` over an existing
  command). A `theorem` entry adds `title` (the heading text) and `within`
  (the counter it is numbered within, or null); a shared counter
  (`\newtheorem{lem}[thm]{Lemma}`) appears in `signature` as `[thm]`.
