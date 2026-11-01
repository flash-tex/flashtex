"""Read-only audit of pinned helper capture; never launches helper/producer."""
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

PEER = '518a2ce3'
BASE = 'crates/preview-controller/benchmarks/helper-9aa-three-state/'
def blob(path):
    return subprocess.check_output(['git', 'show', f'{PEER}:{path}'])
def sha(data):
    return hashlib.sha256(data).hexdigest()
def audit():
    provenance = json.loads(blob(BASE + 'provenance.json'))
    originals = {}
    for name, digest in provenance['step_artifact_sha256'].items():
        compressed = blob(BASE + name)
        assert sha(compressed) == digest
        originals[name[:-3]] = gzip.decompress(compressed)
    for name, digest in provenance['diagnostic_artifact_sha256'].items():
        assert sha(blob(BASE + name)) == digest
    for asset in provenance['assets']:
        data = Path(asset['path']).read_bytes()
        assert ('bytes' not in asset or len(data) == asset['bytes']) and sha(data) == asset['sha256']
    for filename, field in [('helper_display_replay.py', 'replay_script_sha256'), ('helper_replay.py', 'replay_client_sha256')]:
        assert sha(blob('crates/preview-controller/examples/' + filename)) == provenance[field]
    for path, field in [('/home/natkarri/flashtex-preview-performance/crates/preview-controller/target/release/flashtex-preview-controller', 'helper_sha256'), ('/home/natkarri/flashtex-producer-release-artifacts/9aaec57a/target/release/flashtex-render', 'producer_sha256')]:
        assert sha(Path(path).read_bytes()) == provenance[field]
    cases = []
    for step, claim in enumerate(provenance['cases']):
        record = json.loads(originals[f'step-{step}.json'])
        request = record['request']
        payload = request['payload']
        source = payload['documents'][0]['text'].encode()
        assert sha(source) == claim['source_sha256'] and len(source) == claim['source_bytes']
        direct_bytes = originals[f'step-{step}.producer.jsonl']
        lines = direct_bytes.splitlines()
        direct = [json.loads(line) for line in lines]
        assert len(direct) == 2 and direct == record['direct']
        events = record['events']
        previews = [e['payload'] for e in events if e.get('payload', {}).get('kind') == 'preview']
        candidates = [e['payload'] for e in events if e.get('payload', {}).get('kind') == 'display_candidate']
        assert len(previews) == len(candidates) == 1
        preview, candidate = previews[0], candidates[0]
        assert preview['result'] == direct[0] and candidate['display_list'] == direct[1]
        assert candidate['untrusted'] and candidate['source_actions_enabled'] is False
        assert candidate['source_versions'] == preview['source_versions'] == {'main.tex': step + 1}
        assert candidate['compile_revision'] == preview['compile_revision'] == payload['revision'] == step + 2
        assert candidate['request_id'] == preview['request_id'] == request['id'] == direct[0]['id'] == direct[1]['id']
        assert events.index(next(e for e in events if e.get('payload') == preview)) < events.index(next(e for e in events if e.get('payload') == candidate))
        binding = direct[1]['payload']['documents']
        assert len(binding) == 1
        assert binding[0]['path'] == 'main.tex' and binding[0]['sha256'] == sha(source)
        assert binding[0]['byte_length'] == len(source) and binding[0]['revision'] == payload['revision']
        assert direct[0]['payload']['status'] == 'ok' and direct[0]['payload']['diagnostics'] == []
        assert 'display-list-v2' in direct[0]['payload']['layout_capabilities']
        wire = originals[f'step-{step}.candidate.jsonl']
        assert json.loads(wire)['payload'] == candidate
        assert wire.endswith(b'"display_list":' + lines[1].strip() + b'}}\n')
        if step:
            ack = next(e['payload']['document'] for e in events if e.get('type') == 'result')
            assert ack['text'].encode() == source and ack['revision'] == step + 1
        cases.append({'step': step, 'source_sha256': sha(source), 'editor_revision': step + 1,
                      'compile_revision': step + 2, 'exact_v1_v2_and_raw_body': True})
    return {'peer': subprocess.check_output(['git','rev-parse',PEER], text=True).strip(),
            'compressed_hashes_verified': len(originals), 'asset_hashes_verified': len(provenance['assets']),
            'original_sha256': {n: sha(b) for n,b in originals.items()}, 'cases': cases,
            'reopen': 'owner harness assertion/provenance only; reopened snapshot not archived',
            'producer_input': 'direct request reconstructed by owner harness; helper producer stdin not captured',
            'diagnostics_scope': provenance['diagnostic_capture_status'],
            'native_and_performance': 'not independently measured'}
if __name__ == '__main__':
    print(json.dumps(audit(), indent=2, sort_keys=True))
