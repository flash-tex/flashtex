# Real helper delivery comparison

Baseline helper main.rs from f2ea364 versus owned-wrapper main.rs/wire.rs from7956d37, built with identical dependencies in the same checkout. Exact executable hashes are in each summary; compiler is pinned4425 compiler executable documented in runtime baseline. No external service calls.

A temporary disk project/private ledger is opened by the real helper. The driver waits for initial compile, sends sequential full-source edits, reads durable acknowledgement and complete current-source preview, independently runs a clean compiler with matching identity/revision/source, and compares the complete JSON result. After every run it kills/reopens the helper and verifies the exact acknowledged document. Times run from immediately before request write through complete preview frame receipt, including helper pipes/polling/save/index/compile/wrapping/serialization and client handling of the earlier acknowledgement. They exclude final preview JSON decode and native paint. Clean oracle runs are outside measured intervals.

| Source target | Samples per version | Legacy median preview ms | Owned median preview ms | Median save/submit legacy / owned ms |
| --- | --- | --- | --- | --- |
|5KB|5|17.851|12.566|0.567 /0.554|
|50KB|10|141.457|94.910|5.363 /5.376|

All30samples exactly match clean compiler output; all4kill/reopen checks recover the exact acknowledged source. Summary explicitly marks lost-reply retry as unmeasured: these kills happen after acknowledgement. Existing stdio tests separately cover lost receipt/retry behavior. Shared Linux host, small sequential sample set, no claim about burst typing, native rendering or universal latency. Commander reported integration builds finished before these timing runs. Initial driver attempt failed before any edit because the private ledger directory did not exist; fixed by creating the required application-owned directory before launch.

Reproduce with examples/helper_replay.py --helper BINARY --compiler ORIGINAL_COMPILER --size50000 --edits10 (separate option/value with spaces). Use the same source generator, compiler and dependencies across baseline/candidate. The fixture writes only its temporary directories and caps response reads/timeouts and edit count. Source sizes are recorded exactly per sample rather than inferred from target labels.


## Lost acknowledgement recovery

Run helper_replay.py with --lost-reply to exercise an additional unmeasured failure scenario. After draining the initial document and preview, send a new valid full-source edit; wait for stdout readability without consuming the reply, then kill the helper. Reopen and verify exact source/hash and one revision advance. Resend the original stale request, require an error, and verify the recovered document remains exactly unchanged. This is source recovery plus stale-precondition rejection, not idempotent receipt replay for this plain edit operation.

Both5KB and50KB runs passed using the exact owned helper/compiler hashes recorded in lost-reply artifacts. They also include one clean-output comparison and normal acknowledged kill/reopen before the lost-reply phase. No native/provider inference is measured. The previous30-sample latency artifacts retain lost_reply_retry_measured=false because they predate this added scenario.
