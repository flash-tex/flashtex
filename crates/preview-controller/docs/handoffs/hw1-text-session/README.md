# Text edits through the actual compiler protocol

Ten successive requests use the preserved comment-corrected candidate binary
from 61bd63f7. Each complete response equals a fresh-process response to the same
request: pages, diagnostics, status, source spans and identity are all compared.
No normalized subset is substituted for the protocol reply.

The sequence covers valid text, an unsupported nested command twice, an unclosed
argument, a missing opener, spaces, escaped braces, a comment before the opener,
ordinary math and restored text. All four invalid revisions retain recovered
status and diagnostics; the six valid revisions are ok without diagnostics.
This confirms repeated invalid-source cache hits do not erase these errors and
subsequent valid edits clear them. No producer, native rendering or timing claim.

The initial harness incorrectly sent string runtime-v1 as protocol_version; the
worker correctly refused it. Those rejected inputs/replies are archived separately
as invalid-version files. The acceptance run uses the required integer 1. No
compiler source changed between runs. Reproduce the session by piping request.jsonl
to the recorded binary; run each request in a new process for the fresh comparison.
