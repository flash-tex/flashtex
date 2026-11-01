# Edit-to-compile correlation review

Status: bounded full/metadata edit implementation published3014ade2, based on the review against helper f8bb2389. Native adoption remains pending. The analysis below records the design; history/group/apply correlation remains separately scoped.

Current full and metadata edit results return durable document revision/hash,
preview_error and save_and_submit_ms, but no admitted compile identity. Controller
compile_current builds preview-N and only advances generation / submitted after
runtime submission succeeds. Runtime success means queue admission, not process
execution or completion; a queued request may later be superseded. Never derive
N from the document revision, the JSON request id, or by adding one to a counter.

Proposed additive result fields: compile_request_id and compile_revision, both
nullable. Populate them from a typed submission receipt captured only after this
operation's successful compile_current admission. On index/submission failure,
return both null alongside the existing preview_error and durable document. Do
not return the previous successful submission's identity. Invalid edits remain
errors rather than fabricated save acknowledgements. A same-command retry that
admits a fresh compile should report the fresh admission while retaining its
existing durable command_revision and replayed_command semantics.

Prefer a shared internal submission result from finish_saved_index, threaded into
full EditOutcome and metadata outcomes. Do not parse preview-N or expose mutable
internal submitted tuples. Preserve the public compile_current compatibility
wrapper if consumers rely on Result<(), String>. Cover full/metadata edit first;
review grouped/history/apply paths before promising identical correlation there.

Stale already carries request_id and compile_revision. Superseded has request_id
and by_id; cancelled/failed have request_id. Discarded currently loses the revision
from its Event::Preview despite having it in scope: retaining that revision in
Update::Discarded is a direct additive correction, with no guessed lookup table.
Native correlation should key session_id + request_id, and retain project identity
from its session binding. None of these fields grants current source actions;
existing exact source hashes/revisions/currentness checks remain authoritative.

Acceptance: actual child-held first compile followed by multiple edits proves
ACK-to-admission mapping, supersession and later preview mapping. Compiler absent
and queue/encoding refusal prove saved document plus null compile identity.
Full and metadata ACKs agree on semantics; source revision differs from compile
generation after a restart or configuration-triggered compile. Discarded events
retain original event revision. Replay durable command tests retain receipt
identity separately from any new compile. No latency or paint claim follows from
admission timestamps or acknowledgement receipt.

Validation follow-up: actual stdio helper plus child producer test confirms both full and metadata edit ACK identities equal their later preview identities. An extra compile before editing deliberately separates durable source revision from compile generation. Existing held-child lifecycle test covers queued supersession; offline and encoding-refusal tests retain saved source with null admission.

Grouped extension published ac0a860e: apply_group full/metadata ACKs now carry the same typed receipt. Actual held-child tests distinguish permanent command revision2, current source revision3 and fresh retry compile identity. A subsequent encoding-refusal test covers both modes and exact retries: source stays durable at revision2, receipt replay remains true, no compile identity is fabricated, and reopened source matches. Undo/redo and reviewed apply wire correlation remain outside this extension.
