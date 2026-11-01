# Independent typing harness review

Read-only review of helper capture `58d27314` verifies all original/compressed
artifact hashes and declared resources. Full replacement and the literal grouped
edit produce the same 50,000 UTF-8 bytes. The actual edit is byte offset53 versus
character offset52 after the multibyte comment. Both commands bind the same
expected source revision1 and digest, and the returned documents bind revision2
and the exact edited digest. Final helper-delivered producer result Values match.

The retry checks the same command at current revision2 and command revision2.
It proves durable source idempotency, not behavior after a later revision or
conflicting command-fingerprint reuse. Retry processing can submit another
compile through the existing helper path; no no-extra-compile claim is made.
The paired ACK/preview measurement precedes retry, while later diagnostics must
be attributed to the retry separately. This is a single shared-load debug pair,
not a calibrated speed comparison or native latency result.

That original harness did not capture compiler stdin. Literal source comparison
was therefore not labeled actual compiler-request equality. The later actual
proxy capture `53c0c71d` and pinned release/debug replay provide that evidence
separately; see `producer-release-acceptance.md`. No helper code or runtime API
was changed for this review.
