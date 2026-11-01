# Exact producer full-output acceptance

FT049r41. Producer9aaec57a019c6a0073419eeb3ec90f922f5b367c is the published checkpoint
above code7c12dee4. Reviewed changes share font hints/Rc strings and optimize the
full v1 decimal/string writer, with existing assembled-item/adapter caches retained.
No page/delta protocol changes were imported.

The standalone render-pipeline archive was extracted under
`/home/natkarri/flashtex-producer-release-artifacts/9aaec57a/source`.
All378 archived files were verified before and after the unchanged offline locked
release build. All15 path dependencies remain inside that tree; eight vendor PINs
and21 registry package/checksum entries are recorded. The lockfile, source archive,
compiler toolchain, command and build log are preserved. No source or vendor edits,
network dependency downloads, /tmp outputs or old binary replacement were needed.

Built binary:
`/home/natkarri/flashtex-producer-release-artifacts/9aaec57a/target/release/flashtex-render`
SHA256347429f433bb70b7483a9f622917e54f49c3668a7f3a746221c132cff5f5434a.
Compatible downstream manifest: the same root's `manifest.json`, containing exact
producer_sha, binary_sha256 and78 verified font/TFM/license asset entries.

The ignored integration test full_output_peer.rs consumes existing fixtures only:
three large-document states from display-multipage-requests.json and six original
three-document/bibliography requests from the corrected helper capture. It compares
all v1 fields and complete raw v2 siblings between fresh and persistent processes
through the existing runtime. Request/project/revision and runtime source-binding
checks remain active. No old-version output equality or fixed old geometry is used.

All18 default/lowered-cap pairs passed. The large cases retain26 pages and the final
sentinel; full page/item arrays and diagnostics match. Their recovered status retains
overfull_hbox and display_list_declined diagnostics. The six default multidocument
cases are ok; requested generations2/4/5/6 deliver source-bound v2, while legacy1/3
remain v1-only. At2500 bytes the requested cases retain v1 with explicit v2 decline.
One existing small request was additionally tested under a1-byte limit: its compact
failure has status failed and no candidate, distinct from decline. Its uncapped
control also passes; this adds two pairs without introducing another source fixture.

Direct producer stdout is byte-identical fresh/persistent for all nine default
requests, exercising the optimized full writer without Value normalization. Parsed
direct replies equal runtime-captured replies. Every delivered v2 font SHA/length
matches a supplied asset; all declared document byte lengths/hashes and compile
revisions match the original source. Checked33,429 source spans against exact UTF8
byte boundaries across all returned pages and resources. Compiler revision remains
distinct from the helper's individual editor revisions.

Full fresh/persistent replies, raw stdout, request bytes, build/resource provenance,
case manifests and checks are archived in benchmarks/full-output-9aaec57a with
compressed/original SHA256 mappings. Font bytes and binaries are not copied into
Git. For reproduction, decompress the archived cases.json, provide the verified
binary/assets via replay-plan.json environment, then run:

```sh
cargo test --offline --manifest-path crates/document-runtime/Cargo.toml --test full_output_peer -- --ignored
```

The dedicated failure-cases.json uses the same test and a separate output directory.
The archived commands retain exact paths used on this host. Focused strict Clippy
and formatting/diff checks passed. No calibrated timing was recorded; test/build
elapsed times are not performance evidence. This proves same-version transport and
cache equivalence on these cases, not independent typography, arbitrary-source
compatibility, native rendering or sub200ms responsiveness.
