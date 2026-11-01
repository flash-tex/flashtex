# Actual display producer through helper

Release helper source `42425a5` consumes the unchanged original display producer
`65dbe7da7a182e99322070e2c9763cc3b69a342b`. The harness verifies the producer binary
and every supplied font/TFM/license hash against FT049's existing provenance before
running. Exact binary, input, script and artifact hashes are in provenance.json.

Three states passed: original document, first durable edit, second durable edit.
For each state, the helper's v1 result and untrusted display envelope are exactly
equal as JSON values to fresh direct producer replies for the same request ID,
compile generation, capabilities and source. Each source binding matches actual
UTF-8 text. Individual document revisions 1/2/3 remain distinct from compiler
revisions 2/3/4. Matching v1 arrives before the optional candidate. All producer
v1 diagnostics are empty. Killing/reopening the helper restores the exact final
source document. Default-off startup still produces ordinary v1 output.

This is actual producer/runtime/helper transport evidence, not native rendering,
PDF raster parity, typing latency, or live AI-provider evidence. Font/resource
validation and a native helper-route paint measurement remain separate gates.

Reproduce with `examples/helper_display_replay.py --help`; supply the pinned
producer, release helper, FT049 producer provenance and request fixture. Required
asset paths must already exist and match hashes. The harness downloads nothing.
