#!/usr/bin/env python3
"""Audit pinned recorded burst only. Run from repository root; no workload executes.
Requires original locally pinned assets for provenance verification.
"""
import pathlib, subprocess, json, gzip, hashlib, argparse, base64
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--historical", action="store_true")
parser.add_argument('--process-mode', choices=['current', 'historical'])
args = parser.parse_args()
if args.process_mode:
    args.historical = args.process_mode == 'historical'
sha = subprocess.check_output(['git', 'rev-parse', 'c5518be3' if args.process_mode else ('8d913d2a' if args.historical else '06b2f7a8')], text=True).strip()
base = ('crates/preview-controller/benchmarks/typing-burst-50kb-historical/' if args.historical else 'crates/preview-controller/benchmarks/typing-burst-50kb/checked/')

if args.process_mode:
    base = f'crates/preview-controller/benchmarks/typing-burst-process-pair/{args.process_mode}/'

def blob(n):
    return subprocess.check_output(['git', 'show', sha + ':' + base + n])

def digest(b):
    return hashlib.sha256(b).hexdigest()
data = {}
archive = json.loads(blob('archive.json')) if args.process_mode else json.loads(blob('provenance.json'))['compressed_artifacts']
for n, m in archive.items():
    z = blob(m['path'])
    assert digest(z) == m['sha256']
    raw = gzip.decompress(z)
    assert digest(raw) == m['uncompressed_sha256']
    data[n] = raw
provenance_bytes = data['provenance.json'] if args.process_mode else blob('provenance.json')
p = json.loads(provenance_bytes)
for n, raw in data.items():
    assert n not in p['artifacts'] or digest(raw) == p['artifacts'][n]
for a in p['assets']:
    assert digest(pathlib.Path(a['path']).read_bytes()) == a['sha256']
commands = [json.loads(l) for l in data['requests.jsonl'].splitlines()]
assert len(commands) == 20
states = [data['initial.tex']]
ids = []
chain = []
for i, r in enumerate(commands):
    c = r['payload']['command']
    assert c['expected_revision'] == i + 1 and c['expected_sha256'] == digest(states[-1])
    assert r['id'] == f'edit-{i}'
    ids.append(c['command_id'])
    e = c['edits'][0]
    a, b = (e['start_byte'], e['end_byte'])
    assert states[-1][a:b].decode() == e['removed_text']
    new = states[-1][:a] + e['replacement'].encode() + states[-1][b:]
    assert len(new) == 50000
    states.append(new)
    chain.append({'revision': i + 2, 'sha256': digest(new), 'command_id': c['command_id']})
assert len(set(ids)) == 20 and states[-1] == data['final.tex']
events = [json.loads(l) for l in data['events.jsonl'].splitlines()]
acks = []
previews = []
latest = 1
floor = 1
post = {}
measuring = True
for event in events:
    identity = event.get('id')
    v = event.get('payload', {})
    if identity == 'history':
        measuring = False
    if identity and identity.startswith('edit-'):
        i = int(identity[5:])
        h = v['history']
        d = h['document']
        assert d['revision'] == h['command_revision'] == i + 2
        assert d['source_sha256'] == digest(states[i + 1])
        assert h['replayed_command'] is False
        acks.append(identity)
        latest = i + 2
    if measuring and v.get('kind') == 'preview':
        rev = v['source_versions']['main.tex']
        assert rev >= latest and rev > floor
        floor = rev
        previews.append(rev)
    if identity in ['history', 'retry-0', 'retry-19', 'conflict']:
        post[identity] = event
assert acks == [f'edit-{i}' for i in range(20)] and previews == [21]
assert post['history']['payload']['history']['permanent_command_ids'] == 20
for key, revision in [('retry-0', 2), ('retry-19', 21)]:
    h = post[key]['payload']['history']
    assert h['replayed_command'] is True and h['command_revision'] == revision
    assert h['document']['revision'] == 21 and h['document']['source_sha256'] == digest(states[-1])
assert post['conflict']['type'] == 'error' and 'command_id_conflict' in post['conflict']['payload']['message']
streams = []
original_pairs = []
burst_revisions = []
matched_final = False
for name, raw in data.items():
    if '/producer-' not in name or not name.endswith('.input.jsonl'):
        continue
    requests = [json.loads(l) for l in raw.splitlines()]
    output = data[name.replace('.input.', '.output.')]
    replies = [json.loads(l) for l in output.splitlines()]
    assert [r['id'] for r in requests] == [r['id'] for r in replies]
    reopened = len(requests) == 1 and requests[0]['payload']['documents'][0]['text'].encode() == states[-1]
    revisions = []
    for r, reply, line in zip(requests, replies, raw.splitlines(keepends=True)):
        original_pairs.append((name, r, reply))
        revision = r['payload']['revision']
        expected = states[-1] if reopened or revision > 21 else states[revision - 1]
        assert r['payload']['documents'] == [{'path': 'main.tex', 'text': expected.decode()}]
        assert reply['payload']['revision'] == revision
        revisions.append(revision)
        if not reopened and revision <= 21:
            burst_revisions.append(revision)
        if not reopened and revision == 21:
            assert line == data['clean.input.jsonl']
            assert reply == json.loads(data['clean.output.jsonl']) == json.loads(data['final-preview.json'])['result']
            matched_final = True
    assert revisions == sorted(set(revisions))
    streams.append({'capture': name, 'reopened': reopened, 'compile_revisions': revisions, 'exact_snapshot_sequence': True, 'request_sha256': digest(raw), 'response_sha256': digest(output)})
assert matched_final
stale = [e['payload'] for e in events if e.get('payload', {}).get('kind') == 'stale']
superseded = [e['payload'] for e in events if e.get('payload', {}).get('kind') == 'superseded']
historical = [e['payload'] for e in events if e.get('payload', {}).get('kind') == 'completed_snapshot']
completed = historical if args.historical else stale
completed_revisions = [e['compile_revision'] for e in completed]
actual_completed = [r for r in burst_revisions if 1 < r < 21]
assert completed_revisions == sorted(set(completed_revisions))
assert set(completed_revisions) <= set(actual_completed)
unreported_completed = sorted(set(actual_completed) - set(completed_revisions))
assert unreported_completed == ([15] if args.process_mode == 'historical' else [])
assert all((e['request_id'] == f"preview-{e['compile_revision']}" for e in stale))
missing = sorted(set(range(2, 22)) - set(burst_revisions))
assert sorted((int(e['request_id'].split('-')[1]) for e in superseded)) == missing
assert all((int(e['by_id'].split('-')[1]) > int(e['request_id'].split('-')[1]) for e in superseded))
expected_counts = ((0, 6, 12) if args.historical else (8, 0, 11)) if args.process_mode else ((0, 7, 12) if args.historical else (11, 0, 8))
assert (len(stale), len(historical), len(superseded)) == expected_counts
for h in historical:
    revision = h['source_versions']['main.tex']
    assert h['is_current'] is False and h['source_actions_enabled'] is False
    assert h['source_binding_token'] == commands[revision - 2]['payload']['source_binding_token'] == f'burst-{revision - 2}'
    assert h['compile_revision'] < h['current_compile_revision']
    assert h['result']['id'] == h['request_id']
    assert h['result']['payload']['revision'] == h['compile_revision']
    pairs = [(name, request, reply) for name, request, reply in original_pairs if request['id'] == h['request_id'] and reply == h['result']]
    assert len(pairs) == 1
    _, request, reply = pairs[0]
    assert request['payload']['revision'] == h['compile_revision']
    assert request['payload']['documents'] == [{'path': 'main.tex', 'text': states[revision - 1].decode()}]
    assert reply['payload']['project_id'] == request['payload']['project_id'] == h['project_id']
if args.historical:
    enable = next(e for e in events if e.get('id') == 'historical-enable')
    disable = next(e for e in events if e.get('id') == 'historical-disable')
    assert enable['payload']['enabled'] is True and disable['payload']['enabled'] is False
    assert events.index(disable) < events.index(post['retry-0'])
    assert [h['compile_revision'] for h in historical] == ([2, 5, 9, 12, 18, 20] if args.process_mode else [2, 4, 7, 10, 12, 15, 18])

summary = {'schema_version': 1, 'capture_commit': sha, 'capture_path': base, 'capture_provenance_sha256': digest(provenance_bytes), 'verified_artifact_count': len(data), 'verified_assets': len(p['assets']), 'source_chain': chain, 'ack_order': acks, 'current_preview_revisions_during_burst': previews, 'captured_burst_compile_revisions': burst_revisions, 'streams': streams, 'stale_completed_revisions': [e['compile_revision'] for e in stale], 'superseded_revisions': missing, 'retry_command_revisions': [2, 21], 'retry_current_revision': 21, 'conflicting_id_refused': True, 'clean_final_uses_exact_captured_request': True, 'scope': 'Read-only published artifact audit, no repeated throughput run. All20 durable commands accepted; only one current preview21, not every-keystroke or native responsiveness. Postmeasurement retry/reopen captures classified separately.'}
if args.historical:
    summary['historical_revisions'] = [h['compile_revision'] for h in historical]
    summary['max_observed_sender_lateness_ms'] = max(s['sent_ms'] - s['target_ms'] for s in p['sends'])
    summary['cadence_scope'] = '30ms targeted, not achieved exactly; thread sender shared Python process with large result decoder; no causal claim or native timing.'
out = pathlib.Path('crates/document-runtime/benchmarks/typing-burst-historical-review' if args.historical else 'crates/document-runtime/benchmarks/typing-burst-review')
if args.process_mode:
    config = json.loads(data['sender/sender-config.json'])
    sender = json.loads(data['sender/sender-result.json'])
    frames = [base64.b64decode(f, validate=True) for f in config['frames']]
    assert b''.join(frames) == data['requests.jsonl']
    assert len(frames) == len(sender['sends']) == 20
    assert config['interval'] == .030 and config['deadline'] - config['start'] == 30
    for i, (row, observed) in enumerate(zip(sender['sends'], p['sends'])):
        assert row['index'] == i and row['target_ms'] == i * config['interval'] * 1000
        assert row['sent_ms'] <= row['flush_ms']
        if i:
            assert sender['sends'][i - 1]['flush_ms'] <= row['sent_ms']
        assert {k: v for k, v in row.items() if k != 'index'} == {k: v for k, v in observed.items() if k != 'id'}
        assert observed['id'] == f'edit-{i}'
    for filename, expected_hash in p['scripts'].items():
        script = subprocess.check_output(['git', 'show', sha + ':crates/preview-controller/examples/' + filename])
        assert digest(script) == expected_hash
    peer_mode = 'current' if args.historical else 'historical'
    peer_base = base.replace('/' + args.process_mode + '/', '/' + peer_mode + '/')
    def peer_blob(name):
        return subprocess.check_output(['git', 'show', sha + ':' + peer_base + name])
    peer_archive = json.loads(peer_blob('archive.json'))
    peer_data = {}
    for name in ['requests.jsonl', 'initial.tex', 'final.tex', 'clean.input.jsonl', 'clean.output.jsonl', 'provenance.json']:
        entry = peer_archive[name]
        compressed = peer_blob(entry['path'])
        assert digest(compressed) == entry['sha256']
        raw = gzip.decompress(compressed)
        assert digest(raw) == entry['uncompressed_sha256']
        peer_data[name] = raw
    peer_commands = list(map(json.loads, peer_data['requests.jsonl'].splitlines()))
    for i, (command, peer_command) in enumerate(zip(commands, peer_commands)):
        own = json.loads(json.dumps(command))
        other = json.loads(json.dumps(peer_command))
        if args.historical:
            assert own['payload'].pop('source_binding_token') == f'burst-{i}'
        else:
            assert other['payload'].pop('source_binding_token') == f'burst-{i}'
        assert own == other
    for name in ['initial.tex', 'final.tex', 'clean.input.jsonl', 'clean.output.jsonl']:
        assert data[name] == peer_data[name]
    peer_provenance = json.loads(peer_data['provenance.json'])
    for key in ['helper_sha256', 'producer_sha256', 'scripts', 'assets']:
        assert p[key] == peer_provenance[key]
    summary['paired_exact_source_request_and_clean_reply'] = True
    summary['unreported_completed_revisions'] = unreported_completed
    summary['sender_frames_exact'] = True
    summary['max_observed_sender_lateness_ms'] = max(s['sent_ms'] - s['target_ms'] for s in p['sends'])
    summary['cadence_scope'] = 'One recorded separate-process pair; actual timestamps verified, no guarantee or causal/native claim.'
    summary['sender_mode'] = p['sender_mode']
    out = pathlib.Path('crates/document-runtime/benchmarks/typing-burst-process-review') / args.process_mode
out.mkdir(parents=True, exist_ok=True)
(out / 'audit.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps({'acks': len(acks), 'current_previews': previews, 'captured_burst_compile_revisions': burst_revisions, 'streams': streams}, indent=2))
