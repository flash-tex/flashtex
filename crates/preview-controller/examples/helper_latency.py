#!/usr/bin/env python3
"""Measure sequential edit admission -> observed positioned result, never native paint."""
import argparse
import hashlib
import json
import math
import pathlib
import queue
import subprocess
import tempfile
import threading
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--helper', required=True, type=pathlib.Path)
    parser.add_argument('--compiler', required=True, type=pathlib.Path)
    parser.add_argument('--edits', type=int, default=100)
    args = parser.parse_args()
    if not 1 <= args.edits <= 200:
        parser.error('edits must be 1..200 (below durable history capacity)')
    helper, compiler = args.helper.resolve(), args.compiler.resolve()
    with tempfile.TemporaryDirectory(prefix='flashtex-latency-') as temporary:
        root = pathlib.Path(temporary)
        project, private = root / 'project', root / 'private'
        project.mkdir()
        private.mkdir()
        (project / 'main.tex').write_text('Initial source.', encoding='utf-8')
        config = root / 'config.json'
        config.write_text(json.dumps(dict(session_id='benchmark', project_id='benchmark',
            entry_path='main.tex', project_root=str(project), private_ledger_root=str(private),
            compiler_path=str(compiler))), encoding='utf-8')
        with (root / 'stderr.log').open('w+') as errors:
            child = subprocess.Popen([str(helper), str(config)], stdin=subprocess.PIPE,
                                     stdout=subprocess.PIPE, stderr=errors, text=True)
            events = queue.Queue()
            def read():
                try:
                    for line in child.stdout:
                        events.put((time.perf_counter(), json.loads(line)))
                finally:
                    events.put((time.perf_counter(), None))
            reader = threading.Thread(target=read, daemon=True)
            reader.start()
            def receive():
                stamp, event = events.get(timeout=10)
                if event is None:
                    raise RuntimeError('helper exited before expected event')
                if event['type'] == 'error':
                    raise RuntimeError(event['payload'])
                return stamp, event
            def send(identity, kind, payload):
                child.stdin.write(json.dumps(dict(protocol_version=1, session_id='benchmark',
                    id=identity, type=kind, payload=payload)) + '\n')
                child.stdin.flush()
            samples = []
            try:
                _, ready = receive()
                if ready['type'] != 'ready' or ready['payload']['compiler_error']:
                    raise RuntimeError(ready)
                send('get', 'document', dict(path='main.tex'))
                while True:
                    _, event = receive()
                    if event['id'] == 'get':
                        document = event['payload']['document']
                        break
                for index in range(args.edits):
                    text = ('This is a reproducible editor latency paragraph.\n\n' * 20) + f'Edit {index}.'
                    started = time.perf_counter()
                    send(f'edit-{index}', 'edit', dict(path='main.tex',
                        expected_revision=document['revision'], expected_sha256=document['source_sha256'], text=text))
                    expected = document['revision'] + 1
                    saved, preview = False, None
                    while not saved or preview is None:
                        observed, event = receive()
                        if event['id'] == f'edit-{index}':
                            if event['payload']['preview_error']:
                                raise RuntimeError(event['payload']['preview_error'])
                            document = event['payload']['document']
                            saved = True
                        if event['type'] == 'update':
                            payload = event['payload']
                            if payload['kind'] == 'failed':
                                raise RuntimeError(payload)
                            if payload['kind'] == 'preview' and payload['source_versions']['main.tex'] == expected:
                                if payload['result']['payload']['status'] != 'ok':
                                    raise RuntimeError('benchmark source did not compile cleanly')
                                preview = (observed - started) * 1000
                    samples.append(preview)
                ordered = sorted(samples)
                percentile = lambda p: ordered[max(0, math.ceil(len(ordered) * p) - 1)]
                print(json.dumps(dict(edits=len(samples), p50_ms=percentile(.50), p95_ms=percentile(.95),
                    p99_ms=percentile(.99), max_ms=max(samples),
                    helper_sha256=hashlib.sha256(helper.read_bytes()).hexdigest(),
                    compiler_sha256=hashlib.sha256(compiler.read_bytes()).hexdigest(),
                    workload='sequential 20-paragraph edits; one file; no concurrent edits',
                    measurement='client write through received positioned result; includes durable save',
                    native_paint_measured=False, reference_pdf_measured=False)))
            finally:
                child.kill()
                child.wait(timeout=3)
                reader.join(timeout=3)
                child.stdin.close()
                child.stdout.close()


if __name__ == '__main__':
    main()
