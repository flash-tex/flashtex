# Actual producer → raw helper replay

Release helper source `e39f50e594703c0d4b4bd6848e72fd2fe5169678` consumes
runtime `7817e4e8` unchanged. Startup explicitly selects `raw-prototype`, followed
by exact `display-candidates-raw-v1` acknowledgement and renderer confirmation.
The unchanged producer is `65dbe7da7a182e99322070e2c9763cc3b69a342b`, with pinned
font/TFM/license assets in provenance. Default helper mode is not changed.

All three repeat-100 source states have zero diagnostics, exact direct v1/v2
semantic equality and exact original raw sibling bytes inside the helper wrapper.
Each candidate follows its matching current v1 and preserves separate source
revisions1/2/3 and compile generations2/3/4. Final helper kill/reopen restores the
exact document. This is transport/source evidence, not native renderer acceptance.

The compressed `step-N.candidate.jsonl.gz` files contain original helper event
bytes; `step-N.producer.jsonl.gz` contain original direct producer stdout bytes.
`step-N.json.gz` holds request and parsed evidence for identity review, not raw-byte
proof. All artifact hashes are recorded. The raw-body assertion checks the exact
original producer value at the typed wrapper's final display_list field, excluding
producer framing newline/outer whitespace. No JSON re-encoding supplies that proof.

This was a correctness-only run while Commander integration86023 was active.
Candidate receipt65.6/129.7/257.8ms and the scalar stage timings are uncontrolled
single observations; do not compare them with earlier runs to infer a speedup or
claim sub200ms native paint. No model/provider calls were made.
