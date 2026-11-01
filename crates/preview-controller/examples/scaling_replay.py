#!/usr/bin/env python3
"""Pinned multi-file scaling probe; persistent/fresh equality is not TeX/PDF parity."""
import argparse
import hashlib
import json
import pathlib
import subprocess


def digest(data):
    return hashlib.sha256(data).hexdigest()


def workload(size, edits):
    main = '\\documentclass{article}\n\\begin{document}\n\\input{one}\n\\input{two}\n\\end{document}\n'
    available = size - len(main.encode())
    paragraph = 'Measured paragraph with ordinary words and spaces.\n\n'
    def fill(length):
        return (paragraph * (length // len(paragraph) + 1))[:length]
    texts = {'main.tex': main, 'one.tex': fill(available // 2),
             'two.tex': fill(available - available // 2)}
    revisions = {path: 1 for path in texts}
    records = []
    for edit in range(edits):
        path = 'one.tex' if edit % 2 == 0 else 'two.tex'
        # Alternate a fixed-width edit near the beginning and the end of includes.
        offset = 0 if edit % 4 < 2 else max(0, len(texts[path]) - 10)
        value = chr(ord('A') + edit % 26)
        texts[path] = texts[path][:offset] + value + texts[path][offset + 1:]
        revisions[path] += 1
        request = dict(protocol_version=1, type='compile', id=f'edit-{edit}',
                       payload=dict(project_id=f'scaling-{size}', revision=edit + 1,
                                    entry_path='main.tex', documents=[
                                        dict(path=p, revision=revisions[p], text=t)
                                        for p, t in texts.items()]))
        records.append(json.dumps(request, separators=(',', ':')))
    return ('\n'.join(records) + '\n').encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler', required=True, type=pathlib.Path)
    parser.add_argument('--replay', required=True, type=pathlib.Path)
    parser.add_argument('--output', required=True, type=pathlib.Path)
    parser.add_argument('--edits', type=int, default=20)
    parser.add_argument('--max-frame-bytes', type=int, default=8 * 1024 * 1024)
    parser.add_argument('--compiler-source-sha', help='Caller-verified source commit; binary hash is always recorded')
    args = parser.parse_args()
    if not 1 <= args.edits <= 100:
        parser.error('edits must be 1..100')
    if not 128 <= args.max_frame_bytes <= 64 * 1024 * 1024:
        parser.error('max-frame-bytes must be 128..67108864')
    args.output.mkdir(parents=True, exist_ok=True)
    compiler, replay = args.compiler.resolve(), args.replay.resolve()
    results = []
    for size in (5000, 50000, 500000):
        source = workload(size, args.edits)
        stem = args.output / str(size)
        stem.with_suffix('.jsonl').write_bytes(source)
        try:
            run = subprocess.run([str(replay), str(compiler), str(args.max_frame_bytes)], input=source,
                                 capture_output=True, timeout=120)
            stdout, stderr, code = run.stdout, run.stderr, run.returncode
        except subprocess.TimeoutExpired as error:
            stdout, stderr, code = error.stdout or b'', error.stderr or b'', None
        stem.with_suffix('.stdout.jsonl').write_bytes(stdout)
        stem.with_suffix('.stderr.txt').write_bytes(stderr)
        events = [json.loads(line) for line in stdout.splitlines()]
        samples = [event for event in events if event.get('type') == 'sample']
        summary = next((event for event in events if event.get('type') == 'summary'), None)
        failure = stderr.decode('utf-8', errors='replace')[-4096:]
        result = dict(failure_detail=failure, source_bytes=size, edits=args.edits, input_sha256=digest(source),
                      returncode=code, completed_samples=len(samples), summary=summary,
                      all_compiled_ok=bool(samples) and all(s.get('compiler_status') == 'ok' for s in samples),
                      classification='timeout' if code is None else
                          'output_validation_failure' if b'output malformed, truncated or oversized' in stderr else
                          'process_failure' if code != 0 and not any(s.get('exact_json_equal') is False for s in samples) else
                          'persistent_fresh_mismatch' if any(s.get('exact_json_equal') is False for s in samples) else
                          'complete' if summary and len(samples) == args.edits else 'incomplete',
                      mismatch_classes=sorted({s.get('mismatch_class', 'unknown') for s in samples}))
        results.append(result)
        print(json.dumps(result), flush=True)
    report = dict(max_frame_bytes=args.max_frame_bytes, compiler_source_sha=args.compiler_source_sha, compiler_sha256=digest(compiler.read_bytes()), replay_sha256=digest(replay.read_bytes()),
                  generator_sha256=digest(pathlib.Path(__file__).read_bytes()), cases=results,
                  native_paint_measured=False, reference_pdf_measured=False,
                  measurement='Rust runtime submission to received positioned result; excludes editor save and native paint')
    (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
