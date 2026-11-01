# Actual helper receipt scaling (one run)

Same pinned producer/assets and release helper as display-helper-65dbe7d. Repeat
`Office AV fi.` 100 times, then apply the harness's two exact source expansions.
Source sizes are 1,462 / 2,562 / 3,762 UTF-8 bytes. Candidate receipt is
35.46 / 65.65 / 132.32 ms; v1 receipt 26.95 / 51.14 / 100.86 ms; ACK receipt
0.24 / 0.38 / 0.39 ms. Each state exactly equals fresh direct producer v1/display
JSON. All diagnostics are empty; final durable document reopens exactly.

These are single uncontrolled-host request-to-Python-receipt samples, not native
paint, a continuous-typing benchmark, percentiles, or evidence of arbitrary-document
sub-200 ms performance. Direct comparison runs occur after each receipt measurement.
Full source and outputs are retained as deterministic gzip JSON (decompress before
inspection); provenance hashes identify compressed files. Compression is packaging
only and does not change protocol outputs. Reproduce with the replay harness using
`--repeat 100 --compress-artifacts` and the same pinned binary/assets.
