# Independent proxy failure-path review

FT049r35 followup; pinned helper77c8cab1. Read-only review of
producer_capture_proxy.py, its five tests, and typing_burst.py reopen capture.
No peer edits, producer/helper workload rerun or independently executed tests.

The output pump preserves log-before-forward bytes and now routes read, capture,
hold and forwarding exceptions to process exit126. Its error record contains only
constant stage, exception class and errno; exception messages and paths are omitted.
A capture-write failure can leave a prefix even after a full producer readline,
and no uncaptured frame is forwarded. A forward-write failure retains the original
captured bytes even if forwarding itself was partial. These properties do not
retroactively identify the cause of failed run68918.

Proxy exit retains the pre-fork expected-parent/PDEATHSIG contract. The injected
EIO test checks proxy126 and producer absent-or-zombie terminal state; that is not
a claim that the proxy reaped the child after immediate exit. The owner reported
all five tests passing. Tests include byte/partial-EOF preservation, capture ENOSPC,
forward-write failure, injected read EIO/process termination and full stderr pipe.

The reviewed correction opens a FIFO diagnostic descriptor independently through
/proc/self/fd with O_NONBLOCK, so a full diagnostic pipe is not awaited and inherited
file-status flags are not changed. Other unsupported descriptor kinds are skipped.
Regular/character-device and sidecar I/O are still not a universal deadline or
all-filesystem-failure guarantee. Best-effort logging may produce no error record;
the explicit process failure remains distinct from a successful replay.

Reopen raw events and up-to1MiB stderr are retained with truncation and receiver
coverage metadata. Status explicitly says it is a pre-stop snapshot: late terminal
stderr can be missing. Cleanup attempts reopened.stop and event close independently
of capture-write failures. No claim is made that this yields full terminal stderr.

No remaining concrete review blocker was found within this bounded diagnostic
fix. Original failed capture and missing reopen/clean/provenance gates remain
unchanged. No runtime production path, source authority or native behavior changed
as part of this review.
