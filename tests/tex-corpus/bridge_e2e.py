#!/usr/bin/env python3
"""Run an isolated real bridge library harness and compile its resulting snapshots."""
import argparse
import copy
from datetime import datetime,timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

SPEC=importlib.util.spec_from_file_location('corpus_incremental',Path(__file__).with_name('incremental.py'))
inc=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(inc)


def validate_harness(evidence):
    expected={'included-file-context','macro-context-changed','multibyte-rebase','confirm-restart-idempotency'}
    results=evidence['results']
    if len(results)!=4 or {r['scenario'] for r in results}!=expected:
        raise ValueError('missing or duplicate bridge scenario')
    rebase=next(r for r in results if r['scenario']=='multibyte-rebase')
    edit=rebase['prepared']
    if (edit['start_byte'],edit['end_byte'],edit['expected_revision'])!=(11,11,2):
        raise ValueError('multibyte anchor shifted incorrectly')
    if edit['document_before_sha256']!=hashlib.sha256('東京αβ world'.encode()).hexdigest():
        raise ValueError('prepared snapshot hash mismatch')
    if rebase['source']!='東京αβ $x$world':
        raise ValueError('inserted source differs')
    macro=next(r for r in results if r['scenario']=='macro-context-changed')
    if macro['status']!='unverified' or macro['context_revision']!=1 or macro['prepared']['expected_revision']!=2:
        raise ValueError('stale-context review gap must remain explicit')
    receipt=next(r for r in results if r['scenario']=='confirm-restart-idempotency')
    if receipt['status']!='unverified' or receipt['bridge_receipt_retry']!='pass':
        raise ValueError('bridge retry must not be confused with native crash acceptance')
    return results


def compile_snapshot(request,command):
    process=subprocess.run(command,input=json.dumps(request)+'\n',text=True,capture_output=True,timeout=10)
    result={'request':request,'stdout':process.stdout,'stderr':process.stderr,'returncode':process.returncode}
    if process.returncode:
        raise ValueError('compiler failed')
    reply=json.loads(process.stdout)
    if not inc.correlation(request,reply):
        raise ValueError('compiler response is stale/mismatched')
    p=reply['payload'];docs={d['path']:d['text'] for d in request['payload']['documents']}
    items=[i for page in p['pages'] for i in page['items']]
    result['source_ranges_valid']=all(inc.runner.source_valid(i['source'],docs) for i in items)
    result['compiler_status']=p['status'];result['compiler_diagnostics']=p['diagnostics']
    result['transport_status']='pass' if result['source_ranges_valid'] else 'fail'
    stale=copy.deepcopy(reply);stale['payload']['revision']-=1
    result['stale_response_rejected_by_harness']=not inc.correlation(request,stale)
    result['native_stale_preview_gate']='unverified: no native asynchronous preview exercised'
    return result


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--harness',required=True);p.add_argument('--harness-build-command',required=True)
    p.add_argument('--bridge-sha',required=True);p.add_argument('--compiler',required=True)
    p.add_argument('--compiler-sha',required=True);p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    for sha in (args.bridge_sha,args.compiler_sha):
        if len(sha)!=40 or any(c not in '0123456789abcdef' for c in sha):p.error('exact SHA required')
    run=subprocess.run([args.harness],text=True,capture_output=True,timeout=30,check=True)
    evidence=json.loads(run.stdout);results=validate_harness(evidence)
    for r in results:
        if 'compile_request' in r:r['compiler_evidence']=compile_snapshot(r['compile_request'],[args.compiler])
    results.append({'scenario':'stale-preview-response','status':'unverified','harness_revision_rejection':all(
        r.get('compiler_evidence',{}).get('stale_response_rejected_by_harness',True) for r in results),
        'required_native_gate':'Deliver revision N after N+1 to real app; retain newer preview/source mappings.'})
    output={'schema_version':1,'bridge_sha':args.bridge_sha,'compiler_sha':args.compiler_sha,
            'harness_build_command':args.harness_build_command,'harness_source_sha256':hashlib.sha256(Path(__file__).with_name('bridge_scenarios.rs').read_bytes()).hexdigest(),
            'harness_raw_stdout':run.stdout,'harness_stderr':run.stderr,'created_utc':datetime.now(timezone.utc).isoformat(),
            'limitations':'Real bridge library and compiler; converter and restored source are test doubles. No network/Grok/native app/OS crash test.',
            'results':results}
    args.output.write_text(json.dumps(output,indent=2,ensure_ascii=False)+'\n')
    print(json.dumps({s:sum(r['status']==s for r in results) for s in ('pass','fail','unverified')}))
    return int(any(r['status']=='fail' or r.get('compiler_evidence',{}).get('transport_status')=='fail' for r in results))


if __name__=='__main__':sys.exit(main())
