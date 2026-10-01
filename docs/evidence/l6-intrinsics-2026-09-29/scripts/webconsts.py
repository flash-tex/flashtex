#!/usr/bin/env python3
"""Evaluate numeric WEB macros of the merged engine WEB, to check the constants
that crates/flashtex-engine/src/intrinsics.rs hard-codes.

usage: webconsts.py MERGED.web NAME...   (MERGED.web from `web2rust ... --emit-web`)
Also knows the capacities web2rust-default.args passes as --macro/--const."""
import re
import sys

OVERRIDES = {'mem_bot': 0, 'mem_top': 4999999, 'hash_size': 615000, 'hash_prime': 522749,
             'max_halfword': 268435455, 'min_halfword': 0, 'min_quarterword': 0,
             'font_max': 9000, 'max_quarterword': 255}


def load(path):
    defs = {}
    for line in open(path, encoding='latin1'):
        m = re.match(r'@d\s+([A-Za-z_][A-Za-z_0-9]*)\s*=\s*([^{=][^{]*)', line)
        if m and not line.startswith('@d ' + m.group(1) + '=='):
            defs.setdefault(m.group(1), m.group(2).strip().rstrip('@;').strip())
    return defs


def ev(name, defs, seen=()):
    if name in OVERRIDES:
        return OVERRIDES[name]
    if name in seen:
        raise ValueError(name)
    e = defs[name]
    e = re.sub(r"@'([0-7]+)", lambda m: str(int(m.group(1), 8)), e)
    e = re.sub(r'@"([0-9A-Fa-f]+)', lambda m: str(int(m.group(1), 16)), e)
    e = re.sub(r'[A-Za-z_][A-Za-z_0-9]*', lambda m: str(ev(m.group(0), defs, seen + (name,))), e)
    return int(eval(e.replace('div', '//')))


def main():
    defs = load(sys.argv[1])
    for n in sys.argv[2:]:
        try:
            print(f"{n} = {ev(n, defs)}")
        except (KeyError, ValueError, SyntaxError) as e:
            print(f"{n} = ? ({e})")


if __name__ == '__main__':
    main()
