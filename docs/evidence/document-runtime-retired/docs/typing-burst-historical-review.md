# Historical burst: independent recorded-byte audit

FT049r27, exact helper capture `8d913d2a3db241417c071709ca7c9fa857778fd8`. This is a read-only audit
of published bytes; no throughput workload or producer process was repeated.

```sh
python3 crates/document-runtime/tools/audit_typing_burst.py --historical
```

Verified 14 compressed/raw artifacts and 78
locally pinned assets. All20 guarded UTF8 edits, ACKs, durable IDs, old-command retry
revisions and final clean request/result checks pass using the same audit as the
current-only capture. Missing local assets fail explicitly.

Seven historical revisions2,4,7,10,12,15,18 each bind to one exact original request
and full producer result in the same proxy stream. Result ID, compile revision,
project identity, exact source bytes/hash and the command's `burst-N` token agree.
Every historical envelope has `is_current:false`, `source_actions_enabled:false`
and a compile generation older than the declared current generation. Historical
generations increase; their sequence matches every intermediate producer completion.
Twelve other edited generations have matching superseded notifications. Revision21
is the sole current preview. Original initial1, postmeasurement retry22 and restarted
producer1 are accounted separately.

Opt-in enable and disable receipts are explicit; disable precedes old-command
retries. Final21 captured request bytes match the clean input, and complete parsed
producer/clean/helper results agree. No historical result is treated as current or
as authority for source actions.

The target was30ms between sends, but maximum recorded lateness is
23.908608ms. Input used a Python thread sharing its
process with large result decoding. This is a measurement limitation, not proof of
causality or a valid fixed-cadence performance comparison. No native rendering,
continuous current-preview or pixel-fidelity claim follows from these records.

The later upstream `verify_historical_results` correlation correction was audited
in published8d913d2a separately from the original captured harness. It now checks
result ID/revision and unique same-stream matching; original capture provenance
was preserved. Our audit independently rechecks actual bytes with those bindings.

Validation: both current and historical audit modes pass, and current audit output
is unchanged. No Rust production code changed; no redundant runtime suite run.
