# Bounded protocol probe final review

Read exact ownere975f38f `tools/bounded_protocol_probe.py` under preview-controller.
No workload or test suite repeated. Earlier925590b9 concerns are addressed:

- Final stdout is checked through a bounded EOF wait after closing stdin. Any
  extra byte is preserved as unexpected-output-prefix and refuses success.
- Request ID and compile-result project/revision are checked independently of
  reference equality. Complete JSON response equality includes diagnostics.
- Exchange failure retains at most max_reply+1 response bytes; complete mismatches
  were already saved in response.jsonl. Partial bytes are not successful results.
- Existing output directories refuse instead of leaving an old result.json as
  false success. NaN/infinite/nonpositive timeout is refused before startup.

Per-exchange nonblocking writes and reads share one deadline. Final EOF/process
wait has a separate bounded deadline; finally kills and waits for the direct child.
No extra background reader is introduced. Success result.json is written only
following complete exchange validation and terminal child success. Owner eight
fault tests and ten actual compiler exchanges are reported evidence, not a local
reviewer execution.

Scope limits: this is a one-response-line-per-request probe, not a negotiated v2
sibling runner. JSON object equality is semantic and does not preserve duplicate
key distinctions or original spelling; original complete replies are retained.
It does not validate full runtime protocol schema, constrain total input/reference
file size, bound stderr-file growth, or guarantee descendant termination. Final
unexpected-output evidence intentionally retains only the first extra byte.
Filesystem writes and kernel child teardown are not hard real-time guarantees.
Within the pinned compiler fixture scope, no remaining concrete false-success
issue was found. No performance or native rendering claim follows from review.
