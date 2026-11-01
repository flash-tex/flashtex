# Serializer-only compiler comparison

Candidate e75741e957b27482127f01abddc703666a0b445a ports the direct-page serializer onto current main WITHOUT the upstream trailing-page truncation policy. Baseline is compiler4425d9d77b747db69c1f04c493b363bd3a467f4e in its documented combined build. Exact executable hashes are recorded in the equality and replay reports.

First gate: stream the same20 alternating500KB edits through each persistent compiler. All20 replies are byte-identical AND JSON-identical, all preserve301pages and statusok. cross-compiler-equality.json records every reply byte count/page count/status and immutable input hash. Raw streams remain local in /tmp/flashtex-serializer-crosscheck; they are not checked in because they total roughly400MB. This compares two original FlashTeX compilers, not established TeX/PDF visual parity.

Then use the exact same current release runtime replay executable for5/50/500KB,20edits each, for both compilers. All120samples match their respective clean builds. Explicit16MiB experimental frame limit; production default unchanged. Median total from submit through validated positioned result:

| Source | Baseline p50 | Candidate p50 | Baseline p95 | Candidate p95 |
| --- | --- | --- | --- | --- |
|5KB|2.702ms|2.774ms|4.411ms|3.457ms|
|50KB|25.410ms|15.023ms|34.463ms|22.405ms|
|500KB|278.224ms|153.309ms|2047.720ms|843.098ms|

500KB median dispatch-to-first-byte204.315→89.242ms, parse61.444→54.633ms, validation12.654→11.264ms. First-byte interval includes compiler/input/scheduling, not pure compiler CPU. Timing spikes affected multiple phases: baseline worst2267.624ms included584ms parse; candidate worst879.197ms included304ms parse. Cause is not established. The commander reported no overlapping integration builds, and measured host load afterward was2.71/2.61/2.68; neither proves a quiet or deterministic environment. Preserve these tails rather than describing the candidate as universally sub200ms or attributing every difference to compiler code. Native delivery/painting is excluded.

Reproduction uses the existing preview-controller/examples/scaling_replay.py, same immutable inputs/hashes, --max-frame-bytes16777216 (with a space between option and value), and the pinned runtime/compiler binaries. Candidate run's source label is abbreviatede75741e; provenance.json resolves it to the full verified commit. No new implementation changes in this evidence checkpoint.
