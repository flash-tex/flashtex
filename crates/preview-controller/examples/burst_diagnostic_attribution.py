#!/usr/bin/env python3
"""Read existing archived evidence; no compiler execution or timing experiment."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path


def summarize(root):
    result = {}
    for mode in ['current', 'historical']:
        directory = root / mode
        manifest = json.loads((directory / 'archive.json').read_text())
        def read(name):
            entry = manifest[name]
            compressed = (directory / entry['path']).read_bytes()
            assert hashlib.sha256(compressed).hexdigest() == entry['sha256']
            raw = gzip.decompress(compressed)
            assert hashlib.sha256(raw).hexdigest() == entry['uncompressed_sha256']
            return raw
        diagnostics = json.loads(read('diagnostics.json'))
        provenance = json.loads(read('provenance.json'))
        final = json.loads(read('final-preview.json'))
        frames = read('events.jsonl').splitlines(keepends=True)
        historical = [f for f in frames if json.loads(f).get('payload', {}).get('kind') == 'completed_snapshot']
        phases = {}
        for phase, field in [('request', 'handling_ms'), ('request', 'response_serialization_ms'),
                             ('compiler_poll', 'duration_ms'), ('optional_output', 'serialization_ms')]:
            values = [d[field] for d in diagnostics if d['phase'] == phase]
            phases[phase + '.' + field] = dict(count=len(values), total_ms=sum(values),
                                              max_ms=max(values, default=None),
                                              min_ms=min(values, default=None))
        result[mode] = dict(
            diagnostic_scope='whole captured helper session, includes setup and post-measurement receipt checks',
            phases=phases,
            historical_frame_bytes=[len(f) for f in historical],
            historical_total_bytes=sum(map(len, historical)),
            final_runtime_total_ms=final['runtime_total_ms'],
            final_controller_total_ms=final['controller_total_ms'],
            final_post_last_send_ms=provenance['post_last_send_preview_ms'],
            optional_outcomes=[d['outcome'] for d in diagnostics if d['phase'] == 'optional_output'])
    result['limits'] = [
        'Request diagnostics have no request identity or absolute timestamp; cannot bind these samples to delayed ACKs.',
        'Optional serialization timing includes admission checks and queue offer; not pure JSON encoding.',
        'Compiler poll timing covers controller.poll only; excludes later optional envelope construction/serialization.',
        'No writer dequeue/start/flush durations or frame identities were captured.',
        'Receiver timestamps precede JSON decoding of that frame but follow handling of earlier frames; decode durations were not captured.',
        'Runtime/controller totals are nested scopes, not independent additive costs.',
        'No causal allocation of ACK or final-preview differences to serialization, writer backpressure, or receiver parsing is possible from this capture.'
    ]
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('archive', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    args.output.write_text(json.dumps(summarize(args.archive), indent=2) + '\n')
