"""Audit pinned captures only; never executes the helper or producer."""
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

PEER = 'f8bb23892aa01bee3b7c436493feb824ae491cbe'
BASE = 'crates/preview-controller/benchmarks/reply-cap-recovery/'
def blob(path):
    return subprocess.check_output(['git', 'show', PEER + ':' + path])
def sha(data):
    return hashlib.sha256(data).hexdigest()
def audit(mode):
    base = BASE + mode + '/'
    p = json.loads(blob(base + 'provenance.json'))
    originals = {}
    for name, expected in p['step_artifact_sha256'].items():
        compressed = blob(base + name)
        assert sha(compressed) == expected
        originals[name[:-3]] = gzip.decompress(compressed)
    for name, expected in p['diagnostic_artifact_sha256'].items():
        assert sha(blob(base + name)) == expected
    for asset in p['assets']:
        assert sha(Path(asset['path']).read_bytes()) == asset['sha256']
    for name, field in [('helper_display_replay.py','replay_script_sha256'), ('helper_replay.py','replay_client_sha256')]:
        assert sha(blob('crates/preview-controller/examples/' + name)) == p[field]
    assert p['compiler_max_frame_bytes'] == 6000 and p['display_transport'] == 'value'
    accepted, source_hashes, framed_sizes = [], [], []
    for step in range(3):
        d = json.loads(originals[f'step-{step}.json'])
        request = d['request']; source = request['payload']['documents'][0]['text']
        digest = sha(source.encode()); source_hashes.append(digest)
        wire = originals[f'step-{step}.producer.jsonl']
        frames = wire.splitlines(keepends=True)
        assert all(f.endswith(b'\n') and len(f) <= 6000 for f in frames)
        framed_sizes.append([len(f) for f in frames])
        direct = [json.loads(f) for f in frames]
        assert direct == d['direct']
        events = d['events']
        previews = [e['payload'] for e in events if e.get('payload',{}).get('kind') == 'preview']
        candidates = [e['payload'] for e in events if e.get('payload',{}).get('kind') == 'display_candidate']
        assert len(previews) == 1
        preview = previews[0]
        assert preview['result'] == direct[0]
        assert preview['source_versions'] == {'main.tex': step + 1}
        assert preview['compile_revision'] == request['payload']['revision'] == step + 2
        assert preview['request_id'] == request['id'] == direct[0]['id']
        assert all(e.get('type') != 'error' and e.get('payload',{}).get('kind') != 'failed' for e in events)
        payload = direct[0]['payload']; ok = 'display-list-v2' in payload.get('layout_capabilities', [])
        accepted.append(ok)
        assert len(direct) == 1 + int(ok) and len(candidates) == int(ok)
        if ok:
            c = candidates[0]
            assert c['display_list'] == direct[1] and c['untrusted'] and c['source_actions_enabled'] is False
            assert c['source_versions'] == preview['source_versions'] and c['compile_revision'] == step + 2
            assert c['request_id'] == direct[1]['id'] == request['id']
            b = direct[1]['payload']['documents'][0]
            assert b['sha256'] == digest and b['byte_length'] == len(source.encode()) and b['revision'] == step + 2
            assert json.loads(originals[f'step-{step}.candidate.jsonl'])['payload'] == c
            assert payload['status'] == 'ok' and payload['diagnostics'] == []
        else:
            assert payload['status'] == 'recovered'
            assert any(x.get('code') == 'display_list_declined' and '5999-byte' in x['message'] for x in payload['diagnostics'])
        if step:
            document = next(e['payload']['document'] for e in events if e.get('type') == 'result')
            assert document['text'] == source and document['revision'] == step + 1
    reopened_bytes = blob(base + 'reopened-document.json')
    assert sha(reopened_bytes) == p['reopened_document_sha256']
    assert json.loads(reopened_bytes) == document
    assert accepted == ([True, False, False] if mode == 'growing' else [True, False, True])
    if mode == 'recovery': assert source_hashes[0] == source_hashes[2]
    return {'mode': mode, 'peer': PEER, 'accepted': accepted, 'framed_sizes': framed_sizes,
            'compressed_hashes': len(originals), 'original_sha256': {n:sha(b) for n,b in originals.items()},
            'asset_hashes': len(p['assets']), 'reopened_equals_final_ack': True,
            'source_hashes': source_hashes, 'raw_spelling_claim': False,
            'scope': 'Value capture; direct request reconstructed; no helper stdin capture or workload rerun'}
if __name__ == '__main__':
    print(json.dumps([audit(mode) for mode in ['growing','recovery']], indent=2, sort_keys=True))
