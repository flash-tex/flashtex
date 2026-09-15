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
