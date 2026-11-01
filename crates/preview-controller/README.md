# Durable editor preview controller

Pure Rust worker-layer integration of `edit-ledger`, `project-index` and
`document-runtime`. A native adapter owns one controller per project and calls it
on a worker thread: source persistence includes fsync and must not block the UI.

Pass initialized locked document stores and an explicitly configured original
compiler command into `Controller::new`. Construction recovers the index from
those durable documents; call `compile_current` for the initial preview.
`replace_document` accepts an expected revision/hash and the new source text.
Successful source persistence precedes index update and compilation. An `Ok`
`EditOutcome` always carries the durable document; `preview_error` reports a
subsequent cache/compiler failure without pretending the source save failed.
Storage uncertainty returns an error and requires dropping/reopening the ledger.

`poll` delivers only previews matching the current compile generation, request
identity and complete source version vector. `is_current_preview` checks the same
compile generation explicitly; matching source revisions alone do not survive a
compiler restart.
Before painting a retained event, native code must recheck `is_current_preview` on
the serialized controller worker: a newer edit may have arrived since delivery.
`close` cancels publication and further edits. Drop releases source store locks.
`restart` recreates disposable compiler/index state from durable source after a
compiler failure; it cannot repair a poisoned persistence handle in place.

This is not a cross-crate transaction. A crash between the durable edit and cache
update is recovered by rebuilding caches, with source remaining authoritative.
The controller does not write exported `.tex` files, invoke a provider, approve a
capture, implement native keyboard bindings, or render PDF. Store membership is fixed for this first
API; callers reopen with the intended set of initialized stores to change it.

`save_and_submit_ms` includes persistence/index/submission. Preview
`runtime_total_ms` includes the runtime queue and compiler/pipe/poll observation.
Neither measures native painting. Calling code must measure from the original
keystroke through the actual paint completion for the <200ms acceptance gate.

```sh
cargo test --manifest-path crates/preview-controller/Cargo.toml
FLASHTEX_TEST_COMPILER=/absolute/original/flashtex-compiler \
  cargo test --manifest-path crates/preview-controller/Cargo.toml -- --include-ignored
```

Synthetic tests exercise the actual durable ledger and index with a clearly fake
compiler. The opt-in original-compiler test additionally compiles persisted edited
source. Neither proves reference-PDF parity or full TeX/package compatibility.

`Preview::controller_total_ms` measures from the controller's successful edit call
through observed compiler result, including the durable save and index update.
For explicit compile/restart it starts at compile invocation. This still excludes
UI event delivery before the worker call and actual painting afterward. Native
code must also compare its own current editor revision at paint time; a worker
freshness check alone is not an atomic transaction with the UI rendering thread.

For reviewed capture insertion, construct `ApprovedEdit` only in the explicit user
approval handler after showing the exact prepared edit, then call `apply_reviewed`.
The type makes approval an explicit call-site obligation; it cannot verify that a
human clicked a button and must not be constructed by automatic conversion code.
The returned `AppliedOutcome` carries a durable ledger receipt and the current
source state even if preview submission fails. Repeating the identical edit after
restart returns its original receipt and never inserts twice. Use `recovery(path)`
to resend pending receipts, and `confirm_receipt` only after the bridge acknowledges
that exact receipt. Confirmation does not change source or trigger compilation.

Prefer `open_without_compiler` when opening an editor: source and navigation remain
available even if the compiler executable is missing. Edits remain durable and
return an explicit preview-unavailable status. Call `restart` once an original
compiler is configured to attach it and compile the latest saved snapshot. A failed
attachment never overwrites source. `new` is the convenience constructor for a
known available compiler.

`file_project::FileProject::open(project_root, private_ledger_root, project_id,
entry)` imports discovered UTF-8 source into private durable stores. Existing
ledger source always wins on reopen, including when the entry was deleted from
disk. Bindings include canonical root and project identity, so identical project
IDs under different roots do not share stores. `inspect` explicitly distinguishes
matching source, changed disk content, missing files and unavailable reads; it
never updates the source or silently treats a disk change as accepted.

File discovery uses the shared project-files graph. Entry canonicalization adds a
consumer preflight for its unchecked initial path, but is not a race-proof rooted
IO capability. Export is explicitly disabled pending shared rooted-save issue GH18.
Import is durable per document, not an atomic project-wide transaction. The shared
graph's discovery IO limits and native file UI integration remain outstanding.

A repeatable release-helper latency probe runs complete durable edit round trips:

```sh
python3 crates/preview-controller/examples/helper_latency.py \
  --helper /absolute/flashtex-preview-controller \
  --compiler /absolute/original/flashtex-compiler --edits 100
```

It uses temporary file-project storage, sequential edits to one 20-paragraph source,
validates clean compile status and exact source revision, and records both executable
hashes. One Linux run measured p50 3.07ms, p95 3.61ms, p99 4.09ms, max 4.22ms across
100 edits. This is a narrow local observation, not a native responsiveness guarantee:
it excludes UI event handling and painting, uses the temporary filesystem, and does
not exercise complex packages or reference-PDF parity. The probe caps edits below
ledger history capacity and never evicts history to improve the measurement.
