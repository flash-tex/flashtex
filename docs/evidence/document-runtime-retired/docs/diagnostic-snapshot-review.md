# Independent positional diagnostic snapshot review

FT049r38; exact helper e693368b. Read-only inspection of helper_replay.py,
typing_burst.py and four deterministic snapshot tests. No workload rerun,
independent test execution or runtime production changes.

The previous seek/read moves the open-file-description offset shared with the
helper stderr writer. diagnostic_bytes now uses bounded positional pread from0,
retaining at most1MiB+1 bytes without seeking that shared offset. Raw bytes and
status are stored before parsing. The dupfd test checks an existing writer offset3
is unchanged, then verifies an EOF append produces the exact two original lines.
The owner reported all four tests passing; this review does not relabel that as an
independent test run.

Only newline-complete records are parsed. An incomplete tail is retained verbatim
and flagged; complete malformed JSON still raises with non_json status. Over-cap
input is retained in bounded memory and refused, with truncation explicit. The
harness finally persists raw/status on parsing failure, while reopened diagnostics
also use positional reads. These are live pre-stop snapshots, not complete terminal
stderr. If failure occurs before diagnostics() is called, initialized empty raw
and empty status do not establish that stderr was actually empty.

The published58242 failure remains failed_diagnostic_snapshot_before_reopen. Its
raw stderr was not saved; reopened validation, clean comparison and normal
successful-run provenance are missing. This review checked the compressed/raw hash
of the archived producer error sidecar, which records forward_write,
BrokenPipeError, errno32. It has no timestamp establishing whether that happened
before diagnostics parsing failed or after helper cleanup closed the pipe.
Neither the sidecar nor the deterministic offset test proves58242 or68918 cause.
No passing capture or performance comparison is inferred.

No remaining concrete review blocker was found in the bounded snapshot repair.
Validation here consisted of exact-source review, sidecar hash verification and
git diff --check; all original failed-run limitations remain explicit.

Followup e2072fe3 was also reviewed read-only: initial status now explicitly says
captured:false/no snapshot, while successful pread sets captured:true. This resolves
the absent-versus-empty caveat above without changing snapshot bounds or parsing.
No remaining concrete review blocker; no new workload was run by this reviewer.
