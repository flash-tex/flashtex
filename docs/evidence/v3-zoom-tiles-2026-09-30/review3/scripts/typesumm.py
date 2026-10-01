import json, os, sys
base = '/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad/a38-typebench/'
for label in sys.argv[1:]:
    p = base + label + '/typing.json'
    if not os.path.exists(p):
        print(label, 'NO RESULT', open(base + label + '/env.txt').read().splitlines()[-1] if os.path.exists(base + label + '/env.txt') else '')
        continue
    d = json.load(open(p))
    f = lambda k: '-' if d.get(k) is None else f"{d[k]:.1f}"
    print(f"{label}: ppp {d['pixelsPerPoint']} keys {d['keystrokes']} samples {d['samples']} pending {d['pending']} offscreen {d['offscreen']} "
          f"p50 {f('p50Ms')} p95 {f('p95Ms')} max {f('maxMs')} ms; disk {d.get('diskBytesWritten')} B; footprint {d.get('footprintMaxBytes', 0)/1e6:.0f} MB; "
          f"kept {d.get('keptRasterBytesMax', 0)/1e6:.0f} MB; deferred {d.get('deferredSources')}; jobs {d.get('tileJobs')}; status {d['status']}")
