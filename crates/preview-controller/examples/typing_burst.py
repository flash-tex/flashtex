#!/usr/bin/env python3
"""One actual sustained typing burst, exact final clean compile and ledger recovery."""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
from helper_replay import Client, snapshot_after_initial_preview
from scheduled_sender import ScheduledSender


def sha(data): return hashlib.sha256(data).hexdigest()


def verify_historical_results(producer_dir, delivered, states):
    """Bind each historical result and request within one original producer stream."""
    streams=[]
    for request_path in producer_dir.glob('*.input.jsonl'):
        output_path=request_path.with_name(request_path.name.replace('.input.jsonl','.output.jsonl'))
        requests=[json.loads(line) for line in request_path.read_bytes().splitlines()]
        outputs=[json.loads(line) for line in output_path.read_bytes().splitlines()]
        streams.append((requests,outputs))
    for event in delivered:
        p=event['payload'];revision=p['source_versions']['main.tex']
        result=p['result']
        assert result['id']==p['request_id'], 'historical result/request identity mismatch'
        assert result['payload']['revision']==p['compile_revision'], 'historical compile revision mismatch'
        matches=[]
        for requests,outputs in streams:
            matching_requests=[r for r in requests if r['id']==p['request_id']]
            matching_outputs=[r for r in outputs if r==result]
            if matching_requests and matching_outputs:
                assert len(matching_requests)==len(matching_outputs)==1, 'ambiguous producer stream'
                matches.append(matching_requests[0])
        assert len(matches)==1, 'historical result must bind to one producer stream'
        request=matches[0]
        assert request['payload']['revision']==p['compile_revision']
        assert request['payload']['documents']==[dict(path='main.tex',text=states[revision-1])]


def main():
    parser=argparse.ArgumentParser()
    for key in ['helper','producer','producer-evidence','initial-source','output']:
        parser.add_argument('--'+key,required=True)
    parser.add_argument('--historical',action='store_true')
    args=parser.parse_args()
    evidence=json.loads(Path(args.producer_evidence).read_text())
    assert sha(Path(args.producer).read_bytes())==evidence['binary_sha256']
    for asset in evidence['assets']: assert sha(Path(asset['path']).read_bytes())==asset['sha256']
    for suffix,env in [('.otf','FLASHTEX_FONT_DIRS'),('.tfm','FLASHTEX_TFM_DIRS')]:
        os.environ[env]=os.pathsep.join(sorted({str(Path(a['path']).parent) for a in evidence['assets'] if a['path'].endswith(suffix)}))
    source_file=Path(args.initial_source)
    raw=source_file.read_bytes()
    source=gzip.decompress(raw).decode() if source_file.suffix=='.gz' else raw.decode()
    assert len(source.encode())==50000
    offset=source.index('Office');start=len(source[:offset].encode())
    states=[source];requests=[]
    for i in range(20):
        before=states[-1];replacement=chr(ord('A')+i)
        command=dict(command_id='typing-'+str(i),expected_revision=i+1,expected_sha256=sha(before.encode()),
            label='Typing '+str(i),edits=[dict(start_byte=start,end_byte=start+1,removed_text=before[offset],replacement=replacement)])
        payload=dict(path='main.tex',response_mode='metadata',command=command)
        if args.historical:payload['source_binding_token']='burst-'+str(i)
        requests.append(dict(protocol_version=1,session_id='benchmark',id='edit-'+str(i),type='apply_group',payload=payload))
        states.append(before[:offset]+replacement+before[offset+1:])
    wires=[json.dumps(r,ensure_ascii=False,separators=(',',':')).encode()+b'\n' for r in requests]
    out=Path(args.output).resolve();out.mkdir(exist_ok=False);(out/'producer').mkdir()
    (out/'sender').mkdir()
    (out/'requests.jsonl').write_bytes(b''.join(wires));(out/'initial.tex').write_text(source);(out/'final.tex').write_text(states[-1])
    os.environ['FLASHTEX_CAPTURE_DIRECTORY']=str(out/'producer')
    os.environ['FLASHTEX_CAPTURE_PRODUCER']=str(Path(args.producer).resolve())
    with tempfile.TemporaryDirectory() as temp:
        root=Path(temp);(root/'project').mkdir();(root/'private').mkdir();(root/'project/main.tex').write_text(source)
        config=root/'config.json';config.write_text(json.dumps(dict(session_id='benchmark',project_id='p',entry_path='main.tex',project_root=str(root/'project'),private_ledger_root=str(root/'private'),compiler_path=str(Path(__file__).with_name('producer_capture_proxy.py').resolve()),diagnostic_timings=True)))
        client=Client(args.helper,config,capture_wire=True,capture_diagnostics=True)
        sender=None;events=(out/'events.jsonl').open('wb');sends=[];acks={};previews=[];historical=[]
        experiment_deadline = None
        def read():
            assert experiment_deadline is None or time.monotonic() < experiment_deadline, 'overall experiment deadline'
            if sender:sender.check_failure()
            try:
                event,received=client.read()
            except Exception:
                if sender:sender.check_failure()
                raise
            if sender:sender.check_failure()
            assert experiment_deadline is None or received < experiment_deadline, 'overall experiment deadline'
            events.write(client.last_wire);events.flush()
            return event,received
        def reply(identity):
            while True:
                event,_=read()
                if event.get('id')==identity:return event
        try:
            assert snapshot_after_initial_preview(client)['text']==source
            if args.historical:
                client.send('historical-enable','configure_completed_snapshots',dict(capability='completed-snapshots-v1',enabled=True))
                enabled=reply('historical-enable')
                assert enabled['type']=='result' and enabled['payload']['enabled'] is True
            # Record real send times; startup lateness remains visible in evidence.
            t0=time.monotonic()+.2
            experiment_deadline=t0+30
            client.proc.stdin.flush()
            sender=ScheduledSender(client.proc.stdin.fileno(),wires,out/'sender',t0)
            final=None
            while len(acks)<20 or final is None:
                event,received=read();p=event.get('payload',{})
                assert event['type']!='error',event
                assert p.get('kind')!='failed',event
                identity=event.get('id')
                if identity and identity.startswith('edit-'):
                    i=int(identity.removeprefix('edit-'));h=p['history'];doc=h['document']
                    assert identity not in acks
                    assert doc['revision']==i+2 and doc['source_sha256']==sha(states[i+1].encode())
                    assert h['command_revision']==i+2 and h['replayed_command'] is False
                    acks[identity]=dict(received_ms=(received-t0)*1000,revision=i+2,bytes=len(client.last_wire))
                if p.get('kind')=='completed_snapshot':
                    assert args.historical and p['is_current'] is False and p['source_actions_enabled'] is False
                    revision=p['source_versions']['main.tex']
                    assert 2<=revision<=21
                    assert p['source_binding_token']=='burst-'+str(revision-2)
                    assert p['compile_revision']<p['current_compile_revision']
                    assert not historical or p['compile_revision']>historical[-1]['compile_revision']
                    historical.append(dict(received_ms=(received-t0)*1000,revision=revision,
                        request_id=p['request_id'],compile_revision=p['compile_revision'],
                        latest_ack_revision=max([a['revision'] for a in acks.values()],default=1)))
                if p.get('kind')=='preview':
                    revision=p['source_versions']['main.tex']
                    assert 2<=revision<=21
                    assert revision >= max([a['revision'] for a in acks.values()],default=1), 'preview older than delivered ACK'
                    assert not previews or revision > previews[-1]['revision'], 'preview revision moved backward or repeated'
                    previews.append(dict(received_ms=(received-t0)*1000,revision=revision,
                        latest_ack_revision=max([a['revision'] for a in acks.values()],default=1)))
                    if revision==21:final=p
            sends=sender.finish(timeout=2)
            assert len(sends)==20
            for record in sends:record['id']=requests[record.pop('index')]['id']
            measured_done=time.monotonic()
            final_preview_received_ms=next(p['received_ms'] for p in previews if p['revision']==21)
            assert final['result']['payload']['status']=='ok'
            if args.historical:
                client.send('historical-disable','configure_completed_snapshots',dict(capability='completed-snapshots-v1',enabled=False))
                disabled=reply('historical-disable')
                assert disabled['type']=='result' and disabled['payload']['enabled'] is False
            (out/'final-preview.json').write_text(json.dumps(final,indent=2)+'\n')
            client.send('history','history_status',dict(path='main.tex'))
            status=reply('history')['payload'];assert status['history']['permanent_command_ids']==20
            for i in [0,19]:
                client.send('retry-'+str(i),'apply_group',requests[i]['payload'])
                h=reply('retry-'+str(i))['payload']['history']
                assert h['replayed_command'] is True and h['command_revision']==i+2 and h['document']['revision']==21
                assert h['document']['source_sha256']==sha(states[-1].encode())
            conflict=json.loads(json.dumps(requests[0]['payload']));conflict['command']['label']='conflicting fingerprint'
            client.send('conflict','apply_group',conflict);assert reply('conflict')['type']=='error'
            (out/'diagnostics.json').write_text(json.dumps(client.diagnostics(),indent=2)+'\n')
        finally:
            try:
                if sender:sender.stop()
            finally:
                try:client.stop()
                finally:
                    events.close()
                    (out/'diagnostics-raw.jsonl').write_bytes(client.last_diagnostics_raw[:1024*1024])
                    (out/'diagnostics-capture-status.json').write_text(json.dumps(client.diagnostics_status)+'\n')
                    (out/'receiver-timings.json').write_text(json.dumps(dict(records=client.receiver_timings,dropped=client.receiver_timings_dropped),indent=2)+'\n')
        reopened=Client(args.helper,config,capture_wire=True,capture_diagnostics=True)
        reopen_events=(out/'reopen-events.jsonl').open('wb')
        original_read=reopened.read
        def reopen_read():
            result=original_read()
            reopen_events.write(reopened.last_wire);reopen_events.flush()
            return result
        reopened.read=reopen_read
        try:
            durable=snapshot_after_initial_preview(reopened)
            assert durable['revision']==21 and durable['text']==states[-1]
            assert durable['source_sha256']==sha(states[-1].encode())
        finally:
            try:
                # Preserve raw diagnostics, even an incomplete final stderr line.
                data=reopened.diagnostic_bytes()
                (out/'reopen-diagnostics.jsonl').write_bytes(data[:1024*1024])
                (out/'reopen-capture-status.json').write_text(json.dumps(dict(
                    scope="pre-stop snapshot; terminal stderr may be missing",
                    diagnostics_truncated=len(data)>1024*1024,
                    receiver_records=reopened.receiver_timings,
                    receiver_records_dropped=reopened.receiver_timings_dropped))+'\n')
            finally:
                try:reopened.stop()
                finally:reopen_events.close()
    request_frames=[line for f in (out/'producer').glob('*.input.jsonl') for line in f.read_bytes().splitlines(keepends=True)
        if json.loads(line)['id']==final['request_id']]
    assert len(request_frames)==1
    actual=json.loads(request_frames[0]);assert actual['payload']['documents']==[dict(path='main.tex',text=states[-1])]
    clean=subprocess.run([args.producer],input=request_frames[0],capture_output=True,check=True,timeout=20)
    clean_frames=clean.stdout.splitlines();assert len(clean_frames)==1 and json.loads(clean_frames[0])==final['result']
    (out/'clean.input.jsonl').write_bytes(request_frames[0]);(out/'clean.output.jsonl').write_bytes(clean.stdout)
    if historical:
        delivered=[json.loads(line) for line in (out/'events.jsonl').read_bytes().splitlines()
            if json.loads(line).get('payload',{}).get('kind')=='completed_snapshot']
        verify_historical_results(out/'producer', delivered, states)
    summary=dict(helper_sha256=sha(Path(args.helper).read_bytes()),producer_sha256=evidence['binary_sha256'],assets=evidence['assets'],
        sender_mode='separate-process',historical_opt_in=args.historical,historical=historical,sends=sends,acks=acks,previews=previews,final_revision=21,source_sha256=sha(states[-1].encode()),
        post_last_send_preview_ms=final_preview_received_ms-sends[-1]['sent_ms'],
        measured_until_ms=(measured_done-t0)*1000,target_interval_ms=30,
        scope='one20edit burst with observed send schedule, release components and transparent captureproxy; later receipts/conflict/reopen/cleancompile excluded from measured burst; no native or worstcase bound',
        scripts={p.name:sha(p.read_bytes()) for p in [Path(__file__),Path(__file__).with_name('helper_replay.py'),Path(__file__).with_name('producer_capture_proxy.py'),Path(__file__).with_name('scheduled_sender.py')]},
        artifacts={str(p.relative_to(out)):sha(p.read_bytes()) for p in out.rglob('*') if p.is_file()})
    (out/'provenance.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(json.dumps(dict(ack_count=len(acks),preview_count=len(previews),historical_count=len(historical),historical_revisions=[h['revision'] for h in historical],preview_revisions=[p['revision'] for p in previews],post_last_send_preview_ms=summary['post_last_send_preview_ms'],send_ms=[s['sent_ms'] for s in sends])))

if __name__=='__main__':main()
