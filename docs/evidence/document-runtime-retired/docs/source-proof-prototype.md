# Isolated source-proof prototype: retain current production ownership

The proposal49edf117 was tested in a disposable copy of runtime source
5319ab337a1d6af6054dd53bb07090ea602a63b75c0d17819faf5f411ecbf029.
Production runtime, helper and ledger files are unchanged. The reusable preparation
script copies the crate, mechanically derives the existing v1 validator with only
source-map type/access substitutions, and attaches a test-only proof module.
Fixtures and dependency-source hashes are archived beside the results.

Four correctness tests pass: exhaustive small ASCII/UTF8 ranges including empty,
end, four-byte scalar and combining codepoints; equal-length éa/aé mutation;
nine retained producer v1 captures and four siblings; and a real delayed Session
where stale A drains its sibling, queued B remains current despite failed larger
admission, and only B's candidate is delivered. That Session uses the original
owned Request, not the prototype proof. Sibling prototype checks cover hash/length
and altered hashes, not a complete replacement raw envelope validator. No new
route's duplicate/revision/order/lifecycle equivalence is claimed.

One paired release preparation observation followed correctness (24 samples each,
alternating order). Both paths include existing request validation/serialization
and encoded-buffer compaction. Owned path includes Request clone and retained
capacity compaction; proof path includes SHA256, boundary scan/bitmap, owned
identity/path metadata and allocation. Queue mutation/index/store work is excluded.
No end-to-end admission/native latency claim follows.

| Input | Owned preparation median ms | Proof preparation median ms | Owned accounted retained bytes | Proof accounted retained bytes |
|---|---:|---:|---:|---:|
|50KB ASCII|0.115|1.044|100331|50387|
|50KB UTF8|0.117|1.360|100333|56639|
|1MB ASCII|4.029|23.812|2000331|1000387|

Accounted retention includes inline structures, String/Vec capacities, bitmap/hash
and encoded request; excludes allocator headers, process RSS, ledger source and
other runtime state. The prototype hashes before dispatch, unlike the original
v1 path and later optional sibling hash. Moving that cost earlier is material.
The measured memory reduction does not justify adopting this implementation on
the typing path. Keep existing production ownership/API. Any future sharing or
faster-proof work requires a separately scoped measurement and complete raw/state
integration gates, not repetition of this same observation.

Reproduction: run `tools/source-proof-prototype/prepare.py` from this checkout.
It writes only `/home/natkarri/flashtex-source-proof-prototype`, links dependency
crates to the checkout and requires the recorded dependency source hashes/lockfile.
Set PROOF_CASES to that directory's cases.json and CARGO_TARGET_DIR to its target;
run cargo test --offline --manifest-path <scratch>/crates/document-runtime/Cargo.toml
--lib source_proof_prototype. The ignored paired_preparation_cost test additionally
needs PROOF_RESULTS and --release -- --ignored; do not run during other timings.
The archived results already satisfy this bounded experiment; no rerun is needed.
