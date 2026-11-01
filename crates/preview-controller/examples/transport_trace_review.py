#!/usr/bin/env python3
"""Correlate one original capture by writer order; never subtract clock origins."""
import argparse
import json
from pathlib import Path


def review(root):
    diagnostics = json.loads((root/'diagnostics.json').read_text())
    receiver = json.loads((root/'receiver-timings.json').read_text())
    assert receiver['dropped'] == 0, 'receiver timing coverage truncated'
    records = receiver['records']
    frames = {}
    for record in diagnostics:
        if record['phase'] == 'output_frame':
            frames.setdefault(record['sequence'], []).append(record)
    known={'admitted','refused_full','refused_disconnected','refused_busy_or_epoch',
           'evicted_by_required','evicted_by_reset','replaced_optional','dequeued',
           'write_started','write_finished','write_failed'}
    for sequence, lifecycle in frames.items():
        first=lifecycle[0]
        assert isinstance(sequence,int) and sequence>0
        assert all(all(r[key]==first[key] for key in
            ['class','bytes','compile_revision','admission_attempt_ms']) for r in lifecycle)
        outcomes=[r['outcome'] for r in lifecycle]
        assert set(outcomes)<=known and len(outcomes)==len(set(outcomes)), 'unknown or duplicate lifecycle event'
        terminal=[v for v in outcomes if v.startswith(('refused_','evicted_','replaced_'))]
        assert len(terminal)<=1
        if terminal:
            assert not set(outcomes)&{'dequeued','write_started','write_finished','write_failed'}
        assert not {'write_finished','write_failed'}<=set(outcomes)
        if set(outcomes)&{'write_finished','write_failed'}:assert 'write_started' in outcomes
        if 'write_started' in outcomes:assert 'dequeued' in outcomes
    starts = sorted((r for rs in frames.values() for r in rs if r['outcome']=='write_started'),
                    key=lambda r:r['at_ms'])
    assert len(starts) >= len(records), 'writer trace does not cover every observed receipt'
    rows = []
    for ordinal, (start, received) in enumerate(zip(starts, records), 1):
        assert received['sequence']==ordinal
        assert start['bytes']==received['bytes'], 'writer/receiver order or length mismatch'
        same = frames[start['sequence']]
        assert all(r['bytes']==start['bytes'] and r['class']==start['class'] and
                   r['compile_revision']==start['compile_revision'] for r in same)
        finish = [r for r in same if r['outcome']=='write_finished']
        dequeue = [r for r in same if r['outcome']=='dequeued']
        assert len(finish)==len(dequeue)==1, 'received frame lacks complete writer trace'
        assert finish[0]['at_ms']>=start['at_ms']>=dequeue[0]['at_ms']>=start['admission_attempt_ms']
        rows.append(dict(receiver_ordinal=ordinal, output_sequence=start['sequence'],
            frame_class=start['class'], compile_revision=start['compile_revision'], bytes=start['bytes'],
            admission_to_dequeue_ms=dequeue[0]['at_ms']-start['admission_attempt_ms'],
            write_ms=finish[0]['at_ms']-start['at_ms'], receiver_read_ms=received['read_ms'],
            receiver_decode_ms=received['decode_ms']))
    event_lines=(root/'events.jsonl').read_bytes().splitlines(keepends=True)
    events = [json.loads(line) for line in event_lines]
    prefix = len(rows)-len(events)
    assert prefix>=0
    for event, line, row in zip(events, event_lines, rows[prefix:]):
        assert len(line)==row['bytes'], 'captured event/receiver length mismatch'
        if event.get('payload',{}).get('kind')=='completed_snapshot':
            assert row['frame_class']=='optional'
            assert row['compile_revision']==event['payload']['compile_revision']
    optional = [r for r in rows if r['frame_class']=='optional']
    return dict(rows=rows, startup_frames_before_event_capture=prefix,
        optional_write_total_ms=sum(r['write_ms'] for r in optional),
        optional_decode_total_ms=sum(r['receiver_decode_ms'] for r in optional),
        lifecycles={str(seq):dict(compile_revision=rs[0]['compile_revision'],
            outcomes=[r['outcome'] for r in sorted(rs,key=lambda r:r['at_ms'])]) for seq,rs in frames.items()},
        unconsumed_writer_frames=len(starts)-len(records),
        qualification='Original successful writer order aligned with Client-wide receipt ordinal and exact lengths; historical generation checked against captured events. Writer intervals include tracing and flush, receiver decode is json.loads only. Clock origins are not subtracted; no native paint or causal speedup claim.')


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('capture',type=Path);parser.add_argument('output',type=Path)
    args=parser.parse_args();args.output.write_text(json.dumps(review(args.capture),indent=2)+'\n')
