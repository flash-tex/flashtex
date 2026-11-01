import argparse, hashlib, json, pathlib, selectors, subprocess, time
parser = argparse.ArgumentParser()
parser.add_argument("--binary", required=True)
parser.add_argument("--requests", required=True)
parser.add_argument("--expected", required=True)
parser.add_argument("--output", required=True)
a = parser.parse_args()
out = pathlib.Path(a.output); out.mkdir(exist_ok=True)
requests = pathlib.Path(a.requests).read_bytes().splitlines(keepends=True)
expected = pathlib.Path(a.expected).read_bytes().splitlines(keepends=True)
assert len(requests) == len(expected)
process = subprocess.Popen([a.binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
rows = []; replies = []
try:
    for request, reference in zip(requests, expected):
        identity = json.loads(request)["id"]
        start = time.perf_counter()
        process.stdin.write(request); process.stdin.flush()
        selector = selectors.DefaultSelector(); selector.register(process.stdout, selectors.EVENT_READ)
        assert selector.select(30), "response timeout"
        selector.close()
        reply = process.stdout.readline()
        elapsed = (time.perf_counter() - start) * 1000
        assert json.loads(reply) == json.loads(reference), identity
        replies.append(reply)
        rows.append({"id": identity, "elapsed_ms": elapsed, "request_bytes": len(request), "reply_bytes": len(reply)})
finally:
    process.stdin.close()
    code = process.wait(timeout=10)
    assert code == 0
    (out / "stderr.txt").write_bytes(process.stderr.read())
(out / "response.jsonl").write_bytes(b"".join(replies))
(out / "result.json").write_text(json.dumps({"binary_sha256": hashlib.sha256(pathlib.Path(a.binary).read_bytes()).hexdigest(), "all_replies_equal_expected": True, "rows": rows}, indent=2) + "\n")
print(json.dumps(rows))
