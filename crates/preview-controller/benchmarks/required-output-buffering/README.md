# Required output batching

Input is the current source-revision3 preview event from
`../display-helper-raw-multipage/step-2.json.gz`, compactly encoded with Python
json.dumps separators(',', ':'). Its hash and the measured probe source/executable
are recorded. Output is1,746,207bytes including newline. Ten alternating pairs
preserve every byte and exact newline-inclusive/one-byte-too-small limits.

Direct encoding measured5.604–8.396ms;8KiB-buffered encoding2.606–4.030ms.
These are isolated same-process observations, not native paint or total latency.
The extra raw preencoding observations emitted by the shared probe are not used
to justify required-output behavior; required responses remain ordinary Value.

Production required and optional routes now use the same checked buffered frame
serializer. An oversized required result falls back to the existing small error
with original request/session identity and durable-source warning. The next required
ACK remains ordered and intact. If even the error cannot fit, the existing stopped
state is set. No partial buffer is admitted. Only8KiB scratch storage is added;
existing output limits and queue capacity remain unchanged.
