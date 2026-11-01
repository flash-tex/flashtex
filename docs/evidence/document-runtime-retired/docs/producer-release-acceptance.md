# Pinned original producer release acceptance

The original producer commit `65dbe7da7a182e99322070e2c9763cc3b69a342b` was built
with `cargo build --offline --locked --release --bin flashtex-render` in a separate
artifact target. All 355 archived source files were checked before and after the
build; no producer or vendored dependency source changed. The archived lockfile
pins registry dependencies; Rust toolchain/standard-library inputs are recorded
separately. The existing debug executable and its SHA remain preserved.

Release binary:
`/home/natkarri/flashtex-producer-release-artifacts/65dbe7d/target/release/flashtex-render`

SHA256: `1587245d9d68f426678176e45c0e0a4a971cd252c64d3a288147861ec16d62dd`

The build uses a separate `CARGO_TARGET_DIR`, disabled incremental compilation and
release debug information, with Rust flags overrides removed. Exact command,
environment, toolchain, archive/lock hashes, registry checksums and build log are
preserved in `../benchmarks/producer-release-65dbe7d`. The release binary itself is
not copied into Git. The artifact's absolute source paths may affect reproducible
binary hashes on another machine; no universal cross-machine binary claim is made.

The helper owner captured actual producer stdin/stdout in `53c0c71d`, extending
the earlier typing harness. Independent validation checked compressed/original
capture hashes and all declared assets before replay. Four captured sessions
contain seven actual requests (initial edit, current edit, retry and reopened
source). Every complete debug and release stdout byte stream exactly matches its
captured counterpart. Full reply evidence is compressed alongside the manifest.
The full/group edited compiler requests are the same actual captured bytes;
this is stronger than the earlier modeled-source comparison, which is preserved
with its original limitations in `../benchmarks/typing-command-review`.

The standalone `release-manifest.json` includes `binary_sha256` and pinned `assets`
for the existing helper harness. No compiler rebuild or resource substitution is
needed by that consumer. No calibrated speedup, release regression, native paint,
pixel parity or broad compatibility claim follows from this equality test.
