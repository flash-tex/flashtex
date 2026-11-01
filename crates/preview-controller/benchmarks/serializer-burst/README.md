# Full-output serializer through the real helper during typing

Same current release helper (source f72c44c plus unchanged examples/docs through c07f7ba), same helper_burst driver/source, same20edits at30ms intervals, historical snapshots DISABLED. Candidate compiler e75741e957b27482127f01abddc703666a0b445a was run first, then4425baseline. Exact helper/compiler hashes, source bytes and actual send intervals are in both artifacts. The compiler candidate preserves full output; preceding301page byte-equality audit is in document-runtime/benchmarks/serializer-e75741e.

| Compiler | Durable ACKs | Current previews | Stale / superseded | Exact final after last send |
| --- | --- | --- | --- | --- |
|4425baseline|20/20|1|14 /5|90.477ms|
|e757serializer-only|20/20|12|8 /0|43.166ms|

Both final JSON results equal a fresh compiler build, and both kill/reopens preserve the exact final source. These are real helper stdout results, not historical snapshots relabeled current. The candidate still drops8stale results during typing, so this does not prove an update for every keystroke or arbitrary-document latency. One sequential run per compiler on shared Linux; no native paint/referencePDF timing claim. Actual source length is52,210bytes.

Reproduce examples/helper_burst.py --helper HELPER --compiler COMPILER --size 50000 --edits 20 --interval-ms 30 for each pinnedcompiler. The default helper protocol and durable-save behavior are unchanged. No new implementation edits are part of this checkpoint.
