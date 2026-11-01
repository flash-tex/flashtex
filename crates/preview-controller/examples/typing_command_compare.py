#!/usr/bin/env python3
"""One actual typing pair: request/ACK bytes and correctness, not native latency."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
from helper_replay import Client, snapshot_after_initial_preview


def sha(data):
    return hashlib.sha256(data).hexdigest()


def run(args, mode, source, replacement_source, start):
    out = Path(args.output)/mode
    out.mkdir()
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        (root/'project').mkdir(); (root/'private').mkdir()
        (root/'project/main.tex').write_text(source)
        producer_path = str(Path(args.producer).resolve())
        if args.capture_producer:
            capture_root = (out/'producer').resolve(); capture_root.mkdir()
            os.environ['FLASHTEX_CAPTURE_DIRECTORY'] = str(capture_root)
            os.environ['FLASHTEX_CAPTURE_PRODUCER'] = producer_path
            producer_path = str(Path(__file__).with_name('producer_capture_proxy.py').resolve())
        config = root/'config.json'
        config.write_text(json.dumps(dict(session_id='benchmark',project_id='p',entry_path='main.tex',
            project_root=str(root/'project'),private_ledger_root=str(root/'private'),
            compiler_path=producer_path,diagnostic_timings=True)))
        client = Client(args.helper, config, capture_diagnostics=True, capture_wire=True)
        try:
            initial = snapshot_after_initial_preview(client)
            assert initial['text'] == source and initial['revision'] == 1
            if mode == 'full':
                kind = 'edit'
                payload = dict(path='main.tex',expected_revision=1,
                    expected_sha256=initial['source_sha256'],text=replacement_source)
            else:
                kind = 'apply_group'
                payload = dict(path='main.tex',response_mode='metadata',command=dict(
                    command_id='typed-edit',expected_revision=1,expected_sha256=initial['source_sha256'],
                    label='Typing',edits=[dict(start_byte=start,end_byte=start+1,
                        removed_text='O',replacement='A')]))
            request = dict(protocol_version=1,session_id='benchmark',id='typing',type=kind,payload=payload)
            wire = json.dumps(request,ensure_ascii=False,separators=(',',':')).encode()+b'\n'
            (out/'request.jsonl').write_bytes(wire)
            started = time.monotonic()
            client.proc.stdin.write(wire); client.proc.stdin.flush()
            ack = preview = None
            ack_ms = preview_ms = None
            with (out/'events.jsonl').open('wb') as events:
                while ack is None or preview is None:
                    event, received = client.read(); events.write(client.last_wire)
                    assert event['type'] != 'error', event
                    p = event.get('payload',{})
                    assert p.get('kind') != 'failed', event
                    if event.get('id') == 'typing':
                        ack = event; ack_ms=(received-started)*1000
                        (out/'ack.jsonl').write_bytes(client.last_wire)
                        document = p['document'] if mode=='full' else p['history']['document']
                        assert document['revision']==2 and document['source_sha256']==sha(replacement_source.encode())
                        if mode=='full': assert document['text']==replacement_source
                        else:
                            assert 'text' not in document
                            assert p['history']['command_revision']==2 and p['history']['replayed_command'] is False
                    if p.get('kind')=='preview' and p['source_versions']=={'main.tex':2}:
                        preview=p; preview_ms=(received-started)*1000
                        assert p['result']['payload']['status']=='ok', p['result']['payload']['diagnostics']
            if mode=='group':
                client.send('retry','apply_group',payload)
                while True:
                    event,_=client.read()
                    if event.get('id')=='retry':
                        assert event['payload']['history']['replayed_command'] is True
                        assert event['payload']['history']['command_revision']==2
                        (out/'retry.jsonl').write_bytes(client.last_wire)
                        break
            diagnostics=client.diagnostics()
            (out/'diagnostics.json').write_text(json.dumps(diagnostics,indent=2)+'\n')
        finally:
            client.stop()
        reopened=Client(args.helper,config)
        try:
            durable=snapshot_after_initial_preview(reopened)
            assert durable['revision']==2 and durable['text']==replacement_source
            assert durable['source_sha256']==sha(replacement_source.encode())
        finally: reopened.stop()
        (out/'preview.json').write_text(json.dumps(preview,indent=2)+'\n')
        return dict(mode=mode,request_bytes=len(wire),ack_bytes=(out/'ack.jsonl').stat().st_size,
            ack_received_ms=ack_ms,preview_received_ms=preview_ms,
            save_and_submit_ms=ack['payload']['save_and_submit_ms'],
            runtime_total_ms=preview['runtime_total_ms'],controller_total_ms=preview['controller_total_ms'],
            after_controller_to_frame_ms=preview_ms-preview['controller_total_ms'],
            result=preview['result'],source_sha256=durable['source_sha256'],
            reopened_revision=durable['revision'])


def main():
    parser=argparse.ArgumentParser()
    for key in ['helper','producer','producer-evidence','output']:
        parser.add_argument('--'+key,required=True)
    parser.add_argument('--capture-producer',action='store_true')
    args=parser.parse_args()
    evidence=json.loads(Path(args.producer_evidence).read_text())
    assert sha(Path(args.producer).read_bytes())==evidence['binary_sha256']
    for asset in evidence['assets']: assert sha(Path(asset['path']).read_bytes())==asset['sha256']
    for suffix,env in [('.otf','FLASHTEX_FONT_DIRS'),('.tfm','FLASHTEX_TFM_DIRS')]:
        os.environ[env]=os.pathsep.join(sorted({str(Path(a['path']).parent)
            for a in evidence['assets'] if a['path'].endswith(suffix)}))
    out=Path(args.output);out.mkdir(exist_ok=False)
    prefix='\\documentclass{article}\n\\begin{document}\n% α marker\n'
    tail='\n\\end{document}\n'
    fill='Office AV fi. '
    budget=50000-len(prefix.encode())-len(tail.encode())
    body=(fill*((budget//len(fill))+1))[:budget]
    source=prefix+body+tail
    assert len(source.encode())==50000
    start=len(source[:source.index('Office')].encode())
    replacement_source=source.replace('Office','Affice',1)
    (out/'initial.tex').write_text(source);(out/'edited.tex').write_text(replacement_source)
    cases=[run(args,mode,source,replacement_source,start) for mode in ['full','group']]
    assert cases[0]['result']==cases[1]['result'], 'current producer preview differs by edit command'
    assert cases[0]['source_sha256']==cases[1]['source_sha256']
    for case in cases: del case['result']
    summary=dict(helper_sha256=sha(Path(args.helper).read_bytes()),producer_sha256=evidence['binary_sha256'],
        assets=evidence['assets'],source_bytes=50000,edit=dict(start_byte=start,end_byte=start+1,removed='O',replacement='A'),
        capture_proxy=args.capture_producer, cases=cases,scope='one sequential pair, exact same actual final preview and durable source; timings include helper/producer/transport and host scheduling, not native paint or calibrated superiority',
        scripts={p.name:sha(p.read_bytes()) for p in [Path(__file__),Path(__file__).with_name('helper_replay.py'),Path(__file__).with_name('producer_capture_proxy.py')]},
        artifacts={str(p.relative_to(out)):sha(p.read_bytes()) for p in out.rglob('*') if p.is_file()})
    (out/'provenance.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(json.dumps(cases))

if __name__=='__main__': main()
