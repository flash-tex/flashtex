# FlashTeX Rust capture bridge

Original Rust application layer for capture receipt, source context, Grok
transcription and reviewed UTF-8 anchored insertion. This is a separate process
from the compiler. See the authoritative [transfer contract](../../docs/contracts/transfer-v1.md).

```sh
cargo test --manifest-path crates/bridge/Cargo.toml
cargo run --manifest-path crates/bridge/Cargo.toml -- --store /private/app-data/captures
```

The private journal must be outside the repository.

## Conversion providers

Conversion is disabled by default. `--conversion-provider <none|xai|openai-compatible>`
plus an explicit `capture_convert` request enables one request (with bounded
retries) to that provider. `--enable-grok` is a deprecated alias for
`--conversion-provider xai`, kept for one release. Passing both with different
providers is `invalid_arguments`. The bridge never reads
`FLASHTEX_CONVERSION_PROVIDER`: an environment variable alone cannot enable a
network call. The Mac's credential adapter supplies the rest per request:

| Variable | `xai` | `openai-compatible` |
|---|---|---|
| `FLASHTEX_AI_API_KEY` | key, then legacy `XAI_API_KEY` | key, optional only for a loopback base URL |
| `FLASHTEX_CONVERSION_MODEL` | model, then legacy `FLASHTEX_GROK_MODEL`, then `grok-4.6` | required (`invalid_model`) |
| `FLASHTEX_CONVERSION_BASE_URL` | default `https://api.x.ai/v1` → `/responses` | required → `/chat/completions` |
| `FLASHTEX_CONVERSION_MAX_ATTEMPTS` | 1–5, default 3 (legacy `FLASHTEX_GROK_MAX_ATTEMPTS`) | same |

Base URLs must be `https://`, or `http://` on `127.0.0.1`, `localhost` or
`[::1]`, so a local model server works without a key. No provider fallback or
purchase occurs. Both providers get the same prompt, strict JSON schema, image
input, proposal validation, LaTeX safety scan and review gate. xAI uses the
Responses API with `store:false`. The openai-compatible provider uses Chat
Completions with a strict `json_schema` `response_format`.

`capture_proposal` and `capture_status` carry `provider_evidence`
(`{provider, model, response_id, usage:{input_tokens, output_tokens, total_tokens}}`,
or `null`), which is also persisted in the capture journal. It never contains
the key, the image or the source context. `src/provider.rs` is the seam. See
[`docs/design/on-device-conversion.md`](../../docs/design/on-device-conversion.md)
for the planned on-device provider. `tests/conversion_provider.rs` drives both
providers through the real binary against a loopback stub that replays
`tests/fixtures/providers/*.json`, with no network access.

Until FT-066, `context::build` accepted at most 64 `supported_features`. The
compiler-derived list has more than 100 entries, so every real
`capture_convert` through the binary failed with `context_too_large` before
reaching a provider. The bound is now 1024 entries and 16 KiB total
(`features::tests::derived_list_fits_the_conversion_context_limits`).
References: [xAI image understanding](https://docs.x.ai/developers/model-capabilities/images/understanding)
and [structured outputs](https://docs.x.ai/developers/model-capabilities/text/structured-outputs).

Implemented: atomic durable receipt, duplicate detection, decoded image bounds,
source-aware context limits, deterministic provider response checks, anchor
rebasing/invalidation, review requirement and idempotent insertion receipts.
Tests use a deterministic fake converter by default; they do not spend API
credits. One successful live round trip has been made against the real xAI
endpoint, on a synthetic rendered-math image, proving the request/response
plumbing (auth, strict schema, image upload, response parsing) works for that
clean case. Actual handwriting and photo-of-paper conditions (lighting, paper
texture, camera noise, real strokes) remain unproven. A gated, `#[ignore]`d
integration test (`tests/live_grok.rs`) makes further live checks cheap to
run for anyone holding an `XAI_API_KEY`; it never runs in a normal `cargo
test` and asserts only on response structure, never on exact output.

## Restart recovery contract

`Store` (`src/store.rs`) durably persists one `CaptureRecord` per capture: the
original submission, its `request_sha256`, the converted `Proposal`, the
`Context` it was converted against, the `destination_binding` it was captured
against, the `PreparedEdit` (including `document_before_sha256`), the
`AppliedEdit` receipt, and whether it was rejected. All of that survives a
bridge crash or restart, because it is fsynced to disk before any reply is
sent (`Store::save`) and re-validated on every read (`Store::get`).

Open document snapshots (`document_open`) and destination pins
(`destination_pin`) do **not** survive a restart. An anchor pinned with
`mode: "caret"` (the Mac's automatic destination; transfer-v1 additive) is
simply re-pinned by the Mac wherever the caret is after a restart — the same
id at any range and revision — and `capture_prepare_insert` accepts an
optional `wrap {prefix, suffix, kind}` that is journaled with the prepared
edit (`replacement = prefix + proposal + suffix`); see
`docs/contracts/transfer-v1.md`. `Bridge::documents` and
`Bridge::anchors` (`src/lib.rs`) are ordinary in-memory maps, populated only
by those two requests; a freshly spawned bridge process starts with both
empty. This is deliberate, not an oversight: the Mac's live file is the only
source of truth for the document's actual current text, and caching a
snapshot across a restart would risk preparing or applying an edit against
source that has since changed underneath it. Reject-by-default is the safe
default; the cost is that the client must know what to resend.

**What a client must resend after a bridge restart, per request:**

| To resume with... | Resend first |
|---|---|
| `capture_status` | Nothing. It only reads the durable store and works immediately. |
| `capture_applied` (confirm an already-prepared edit) | `document_open` with the exact pre-edit snapshot (`project_id`/`path`/`revision`/`text`) the prepared edit's `document_before_sha256` was computed from. `destination_pin` is not needed for this call. |
| `capture_prepare_insert` (replay an existing prepared edit) | Both `document_open` **and** `destination_pin`, reproducing the original binding exactly. The idempotent-replay path re-verifies proposal context (`verify_proposal_context` → `context()` → `capture_anchor()`), which needs the in-memory anchor, not just the document. |
| `capture_submit` / a first `capture_prepare_insert` on a new capture | Both `document_open` and `destination_pin`, as for the initial (non-restart) flow. |

A client that sends `capture_applied` (or any other document-needing request)
to a freshly restarted bridge without first resending `document_open` gets:

```json
{"code":"document_missing","message":"Open the source snapshot on the Mac first"}
```

This does not lose anything: the paid `Proposal` and the reviewed
`PreparedEdit` are already durable, `capture_applied` has not mutated
anything, and simply resending `document_open` (then, if needed,
`destination_pin`) and retrying recovers without another paid `capture_convert`
call. `tests/restart_recovery.rs` and `tests/restart_cli.rs` pin exactly this
sequence — the second through genuinely separate OS processes talking the
real JSON-lines protocol against one `--store` directory, reproducing the
measured failure and its recovery.

`document_before_sha256` on a `PreparedEdit` is enforced, not merely reported:
both `confirm_insert` and the idempotent-replay branch of `prepare_insert`
recompute the digest of whatever document is currently open and compare it
against the value captured at preparation time, before allowing any mutation
or replay. If a client resends a document under the *same* revision number
but with different bytes at the prepared offsets (a stale cache, or a bug),
the digest mismatch turns that into a rejected `revision_conflict` rather than
an insertion at a now-wrong byte offset. `tests/restart_recovery.rs`'s
`confirm_insert_rejects_a_same_revision_but_different_text_snapshot_after_restart`
proves this by attempting exactly that and asserting no mutation occurs.

Outstanding: encrypted paired nearby networking, real native credential and
transaction adapters, actual compiler validation before review, real Pencil and
camera evidence, end-to-end crash/device tests. Do not describe the local protocol
as a completed secure Mac/iPad connection or as proof of LaTeX compatibility.
