#!/usr/bin/env python3
"""kw_sum.py KW.log: for each jump's first key (kw.sh's `[kw]` lines, host and client on one
clock), the host's events from the VIEWPORT to the key's watched PAGE, in ms after the key's
COMPILE was sent (lane MEMORY, measurement only)."""
import re, sys

blocks, cur = [], None
for line in open(sys.argv[1]):
    if line.startswith('==='):
        cur = {'head': line.strip('= \n'), 'lines': []}
        blocks.append(cur)
        continue
    m = re.match(r'\[kw\] (\d+) (.*)', line)
    if m and cur is not None:
        cur['lines'].append((int(m.group(1)), m.group(2)))
# the host's lines of a configuration and round follow its clients' blocks
hosts = {}
for b in blocks:
    if b['head'].startswith('host '):
        hosts[b['head'][5:]] = b['lines']
for b in blocks:
    if not b['head'].startswith('client '):
        continue
    cfg_round, step = b['head'][7:].split(' step ')
    if step.startswith('0'):
        continue
    # (the client's first compile only learns the pages: watching None)
    sent = next((t for t, w in b['lines'] if 'watching Some' in w), None)
    if sent is None:
        continue
    nxt = next((t for t, w in b['lines'] if 'watching Some' in w and t > sent), sent + 400_000)
    page_reads = [(t, w) for t, w in b['lines'] if 'PAGE' in w and sent < t < nxt]
    end = page_reads[-1][0] if page_reads else sent + 200_000
    print(f'--- {cfg_round} step {step}: the first key')
    for t, w in hosts.get(cfg_round, []):
        if sent - 400_000 <= t <= end + 1000:
            print(f'  {(t - sent) / 1e3:9.3f}  {w}')
    for t, w in page_reads:
        print(f'  {(t - sent) / 1e3:9.3f}  {w}')
