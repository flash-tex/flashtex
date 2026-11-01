# Independent corrected correctness acceptance

FT049r39; exact capture7a7fd4f26290c2ee41115734631991d2557672bd.
No helper/producer workload was rerun. This audit separately reports recorded
correctness acceptance and incomplete live diagnostic trace.

```sh
python3 crates/document-runtime/tools/audit_instrumented_prefix.py --corrected
```

Verified24 archived compressed/raw hashes, captured scripts, normal provenance
artifact hashes and78 local asset hashes. Independently rehashed the actual binaries:

- helper `/home/natkarri/flashtex-preview-performance/crates/preview-controller/target/release/flashtex-preview-controller`:
  `29810763137daf83156e800ffa06491baa95d5d04fda524afc530fd448ea7763`
- producer `/home/natkarri/flashtex-producer-release-artifacts/65dbe7d/target/release/flashtex-render`:
  `1587245d9d68f426678176e45c0e0a4a971cd252c64d3a288147861ec16d62dd`

Twenty guarded UTF8 edits and ordered ACKs match exact source fingerprints.
Original producer requests/replies bind the five delivered historical generations
2,4,12,14,18 and sole current21. Historical tokens and disabled current/source-action
flags agree with their original source snapshots. Old command receipts preserve
command2/current21 and21/current21, permanent IDs20, and conflicting reuse is refused.

The separately captured reopen returns exact durable revision21/text/SHA and a
preview matching its original producer reply. The clean comparison input is an
exact captured request line; its full reply equals final current output. These
recorded correctness gates pass independently of telemetry completeness.

The diagnostic status explicitly says captured:true, partial_tail:true,
truncated:false, non_json:false. Complete raw lines exactly match diagnostics.json;
the remaining partial tail is preserved. At least one received output sequence
lacks a complete write_finished record, independently detected and listed in
benchmarks/positional-correctness-review/audit.json. The strict trace refusal is
preserved. No terminal event is invented and no complete writer/receiver attribution
is claimed. Helper and receiver clocks are not subtracted.

This is one corrected correctness acceptance, not a performance comparison or
proof of58242/68918 causes. Both failed captures remain unchanged. Further telemetry
work was paused by Commander; no post-stop implementation or extra run was made.

Validation: corrected and original failed-prefix artifact audit modes pass;
original failed-prefix JSON remains unchanged; git diff --check passes. No runtime
production changes or redundant Rust test suite runs.
