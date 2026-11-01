#!/usr/bin/env python3
"""Actual producer -> helper display replay. No network, native paint or oracle claim."""
import argparse
import hashlib
import gzip
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
from helper_replay import Client, snapshot_after_initial_preview


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    for name in ('helper', 'producer', 'producer-evidence', 'fixture', 'output'):
        parser.add_argument('--' + name, required=True)
    parser.add_argument('--repeat', type=int, default=1)
    parser.add_argument('--reply-limit', type=int)
    parser.add_argument('--compiler-max-frame-bytes', type=int)
    parser.add_argument('--helper-source-sha')
    parser.add_argument('--compress-artifacts', action='store_true')
    parser.add_argument('--display-transport', choices=('value', 'raw-prototype'), default='value')
    args = parser.parse_args()
    if not 1 <= args.repeat <= 2000:
        parser.error('--repeat must be 1..2000')
    expected = json.loads(Path(args.producer_evidence).read_text())
    assert digest(args.producer) == expected['binary_sha256'], 'producer binary drift'
    for asset in expected['assets']:
        assert digest(asset['path']) == asset['sha256'], 'asset drift: ' + asset['path']
    font_dirs = sorted({str(Path(a['path']).parent) for a in expected['assets'] if a['path'].endswith('.otf')})
    tfm_dirs = sorted({str(Path(a['path']).parent) for a in expected['assets'] if a['path'].endswith('.tfm')})
    assert font_dirs and tfm_dirs
    os.environ['FLASHTEX_FONT_DIRS'] = os.pathsep.join(font_dirs)
    os.environ['FLASHTEX_TFM_DIRS'] = os.pathsep.join(tfm_dirs)
    os.environ.pop('FLASHTEX_MAX_REPLY_BYTES', None)
    if args.reply_limit is not None:
        os.environ['FLASHTEX_MAX_REPLY_BYTES'] = str(args.reply_limit)
    fixture = json.loads(Path(args.fixture).read_text())
    source_states = None
    if isinstance(fixture, list):
        assert len(fixture) == 3 and args.repeat == 1, 'sequence requires three exact states and repeat=1'
        fixture = [case.get('request', case) for case in fixture]
        assert all(len(r['payload']['documents']) == 1 and
                   r['payload']['documents'][0]['path'] == 'main.tex' for r in fixture)
        source_states = [r['payload']['documents'][0]['text'] for r in fixture]
        source = source_states[0]
    else:
        source = fixture['payload']['documents'][0]['text']
        source = source.replace('Office AV fi.', ('Office AV fi.\n' * args.repeat).rstrip())
    output = Path(args.output)
    output.mkdir(parents=True, exist_ok=False)
    cases = []
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        (root / 'project').mkdir()
        (root / 'private').mkdir()
        (root / 'project/main.tex').write_text(source)
        config = root / 'config.json'
        settings = dict(session_id='benchmark', project_id='p', entry_path='main.tex',
            project_root=str(root/'project'), private_ledger_root=str(root/'private'),
            compiler_path=str(Path(args.producer).resolve()), diagnostic_timings=True)
        if args.compiler_max_frame_bytes is not None:
            settings['compiler_max_frame_bytes'] = args.compiler_max_frame_bytes
        if args.display_transport == 'raw-prototype':
            settings['display_transport'] = 'raw-prototype'
        capability = ('display-candidates-raw-v1' if args.display_transport == 'raw-prototype'
                      else 'display-candidates-v1')
        config.write_text(json.dumps(settings))
        client = Client(args.helper, config, capture_diagnostics=True, capture_wire=True)
        try:
            document = snapshot_after_initial_preview(client)
            assert document['text'] == source
            for step in range(3):
                started = time.monotonic()
                ack_ms = v1_ms = candidate_ms = None
                candidate = current = None
                candidate_wire = None
                if step == 0:
                    client.send('enable', 'configure_display_candidates', dict(capability=capability,
                        enabled=True, renderer_support_confirmed=True))
                    reply_id = 'enable'
                else:
                    source = source_states[step] if source_states else (
                        source.replace('Office AV fi.', 'Office AV fi. More text.') if step == 1
                        else source.replace('More text.', 'More text. Final edit.'))
                    reply_id = 'edit-' + str(step)
                    client.send(reply_id, 'edit', dict(path='main.tex', expected_revision=document['revision'],
                        expected_sha256=document['source_sha256'], text=source))
                events = []
                previews = {}
                acknowledged = False
                while True:
                    event, received = client.read()
                    events.append(event)
                    payload = event.get('payload', {})
                    assert event['type'] != 'error', event
                    assert payload.get('kind') != 'failed', event
                    if event.get('id') == reply_id:
                        assert event['type'] == 'result'
                        if step == 0:
                            assert payload['capability'] == capability and payload['enabled'] is True
                        acknowledged = True
                        ack_ms = (received - started) * 1000
                        if step:
                            document = payload['document']
                            assert document['text'] == source
                    if payload.get('kind') == 'preview':
                        previews[payload['request_id']] = payload['result']
                        if acknowledged and payload['source_versions'] == {'main.tex': document['revision']}:
                            current = payload
                            v1_ms = (received - started) * 1000
                            if 'display-list-v2' not in payload['result']['payload'].get('layout_capabilities', []):
                                break
                    if payload.get('kind') == 'display_candidate':
                        assert acknowledged
                        candidate = payload
                        candidate_wire = client.last_wire
                        candidate_ms = (received - started) * 1000
                        assert candidate['untrusted'] and not candidate['source_actions_enabled']
                        assert candidate['source_versions'] == {'main.tex': document['revision']}
                        assert candidate['request_id'] in previews, 'candidate preceded matching v1'
                        assert current is not None and candidate['request_id'] == current['request_id']
                        assert candidate['compile_revision'] == current['compile_revision']
                        break
                request = dict(protocol_version=1, type='compile', id=current['request_id'], payload=dict(
                    project_id='p', revision=current['compile_revision'], entry_path='main.tex',
                    documents=[dict(path='main.tex', text=source)], layout_capabilities=['display-list-v2']))
                direct_env = os.environ.copy()
                if args.compiler_max_frame_bytes is not None:
                    budget = args.compiler_max_frame_bytes - 1
                    if args.reply_limit is not None and args.reply_limit > 0:
                        budget = min(budget, args.reply_limit)
                    direct_env['FLASHTEX_MAX_REPLY_BYTES'] = str(budget)
                direct = subprocess.run([args.producer], env=direct_env, input=(json.dumps(request)+'\n').encode(),
                    capture_output=True, check=True, timeout=20)
                lines = [json.loads(line) for line in direct.stdout.splitlines()]
                assert current is not None
                assert lines[0] == previews[current['request_id']], 'v1 output changed through helper'
                source_hash = hashlib.sha256(source.encode()).hexdigest()
                if candidate is not None:
                    assert len(lines) == 2
                    assert lines[1] == candidate['display_list'], 'display output changed through helper'
                    binding = lines[1]['payload']['documents'][0]
                    assert binding['sha256'] == source_hash
                    assert binding['byte_length'] == len(source.encode())
                    if args.display_transport == 'raw-prototype':
                        # Typed helper wrapper writes display_list last in payload.
                        # Assert the original nested body, not re-encoded JSON equality.
                        body = direct.stdout.splitlines()[1].strip()
                        assert candidate_wire.endswith(b'"display_list":' + body + b'}}\n'), 'raw producer spelling changed'
                    (output/f'step-{step}.candidate.jsonl').write_bytes(candidate_wire)
                else:
                    assert len(lines) == 1, 'declined producer sent unexpected sibling'
                (output/f'step-{step}.json').write_text(json.dumps(dict(request=request, events=events, direct=lines), indent=2)+'\n')
                (output/f'step-{step}.producer.jsonl').write_bytes(direct.stdout)
                cases.append(dict(step=step, source_bytes=len(source.encode()), source_revision=document['revision'], compile_revision=current['compile_revision'],
                    exact_direct_v1=True, exact_direct_display=True if candidate else None, source_sha256=source_hash,
                    exact_raw_body=True if candidate and args.display_transport == 'raw-prototype' else None,
                    v2_accepted=candidate is not None, status=lines[0]['payload']['status'], diagnostics=lines[0]['payload']['diagnostics'],
                    page_count=len(lines[0]['payload']['pages']),
                    page_item_counts=[len(p.get('items', [])) for p in lines[0]['payload']['pages']],
                    ack_receipt_ms=ack_ms, v1_receipt_ms=v1_ms, candidate_receipt_ms=candidate_ms))
            diagnostics = client.diagnostics()
        finally:
            try:client.stop()
            finally:
                (output/'diagnostics-raw.jsonl').write_bytes(client.last_diagnostics_raw[:1024*1024])
                (output/'diagnostics-capture-status.json').write_text(json.dumps(client.diagnostics_status)+'\n')
        diagnostic_capture_status = client.diagnostics_status
        client = Client(args.helper, config)
        try:
            recovered = snapshot_after_initial_preview(client)
            assert recovered == document, 'durable source changed on reopen'
            (output/'reopened-document.json').write_text(json.dumps(recovered, indent=2)+'\n')
        finally:
            client.stop()
    evidence = dict(cases=cases, exact_reopen=True, helper_sha256=digest(args.helper),
        producer_sha256=digest(args.producer), producer_source_sha=expected['producer_sha'],
        producer_evidence_sha256=digest(args.producer_evidence), fixture_sha256=digest(args.fixture),
        assets=expected['assets'], diagnostics=diagnostics, diagnostic_capture_status=diagnostic_capture_status,
        repeat=args.repeat, reply_limit=args.reply_limit, compiler_max_frame_bytes=args.compiler_max_frame_bytes,
        display_transport=args.display_transport, negotiated_capability=capability,
        native_rendering='not performed', native_latency='not measured')
    evidence['reopened_document_sha256'] = digest(output/'reopened-document.json')
    evidence['helper_source_sha'] = args.helper_source_sha
    evidence['replay_script_sha256'] = digest(__file__)
    evidence['replay_client_sha256'] = digest(Path(__file__).with_name('helper_replay.py'))
    if args.compress_artifacts:
        for artifact in list(output.glob('step-*.json')) + list(output.glob('step-*.jsonl')):
            artifact.with_name(artifact.name + '.gz').write_bytes(gzip.compress(artifact.read_bytes(), mtime=0))
            artifact.unlink()
    evidence['diagnostic_artifact_sha256'] = {name: digest(output/name) for name in ['diagnostics-raw.jsonl','diagnostics-capture-status.json']}
    evidence['artifact_encoding'] = 'gzip' if args.compress_artifacts else 'json'
    evidence['step_artifact_sha256'] = {p.name: digest(p) for p in sorted(output.glob('step-*.json*'))}
    (output/'provenance.json').write_text(json.dumps(evidence, indent=2)+'\n')
    print(json.dumps([{**{k:v for k,v in case.items() if k != 'diagnostics'},
                       'diagnostic_count':len(case['diagnostics'])} for case in cases]))


if __name__ == '__main__':
    main()
