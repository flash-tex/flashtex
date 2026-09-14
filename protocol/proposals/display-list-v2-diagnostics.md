# Proposal: `display-list-v2-diagnostics` — structured fields on v2 diagnostics

Status: **PROPOSAL ONLY** (issue #277 v2 transport; lane
`agent/daniel-parent/v2-structured-diagnostics`; #354 is on main). Additive
and opt-in. It changes no frozen contract: `protocol/rendering-v2.schema.json`,
`docs/contracts/**` and the `display_list` / `display_list_delta` bytes of a
request that does not ask for this capability stay byte-for-byte as they are.
Needs a consumer co-sign (Mac v2 decoder) before the Mac side relies on it.
Nothing in `apps/`, `protocol/*.schema.json` or `docs/contracts` changes with
this document.

The runtime-v1 sibling is
[`runtime-v1-structured-diagnostics.md`](runtime-v1-structured-diagnostics.md)
(PR #346): optional `suggestion`, `labels`, `notes`, `help` on each
`compile_result` diagnostic. This document is the same shapes on the
rendering-v2 `display_list` line, gated because `$defs/diagnostic` is
`additionalProperties: false`.

## 1. Summary

1. New layout capability string `display-list-v2-diagnostics`. It is accepted
   only when the same request also lists `display-list-v2`; unknown to old
   producers (never echoed), never sent by old consumers.
2. When accepted, each object in `payload.diagnostics` (full line **and**
   delta — deltas already carry the complete diagnostics array) MAY gain
   `suggestion`, `labels`, `notes`, `help` with **exactly** the runtime-v1
   proposal shapes. Nested `source` objects use the **v2** source already in
   the frozen schema (`#/$defs/source`, §3), not a new type. Each key is
   **omitted when empty**, never `null`, never `[]`, never `{}`.
3. `required_features` is **not** extended: extra diagnostic keys are not a
   paint feature. A consumer that cannot decode them must not request the
   capability.
4. Without the capability the `display_list` / `display_list_delta` bytes are
   byte-identical to today, including when a diagnostic's in-memory
   `suggestion` is `Some`. The delta hash (`dl2-canon-1` header digest)
   includes a field only when that field is serialised, so a suggestion
   change produces a delta if and only if the capability is on. Empty
   strings are omitted the same way as unset (`Some("")` is not on the wire
   and is not hashed).
5. The runtime-v1 `compile_result` is unchanged by this capability (PR #354
   already emits `suggestion` there without a flag).

## 2. Request

```json
{"protocol_version":1,"id":"…","type":"compile","payload":{
  "project_id":"…","revision":N,"entry_path":"main.tex","documents":[…],
  "layout_capabilities":["display-list-v2","display-list-v2-diagnostics"]
}}
```

The echoed `accepted` list names `display-list-v2-diagnostics` only when it
was honoured (same rule as `display-list-v2-images` /
`display-list-v2-device-color`): requested, not duplicated, and only together
with `display-list-v2`. Requested alone, or by a producer that does not know
the name, it is silently not accepted.

## 3. v2 `source` objects (frozen shape, reused)

Every nested `source` on the new fields is `#/$defs/source` from
`protocol/rendering-v2.schema.json` (the same object already used by
`diagnostic.sources[]`, glyph-run clusters, rules and paths):

| field | type | rule |
|---|---|---|
| `path` | `#/$defs/path` | Project-relative document of the request. Frozen path pattern: no leading `/`, no `.` / `..` segments, no `//`, no `\`, `:`, or NUL, must not end with `/`, length 1..4096. |
| `start_byte` | integer | UTF-8, zero-based, inclusive. Range `0 .. 2^53−1`. |
| `end_byte` | integer | UTF-8, end-exclusive. Same range. `start_byte == end_byte` is a caret / insertion. |

`additionalProperties` on `#/$defs/source` stays `false`. This is the same
three fields as runtime-v1 `{path, start_byte, end_byte}`, with the v2 path
pattern applied. Producers must not emit a runtime-v1 source that would fail
`#/$defs/path` (a leading `/`, a `..` segment, …).

v2 diagnostics locate the problem with **`sources` (array)**, not a single
nullable `source`. The primary span is `sources[0]` when the array is
non-empty. `labels[].source` and `help.replacement.source` are each one
`#/$defs/source` object, matching the runtime-v1 proposal's nested `source`.

## 4. Wire format (when the capability is accepted)

Each diagnostic object may gain these keys. They are **omitted when unset**.
They are never `null`. A producer that has nothing to say writes today's
object (`code`, `message`, `severity`, `sources` only).

```json
{
  "code": "unknown_command",
  "message": "\\alpah is not supported by this compiler version",
  "severity": "error",
  "sources": [
    {"path": "notes.tex", "start_byte": 1234, "end_byte": 1240}
  ],
  "suggestion": "\\alpha",
  "labels": [
    {
      "source": {"path": "notes.tex", "start_byte": 1234, "end_byte": 1240},
      "text": "this command",
      "primary": true
    }
  ],
  "notes": ["\\alpah looks like a misspelling of \\alpha"],
  "help": {
    "message": "did you mean \\alpha?",
    "replacement": {
      "source": {"path": "notes.tex", "start_byte": 1234, "end_byte": 1240},
      "text": "\\alpha"
    }
  }
}
```

Compact and JSON-tree writers emit keys in `BTreeMap` order, so when the new
keys are present they sort among the frozen ones:
`code`, `help`, `labels`, `message`, `notes`, `severity`, `sources`,
`suggestion`. When they are absent the bytes of that diagnostic are unchanged.

### 4.1 `suggestion`

Optional non-empty string. Replacement **text for the diagnostic's primary
span** (`sources[0]` when present) — currently a did-you-mean command such as
`\alpha` on `unknown_command`. Same meaning as runtime-v1 `suggestion`
(issue #76 / PR #354). Omitted when `None`.

### 4.2 `labels`

Array of one or more objects (omit the key when there are none):

| field | type | rule |
|---|---|---|
| `source` | `#/$defs/source` | §3. A secondary label may name a different path. |
| `text` | string | Caption next to the carets. Prefer a short phrase. |
| `primary` | bool | Exactly one element of a non-empty `labels` array has `primary: true`. |

When `labels` is omitted, `sources[0]` is the implicit primary span. Producers
that emit `labels` should keep the primary label's `source` equal to
`sources[0]` (or a subrange of it).

### 4.3 `notes`

Array of one or more non-empty strings. Each is one `= note:` line, without
the prefix. Not recovery, not the suggested fix.

### 4.4 `help`

A single object, not an array:

| field | type | rule |
|---|---|---|
| `message` | string | Required. The `= help:` text, no prefix. |
| `replacement` | object | Optional. Omitted when the help is advice only. |

`replacement`:

| field | type | rule |
|---|---|---|
| `source` | `#/$defs/source` | §3. **Need not** equal `sources[0]` (a replacement may edit an included file). `start_byte == end_byte` is an insertion. |
| `text` | string | The replacement (may be empty = delete). |

Advice-only help (`message` without `replacement`) is valid.

`suggestion` and `help.replacement` may both be set; consumers prefer
`help.replacement` when present and otherwise use `suggestion` as they do
on runtime-v1.

## 5. Schema delta (text only — do not edit `rendering-v2.schema.json`)

This is the intended addition to `$defs/diagnostic.properties`. The frozen
`required` list and `additionalProperties: false` stay. The new properties
are optional; a producer that omits them validates against today's schema.

```json
"suggestion": {
  "type": "string",
  "minLength": 1,
  "maxLength": 4096
},
"labels": {
  "type": "array",
  "minItems": 1,
  "maxItems": 128,
  "items": {
    "type": "object",
    "properties": {
      "source": { "$ref": "#/$defs/source" },
      "text": { "type": "string", "maxLength": 4096 },
      "primary": { "type": "boolean" }
    },
    "required": ["source", "text", "primary"],
    "additionalProperties": false
  }
},
"notes": {
  "type": "array",
  "minItems": 1,
  "maxItems": 32,
  "items": {
    "type": "string",
    "minLength": 1,
    "maxLength": 4096
  }
},
"help": {
  "type": "object",
  "properties": {
    "message": {
      "type": "string",
      "minLength": 1,
      "maxLength": 4096
    },
    "replacement": {
      "type": "object",
      "properties": {
        "source": { "$ref": "#/$defs/source" },
        "text": { "type": "string", "maxLength": 4096 }
      },
      "required": ["source", "text"],
      "additionalProperties": false
    }
  },
  "required": ["message"],
  "additionalProperties": false
}
```

`$defs/source` is unchanged. Deltas (`display_list_delta`) reuse the same
`diagnostics` array schema; there is no second diagnostic type.

A request that **did not** negotiate this capability must still validate
against the **frozen** schema (no extra keys). A request that **did** must
be validated against this delta (or a proposal-local copy of the schema).
`crates/render-pipeline` cargo tests do **not** currently run
`scripts/check_rendering_v2.py` against producer output. Frozen-path writers
(`DisplayList::to_json` / `write_json`, default `Wire`) stay capability-off,
so existing shape tests remain schema-valid. Capability-on bytes are asserted
only in dedicated unit tests that do not feed the frozen schema checker; no
proposal-local schema copy is added until a test needs one.

## 6. Producer semantics (this lane)

Negotiation copies `display-list-v2-images` / `display-list-v2-device-color`:
`CAP_DIAGNOSTICS` in `crates/render-pipeline/src/v1.rs`, accepted only with
`display-list-v2`; `display::Wire { diagnostics: bool }` set from that flag;
`write_diagnostics` (full line and delta) and the JSON-tree writer emit
`suggestion` when the capability is on and the value is a non-empty `Some`.

`dl2-canon-1` `header_digest` includes `suggestion` only when it is
serialised (`Wire.diagnostics && suggestion` is a non-empty string). A suggestion-only
edit therefore changes the list digest — and produces a delta — exactly when
the consumer asked for the fields.

### 6.1 What this producer fills now vs after #346

PR #354 already stores `display::Diagnostic.suggestion` (forwarded from the
vendored compiler). This lane serialises that field under the capability.

`labels`, `notes` and `help` are **not** slots on `display::Diagnostic` in
this lane. The vendored compiler (`crates/render-pipeline/vendor/compiler`)
does not yet carry those types — they land on `flashtex_compiler::diagnostics::Diagnostic`
in PR #346. Filling them in `from_compiler` needs a `vendor/compiler` re-pin
**past #346**, which is out of this lane (no `vendor/` edits). After that
re-pin, the producer should add the three slots (default empty, omit-when-empty
JSON, same delta-hash rule as `suggestion`) and copy them in `from_compiler`.
Until then a negotiated diagnostic may carry `suggestion` only.

## 7. Compatibility

- **Old producer, new consumer:** capability never echoed; diagnostics have
  only `code` / `message` / `severity` / `sources`. Consumer uses `sources[0]`
  as the only span, Fix… only from runtime-v1 `suggestion` if that path is
  used. Output looks like today.
- **New producer, old consumer:** the consumer does not request the
  capability, so the extra keys are never sent. Frozen `additionalProperties:
  false` continues to hold. No error, no corruption.
- **New producer, new consumer, capability off:** byte-identical to today,
  even if in-memory `suggestion` is `Some`.
- **New producer, new consumer, capability on:** extra keys present when
  set. A strict frozen-schema validator must not be pointed at those bytes
  (use this delta, or skip schema validation for capability-on fixtures).
- **Deltas:** a consumer that negotiated both `-diagnostics` and `-delta`
  sees the extra keys on the delta's `diagnostics` array. A consumer that
  negotiated only `-delta` sees today's diagnostic objects. Mixing a
  diagnostics-on base with a diagnostics-off follow-up is a wire mismatch
  (`Snapshot.wire` already refuses a delta when `Wire` differs).

## 8. What the Mac does (not in this lane)

Out of this task's paths (`apps/` is forbidden). Specified so a Mac co-sign
has a contract.

1. **Do not advertise `display-list-v2-diagnostics` until the v2 decoder
   accepts the extra keys.** Today's Mac v2 consumer is written against the
   frozen schema (`additionalProperties: false` on `$defs/diagnostic`).
   Requesting the capability without a decoder change would make a strict
   validate-then-paint path refuse the frame. The Problems panel today reads
   **runtime-v1** `compile_result` diagnostics (`RuntimeV1.Diagnostic`,
   issue #277 part (4) / PR #345), not the v2 sibling line, so preview
   painting does not need these fields to draw glyphs.
2. **When advertised** (alongside `display-list-v2`): decode `suggestion`
   now; decode `labels` / `notes` / `help` once the producer fills them
   (post-#346 re-pin). Unknown extra keys on a negotiated diagnostic are
   still a producer bug. Prefer `help.replacement` for Fix… when present,
   else `suggestion` as the replacement of `sources[0]`.
3. **`display-list-v2-delta`:** if the Mac acknowledges a diagnostics-on
   base, keep requesting the capability on the follow-up. The producer will
   not delta across a `Wire` change.
4. **PDF export / `crates/pdf`:** paint does not read diagnostic fields.
   No change is required for correctness of a page image.
5. **CLI (`flashtex check`, PR #303):** still consumes pipeline diagnostics
   through the runtime-v1 fallback unless it starts reading the v2 sibling.
   This capability does not by itself change CLI output.

## 9. Worked JSON example

Request (abridged):

```json
{"protocol_version":1,"id":"r1","type":"compile","payload":{
  "project_id":"p","revision":1,"entry_path":"notes.tex",
  "documents":[{"path":"notes.tex","text":"\\alpah"}],
  "layout_capabilities":["display-list-v2","display-list-v2-diagnostics"]
}}
```

Sibling `display_list` diagnostic (capability on, suggestion filled, labels /
notes / help still empty after this lane):

```json
{"code":"unknown_command","message":"\\alpah is not supported by this compiler version","severity":"error","sources":[{"end_byte":6,"path":"notes.tex","start_byte":0}],"suggestion":"\\alpha"}
```

Same diagnostic, capability **off** (byte-identical to today):

```json
{"code":"unknown_command","message":"\\alpah is not supported by this compiler version","severity":"error","sources":[{"end_byte":6,"path":"notes.tex","start_byte":0}]}
```

After the #346 vendor re-pin, the capability-on object may also include
`help` / `labels` / `notes` as in §4.

## 10. What this lane will implement (and what it will not)

In `crates/render-pipeline` and this proposal file only:

1. This document (no frozen-contract or schema edit).
2. `CAP_DIAGNOSTICS`, `Capabilities.diagnostics`, `Wire.diagnostics`,
   negotiation gated on `display-list-v2`.
3. Compact + JSON-tree writers emit `suggestion` when the capability is on
   and the value is `Some`; deltas use the same writer; `header_digest`
   hashes `suggestion` only then.
4. Tests: negotiated vs not (byte-identical without the cap; `suggestion`
   present with it); compact writer agrees with the JSON tree; a delta
   where only a suggestion changes is emitted.

Not in this lane:

- Editing `protocol/*.schema.json` or `docs/contracts/**`.
- `vendor/`, `crates/compiler`, `apps/`, `coordination/`.
- `labels` / `notes` / `help` struct slots and `from_compiler` forwarding
  of those (blocked on a vendor re-pin past #346).
- Mac decoder / Problems panel / CLI renderer.
