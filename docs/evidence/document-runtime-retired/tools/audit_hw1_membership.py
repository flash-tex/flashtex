"""Hash/source audit only; does not execute the candidate compiler."""
import collections
import gzip
import hashlib
import json
import subprocess
PEER='3354e31381eb3a9e6db68eeecdc326a437152e69'
BASE='crates/preview-controller/docs/handoffs/'
def read(name): return subprocess.check_output(['git','show',PEER+':'+BASE+name])
def sha(b): return hashlib.sha256(b).hexdigest()
def audit():
    evidence=json.loads(read('hw1-membership/evidence.json'))
    for name,digest in evidence['files'].items(): assert sha(read('hw1-membership/'+name))==digest
    raw={n:read(n) for n in ['hw1-starred-sections/request.jsonl.gz','hw1-starred-sections/base.jsonl.gz','hw1-membership/hw1.jsonl']}
    decoded={n:gzip.decompress(b) if n.endswith('.gz') else b for n,b in raw.items()}
    request,base,candidate=[json.loads(decoded[n]) for n in raw]
    source=request['payload']['documents'][0]['text'].encode()
    assert sha(source)=='f725e23897df3c9645bde1ceaab0876d78ea6ef4a44fd0e80d60a5161fac9d9e'
    for reply in [base,candidate]:
        assert reply['id'] in ['hw1-starred-acceptance','hw1-membership']
        for field in ['project_id','revision']: assert reply['payload'][field]==request['payload'][field]
    key=lambda x:json.dumps(x,sort_keys=True)
    old=collections.Counter(map(key,base['payload']['diagnostics']))
    new=collections.Counter(map(key,candidate['payload']['diagnostics']))
    removed=[json.loads(s) for s,n in (old-new).items() for _ in range(n)]
    assert not (new-old) and len(removed)==12
    assert len(base['payload']['diagnostics'])==119 and len(candidate['payload']['diagnostics'])==107
    spans=[]
    for page in candidate['payload']['pages']:
        for item in page['items']:
            if item.get('text')!='∈': continue
            span=item['source']; assert span['path']=='HW1.tex'
            token=source[span['start_byte']:span['end_byte']].decode()
            assert token=='\\in'
            spans.append(span)
    assert len(spans)==12
    assert collections.Counter(key(x['source']) for x in removed)==collections.Counter(map(key,spans))
    assert all('\\in' in x['message'] and x['severity']=='error' for x in removed)
    return {'peer':PEER,'base':evidence['base'],'source_sha256':sha(source),'raw_sha256':{n:sha(b) for n,b in raw.items()},'decoded_sha256':{n:sha(b) for n,b in decoded.items()},'diagnostics':[119,107],'removed':removed,'membership_spans':spans,'request_scope':'shared source request has starred-acceptance ID, not exact candidate stdin; owner confirms original membership request not retained; standalone binary rebuilt in place, hash unavailable','scope':'isolated compiler membership output only; no combined starred/PDF/native/production parity'}
if __name__=='__main__': print(json.dumps(audit(),indent=2,sort_keys=True))
