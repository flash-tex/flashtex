# Helper source contract review

Read-only peer reviewed: `9f28baebad9585940a390fef699fefc6a3e5d142`,
`crates/preview-controller/src/lib.rs` and `src/display.rs`.
Runtime reviewed: `106bca6e` ancestry including accepted-buffer compaction
`0333be37` and request-derived raw metadata bounds `00490d56`.

`Controller::compile_current` constructs each runtime document from its literal
path and immutable text. It sends the controller compile generation as the
request revision; individual editor revisions remain in the submitted/current
`VersionSnapshot`. Runtime source entries must have that compile revision and
exact request source digest/UTF-8 length. They are not editor-version claims.

`take_current_raw_display_payload` separately checks request/project/generation,
submitted versus current index snapshot, source count, each current durable editor
revision, source digest and byte length. It emits the current editor revisions in
`source_versions`, while `compile_revision` remains distinct. Compaction changes
only owned spare capacity, never these values or encoded request bytes.

`open_with_bibliography` validates an explicit source-kind declaration and feeds
bibliography documents to the corresponding index operation. Runtime requests do
not contain source-kind tags; all uploaded documents still participate in source
binding. This review does not claim bibliography rendering support or infer kind
from filenames. No runtime source-kind API need was found in this contract.

`restart` preserves the selected decoder constructor but disables display delivery
and removes `display-list-v2` from requested capabilities before the initial new
compile. A later explicit opt-in is required. Old runtime ownership is replaced;
compiled source generation and live editor metadata must still pass helper checks.

Static review found no acceptance mismatch. The independent actual-byte replay
below now verifies the combined capture. No modeled producer output or new runtime
API is added here.

## Capture proxy review before independent replay

The helper-owned `examples/producer_capture_proxy.py` forwards binary lines with
one writer in each direction. Logging occurs before forwarding, so a captured
input alone proves proxy receipt, and captured output alone proves producer
emission. Delivery must be corroborated by helper events and exact nested bytes.
Line buffering changes timing and is not suitable for an unqualified performance
claim. Interrupted captures must not be treated as completed transactions.

An initial in-progress version captured `getppid()` inside the forked child before
installing Linux parent-death notification. Review identified the race if the
proxy died before that read. The owner corrected it before the first actual run:
the expected proxy PID is captured before `Popen`, and the child compares its
parent with that exact PID after successful `prctl`. The current code was reread
and confirmed. Both producer PID files from the first capture pointed to processes
already absent from `/proc`; no surviving producer was observed. This is not an
arbitrary inherited-descriptor or grandchild-reaping guarantee.

## Published actual capture replay

Capture `e680d2ef` contains exact producer requests/replies and helper wrapper
bytes. Independent checks reverified every artifact hash and77 pinned assets,
all three source digests/lengths per sibling, explicit bibliography snapshot kind,
and raw wrapper bodies for generations2/4/6. Editor revisions remain7/19/23 until
the chapter advances to21; compile generations run1 through6.

The ignored `helper_capture` test replays the exact captured replies and requires
the runtime's serialized requests to be byte-identical too. It creates a new raw
session for the captured restart, retains default-v1 generations1/3, explicitly
reenables display for2/4, then submits5 and6 before polling. Generation5 produces
a stale event and no preview/candidate;6 delivers the exact captured current
output. All six accepted request byte streams and delivered v1/raw bodies match.
No runtime contract gap was found. The captured recovered `tfm_missing` outcomes
remain unchanged, and bibliography/native rendering correctness is not claimed.

Reproduce by decompressing `../benchmarks/helper-source-capture/replay-config.json.gz`
to a temporary JSON file and setting `FLASHTEX_HELPER_CAPTURE_CONFIG` to it. Run
`cargo test --test helper_capture -- --ignored --nocapture` with the runtime
manifest; optionally set `FLASHTEX_HELPER_RUNTIME_EVIDENCE` to an output JSON path.
The test executes a captured-byte replay subprocess, not a fresh producer.

## Corrected official 10pt metric capture

The independent data-only follow-up consumes `eca6ab25`. All control/verified
artifact hashes and resource hashes were checked, including the official LM2.004
archive's `ec-lmr10.tfm` member and GUST license. The six serialized requests are
byte-identical across the earlier `e680d2ef` capture, new control and corrected
capture. Control and corrected binaries match each other; the earlier capture
used a different helper build, so it is not described as binary-identical to them.
The original producer binary remains unchanged.

All six corrected compile results are `ok` with empty diagnostics. Every sibling
retains its exact source digest/length/compile revision; raw helper wrapper bodies
for2/4/6 match producer bytes. The unchanged captured-byte consumer passes the
restart and supersession assertions. No runtime/API code changed or full-suite
rerun was needed for this fixture-only follow-up. Original recovered evidence
remains intact. See `../benchmarks/helper-source-capture-corrected` for complete
verified comparison metadata, exact replay config, results and artifact digests.
This supports the missing-metric resource gate only, not native or visual parity.
