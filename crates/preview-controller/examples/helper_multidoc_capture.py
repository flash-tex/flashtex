#!/usr/bin/env python3
"""Capture original producer through helper for unequal editor revisions; no native claim."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
from helper_replay import Client


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    for name in ('helper', 'ledger', 'producer', 'producer-evidence', 'output'):
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    evidence = json.loads(Path(args.producer_evidence).read_text())
    assert sha(Path(args.producer).read_bytes()) == evidence['binary_sha256']
    for asset in evidence['assets']:
        assert sha(Path(asset['path']).read_bytes()) == asset['sha256']
    for suffix, env in [('.otf', 'FLASHTEX_FONT_DIRS'), ('.tfm', 'FLASHTEX_TFM_DIRS')]:
        os.environ[env] = os.pathsep.join(sorted({str(Path(a['path']).parent)
            for a in evidence['assets'] if a['path'].endswith(suffix)}))
    os.environ['FLASHTEX_CAPTURE_DIRECTORY'] = str(output)
    os.environ['FLASHTEX_CAPTURE_PRODUCER'] = str(Path(args.producer).resolve())
    proxy = Path(__file__).with_name('producer_capture_proxy.py').resolve()
    sources = {'main.tex': (7, '\\documentclass{article}\n\\begin{document}\nOffice AV fi.\\input{chapter}\n\\end{document}\n'),
        'chapter.tex': (19, 'Chapter text with \\(a+b\\).\n'),
        'refs.bib': (23, '@article{sample, title={Example}, author={A. Author}, year={2026}}\n')}
    observations = []
    source_snapshots = {}
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        stores = []
        for index, (path, (revision, text)) in enumerate(sources.items()):
            store = root / str(index); stores.append(str(store))
            document = dict(project_id='p', path=path, revision=revision,
                source_sha256=sha(text.encode()), text=text)
            frame = dict(id='initialize', operation='initialize', document=document)
            result = subprocess.run([args.ledger, '--store', str(store)],
                input=(json.dumps(frame)+'\n').encode(), capture_output=True, check=True, timeout=10)
            assert json.loads(result.stdout)['command_succeeded'], result.stdout
        config = root / 'config.json'
        config.write_text(json.dumps(dict(session_id='benchmark', project_id='p', entry_path='main.tex',
            store_paths=stores, bibliography_paths=['refs.bib'], compiler_path=str(proxy),
            display_transport='raw-prototype', diagnostic_timings=True)))
        client = Client(args.helper, config, capture_wire=True, capture_diagnostics=True)
        capture = (output/'helper.output.jsonl').open('wb')
        minimum_chapter_revision = None
        def read():
            event, _ = client.read(); capture.write(client.last_wire); capture.flush()
            assert event['type'] != 'error', event
            payload = event.get('payload', {})
            assert payload.get('kind') != 'failed', event
            if minimum_chapter_revision is not None and payload.get('kind') in ('preview','display_candidate'):
                assert payload['source_versions']['chapter.tex'] >= minimum_chapter_revision, 'stale preview escaped'
            return event
        def reply(identity):
            while True:
                event = read()
                if event.get('id') == identity: return event['payload']
        def preview(candidate=False):
            while True:
                event = read(); p = event.get('payload', {})
                if p.get('kind') == ('display_candidate' if candidate else 'preview'):
                    assert p['source_versions'] == {k:v[0] for k,v in sources.items()}
                    observations.append(p)
                    source_snapshots[p['request_id']] = {k:v[1] for k,v in sources.items()}
                    return p
        try:
            assert read()['type'] == 'ready'
            initial = preview()
            assert 'display-list-v2' not in initial['result']['payload'].get('layout_capabilities', [])
            client.send('snapshot', 'snapshot', {})
            snapshot = reply('snapshot')
            (output/'helper.snapshot.json').write_text(json.dumps(snapshot,indent=2)+'\n')
            assert snapshot['document_kinds']['refs.bib'] == 'bibliography'
            assert snapshot['source_versions'] == {k:v[0] for k,v in sources.items()}
            for round_number in range(2):
                client.send('enable', 'configure_display_candidates', dict(capability='display-candidates-raw-v1',
                    enabled=True, renderer_support_confirmed=True))
                assert reply('enable')['enabled'] is True
                candidate = preview(True)
                expected = {path:dict(sha256=sha(text.encode()), byte_length=len(text.encode()))
                    for path, (_,text) in sources.items()}
                bindings = candidate['display_list']['payload']['documents']
                assert {d['path'] for d in bindings} == set(sources)
                for d in bindings:
                    assert d['revision'] == candidate['compile_revision']
                    assert d['sha256'] == expected[d['path']]['sha256']
                    assert d['byte_length'] == expected[d['path']]['byte_length']
                if round_number == 0:
                    client.send('restart', 'restart', {})
                    reply('restart')
                    fallback = preview()
                    assert 'display-list-v2' not in fallback['result']['payload'].get('layout_capabilities', [])
            # Hold original producer output only after it has actually been emitted.
            # A newer durable edit supersedes the in-flight generation before delivery.
            (output/'hold-output').touch()
            old_text = sources['chapter.tex'][1]
            client.send('first-edit', 'edit', dict(path='chapter.tex', expected_revision=19,
                expected_sha256=sha(old_text.encode()), text=old_text+'First pending edit.\n', response_mode='metadata'))
            assert reply('first-edit')['document']['revision'] == 20
            deadline = time.monotonic()+10
            while not (output/'output-held').exists():
                assert time.monotonic() < deadline, 'producer did not emit gated frame'
                time.sleep(.002)
            pending = old_text+'First pending edit.\n'
            latest = old_text+'Latest superseding edit.\n'
            client.send('latest-edit', 'edit', dict(path='chapter.tex', expected_revision=20,
                expected_sha256=sha(pending.encode()), text=latest, response_mode='metadata'))
            assert reply('latest-edit')['document']['revision'] == 21
            sources['chapter.tex'] = (21, latest)
            minimum_chapter_revision = 21
            (output/'hold-output').unlink()
            superseded = preview(True)
            assert superseded['source_versions']['chapter.tex'] == 21
            for path, (revision, text) in sources.items():
                client.send('document', 'document', dict(path=path))
                doc = reply('document')['document']
                assert doc['revision'] == revision and doc['text'] == text
                assert doc['source_sha256'] == sha(text.encode())
            (output/'diagnostics.json').write_text(json.dumps(client.diagnostics(), indent=2)+'\n')
        finally:
            client.stop(); capture.close()
        reopened = {}
        for store, (path, (revision, text)) in zip(stores, sources.items()):
            result = subprocess.run([args.ledger, '--store', store],
                input=b'{"id":"status","operation":"status"}\n', capture_output=True, check=True, timeout=10)
            record = json.loads(result.stdout)
            assert record['command_succeeded']
            doc = record['payload']['document']
            assert doc['revision'] == revision and doc['text'] == text
            assert doc['source_sha256'] == sha(text.encode())
            reopened[path] = doc
        (output/'ledger.reopened.json').write_text(json.dumps(reopened,indent=2)+'\n')
    inputs = []
    outputs = []
    for file in sorted(output.glob('producer-*.input.jsonl')):
        inputs.extend(json.loads(line) for line in file.read_bytes().splitlines())
    for file in sorted(output.glob('producer-*.output.jsonl')):
        outputs.extend(json.loads(line) for line in file.read_bytes().splitlines())
    by_id = {r['id']:r for r in inputs}
    for observed in observations:
        request = by_id[observed['request_id']]
        assert request['payload']['revision'] == observed['compile_revision']
        assert {d['path']:d['text'] for d in request['payload']['documents']} == source_snapshots[observed['request_id']]
        original = observed.get('display_list', observed.get('result'))
        assert original in outputs
        if observed['kind'] == 'display_candidate':
            for binding in original['payload']['documents']:
                text = source_snapshots[observed['request_id']][binding['path']]
                assert binding['revision'] == observed['compile_revision']
                assert binding['sha256'] == sha(text.encode())
                assert binding['byte_length'] == len(text.encode())
        if observed['kind'] == 'display_candidate':
            producer_frames = [line for file in output.glob('producer-*.output.jsonl')
                for line in file.read_bytes().splitlines() if json.loads(line) == original]
            helper_frames = (output/'helper.output.jsonl').read_bytes().splitlines(keepends=True)
            assert any(frame.endswith(b'"display_list":'+body+b'}}\n')
                for body in producer_frames for frame in helper_frames), 'original raw body changed'
    provenance = dict(helper_sha256=sha(Path(args.helper).read_bytes()),
        ledger_sha256=sha(Path(args.ledger).read_bytes()), producer_sha256=evidence['binary_sha256'],
        assets=evidence['assets'], source_versions={k:v[0] for k,v in sources.items()},
        declared_bibliography=['refs.bib'], observations=[dict(kind=p['kind'],request_id=p['request_id'],
            compile_revision=p['compile_revision'],source_versions=p['source_versions']) for p in observations],
        source_documents={k:v[1] for k,v in sources.items()}, request_source_snapshots=source_snapshots,
        scripts={p.name:sha(p.read_bytes()) for p in [Path(__file__), proxy, Path(__file__).with_name('helper_replay.py')]},
        artifacts={p.name:sha(p.read_bytes()) for p in output.iterdir() if p.is_file()},
        scope='actual original producer via transparent Linux capture proxy; default fallback, unequal editor revisions, explicit bibliography, restart and reenable, producer-emitted result held then superseded by newer durable edit; logs alone do not prove receipt, helper outputs are crosschecked; no native or bibliography layout claim')
    (output/'provenance.json').write_text(json.dumps(provenance,indent=2)+'\n')
    print(json.dumps({k:v for k,v in provenance.items() if k in ['source_versions','observations','scope']}))

if __name__ == '__main__':
    main()
