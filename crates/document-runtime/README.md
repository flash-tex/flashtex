# Persistent compiler document runtime

Original Rust session manager for FlashTeX live preview. It launches the explicitly
configured original compiler once and keeps stdin/stdout open across edits. Native
integration is outstanding; this library does not claim a measured native
keystroke-to-visible-update latency or exact reference-PDF fidelity.

`Session::spawn(binary, Limits)` starts a session. `submit(Request)` accepts a
complete unsaved project snapshot with a new per-project revision and live request
ID. `poll()` returns explicit events. Call these from the app's worker layer;
shutdown and failure cleanup can wait for the child process and must not run on the
UI thread. The caller should poll promptly so available results reach the UI.

Only one compile is in flight. Each project retains at most one queued snapshot;
newer queued snapshots replace older ones with a `Superseded` event. In-flight
results are validated then emitted as `Stale` when superseded, never presented as
the newest preview. No source is approximated or silently reused. `Preview` carries
the exact compiler envelope, project/revision and observed queue/compiler/total
milliseconds. These timings include polling delay and exclude native painting.

Concurrent bounded writer/reader channels prevent pipe deadlocks. Stderr is drained
in a fixed buffer without retaining potentially sensitive logs. Frame, project and
unconsumed-event limits apply backpressure. Every accepted unfinished request gets
an explicit failure if the child crashes, sends invalid correlation/UTF-8 mappings,
or times out. Future submissions to a failed session fail. Recovery is explicit:
create a new session with complete current snapshots; no automatic restart loop.

The child is trusted project code. Drop kills and reaps that direct process; this
is not a sandbox for arbitrary executables or independently detached descendants.
Runtime-v1 text primitives are checked; rendering-v2 requires negotiated integration
before additional primitives are accepted. No PDF producer or compiler internals
are replaced here.

```sh
cargo test --manifest-path crates/document-runtime/Cargo.toml
FLASHTEX_TEST_COMPILER=/absolute/original/flashtex-compiler \
  cargo test --manifest-path crates/document-runtime/Cargo.toml -- --include-ignored
```

The optional real-compiler test compares three persistent-process results against
fresh-process results including Unicode. It is a scoped equivalence check, not proof
of full LaTeX compatibility, incremental cache reuse or universal pixel perfection.

Replay a JSONL sequence of runtime-v1 complete project snapshots:

```sh
cargo run --manifest-path crates/document-runtime/Cargo.toml --example replay -- \
  /absolute/original/flashtex-compiler \
  < crates/document-runtime/examples/edits.jsonl
```

The example exits nonzero on any persistent/fresh JSON mismatch or runtime failure.
It emits per-edit measurements and nearest-rank p50/p95/p99/max. Inputs are bounded
at 10,000 edits and the session frame limit. The included six-edit fixture changes
an included file through Unicode, math, a broken expression and repair. Unsupported
features may produce diagnostics: equality is not proof that features compiled
correctly. Compiler execution, pipe transport and polling are reported together
because runtime-v1 has no trusted compiler-only timing field. This harness does not
produce or compare reference PDFs and does not measure UI painting. For rigorous
performance evidence use a release compiler and substantially larger workloads.

Call `close_project(project_id)` when the app closes a project to reclaim its
capacity and emit `Cancelled` exactly once for accepted active/queued work. Poll
pending events first if backpressure rejects the close. A cancelled in-flight wire
request is drained and checked before dispatch resumes; its result is never shown.
A reopened project can start a new revision sequence using a distinct live request
ID. A hung cancelled compiler still reaches the ordinary timeout and requires a
fresh session. This avoids silently treating cancellation as process interruption.

`submit_with_capabilities(request, capabilities)` explicitly negotiates the additive
runtime-v1 `rules-v1` and `font-hints-v1` contract. Default `submit` omits capabilities.
Acceptance is validated against each exact in-flight request; unknown/unrequested
acceptance, malformed rules/fonts and unknown primitives fail the session. Typed
rule coordinates and dimensions obey the one-million-unit bound. The exact compiler
JSON and paint order are preserved. Acceptance is optional: consumers must inspect
missing capabilities and report unsupported rendering rather than guess support.
