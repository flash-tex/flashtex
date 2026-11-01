# Negotiated helper burst evidence

One run per size/policy, same release helper ab945e6 and pinned e75741e compiler.
Twenty edits target 30 ms intervals. Actual send intervals and binary hashes are
recorded in each result. No native paint or LLM call is measured.

| Actual source size | Policy | Current | Historical | Final after last send |
| --- | --- | --- | --- | --- |
| ~5 KB | off | 20 | 0 | 6.06 ms |
| ~5 KB | on | 20 | 0 | 4.03 ms |
| ~52 KB | off | 2 | 0 | 79.44 ms |
| ~52 KB | on | 6 | 12 | 68.73 ms |

All 80 acknowledged edits in those four runs preserve exact source/revisions;
four final compiler results equal independent clean builds and four killed-helper
reopens recover exact final source. Every historical frame checks original token,
source revision, session/project, inactive source-action flags and monotonic display
ordering. These checks do not constitute independent clean comparison of every
historical result. Historical delivery lag is reported separately in the JSON.

The 52 KB enabled run's actual mean send interval was 37.45 ms versus 30.05 ms
disabled: sender/consumer/runtime contention and pipe backpressure affect the test.
Do not attribute the current-preview count difference or final latency solely to
negotiation, claim a stable speedup, or equate frames with native paints.

Both requested 500 KB runs failed before typing. The unchanged workload generator
produces 521,792 bytes; the valid original compiler response is 13,117,053 bytes
and 129 pages, above the helper's 12,582,912-byte compiler-frame limit. The driver
previously ignored the failure event until timeout; it now fails immediately.
The failed cases are preserved separately and excluded from passing burst counts.
No pages were removed and no source was reduced to make the case pass. Larger-frame
or streamed-output work is required before this case can participate in the gate.

## GH29 explicit larger-frame configuration

With `--compiler-frame-mib 15`, the unchanged requested500KB workload completes
in both policies: 40/40 durable acknowledgements, two exact independent clean final
results, and two exact killed-helper source reopens. The default compiler limit
remains8MiB. The helper allows an explicit15MiB input ceiling with1MiB typical
metadata reserve and still checks the entire encoded envelope against16MiB;
no fixed reserve guarantees fit for arbitrary metadata or JSON reserialization.
An overflow regression proves one small complete error followed by an intact ACK.

Enabled: one historical and one current frame, final1584.760ms after last send.
Disabled: one current, three stale and16superseded, final13664.010ms. Mean actual
send intervals30.052/30.386ms. These are single samples with an unexplained large
latency difference, not evidence of a stable causal speedup. Both exceed200ms.
The cap repair establishes delivery for this specific full-result case, not
responsive rendering, arbitrary-source-size support or native acceptance.

## Follow-up attribution, not a speed comparison

`--phases` enables source-free helper stderr timing records; the production default
is off and the JSONL wire is unchanged. Nonempty controller polls and request
handling/response serialization are measured separately. The driver additionally
records ACK latency, current runtime/controller totals and driver CPU versus wall.

One external-timing follow-up produced lastACK3380ms and final3983ms, with final
runtime499ms/controller561ms. One internal-phase follow-up produced lastACK973ms,
final1547ms and finalruntime441ms/controller517ms. All40ACKs, both clean finals and
both reopened sources are exact. Variation remains unexplained and large.

In the latter run, request handling totals931ms; edit handling grows from7ms to
76ms over20edits. Response serialization totals18ms. Several compiler polls consume
115–190ms each. Timings include startup snapshot/poll diagnostics where indicated
by the captured sequence. They locate both a request-service backlog and substantial
compiler response processing; they do not measure ledger internals separately.
Read-only inspection finds undo history retaining full before/after text, cloned,
validated and serialized on each durable update. This is a concrete candidate for
follow-up profiling/compact-history work, not yet proof of its isolated cost.
