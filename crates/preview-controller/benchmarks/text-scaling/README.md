# Text candidate compiler scaling probe

One persistent process, one giant ASCII text math box, 5KB/50KB/500KB bodies.
Each size has one initial compile followed by three single-character changes.
Timing covers writing the full request through reading the complete JSON reply;
it excludes native rendering. This synthetic unbreakable text box is not a
representative document, and three edit samples do not establish p95 or p99.

Release changed-compile ranges: 5KB 1.32–1.81ms, 50KB 13.94–19.06ms,
500KB 161.69–170.93ms. Initial times: 2.68/17.31/192.06ms. Debug changed
ranges were 57.84–70.80/169.23–218.24/1115.22–1400.98ms. Builds were
measured sequentially under ambient host load, not an isolated paired benchmark.
All twelve release replies equal the earlier debug replies as complete JSON
values; the returned text is not omitted or truncated. No code optimization was
introduced between builds. The large case leaves little latency for rendering;
this does not certify the product's under-200ms typing-to-preview requirement.

Exact requests and both response streams are losslessly gzip-compressed, alongside
binary/artifact hashes, build log and a reusable replay script. Decompress the
requests and debug responses, then run replay.py with --binary, --requests,
--expected and --output. It compares every reply with the expected complete value.
The debug evidence retains original capture paths/hashes; local compressed files
have their own hashes in evidence.json. No paid APIs were used.
