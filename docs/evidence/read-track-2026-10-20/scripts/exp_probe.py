#!/usr/bin/env python3
"""Measurement-only probe (never committed): with RT_PROBE set, the Classic convergence test
goes past "an input file's lookahead differs" so that same_words reports what else differs."""
import sys
p = f'{sys.argv[1]}/crates/flashtex-engine/src/incr.rs'
s = open(p).read()
old = """            {
                return Err("an input file's lookahead differs".into());
            }"""
new = """            {
                if std::env::var_os("RT_PROBE").is_none() {
                    return Err("an input file's lookahead differs".into());
                }
                eprintln!("[rt] lookahead differs: old {:?} new {:?}", (a.buf, a.pos, a.have_line, a.at_eof), (b.buf, b.pos, b.have_line, b.at_eof));
            }"""
assert old in s
s = s.replace(old, new, 1)
open(p, 'w').write(s)
print('patched')
p2 = f'{sys.argv[1]}/crates/flashtex-engine/src/incr.rs'
s = open(p2).read()
old = """    let left: Vec<_> = left.into_iter().filter(|w| w.region != "dl_side").collect();
    if side_any && !relabel {"""
new = """    let left: Vec<_> = left.into_iter().filter(|w| w.region != "dl_side").collect();
    let left: Vec<_> = if std::env::var_os("RT_PROBE").is_some() {
        let (dorm, rest): (Vec<_>, Vec<_>) = left.into_iter().partition(|w| {
            w.region == "buffer" || matches!(w.scalar, Some("first") | Some("last"))
        });
        eprintln!("[rt] set aside {} buffer/first/last words", dorm.len());
        rest
    } else {
        left
    };
    if side_any && !relabel {"""
assert old in s
s = s.replace(old, new, 1)
open(p2, 'w').write(s)
print('patched 2')
p3 = f'{sys.argv[1]}/crates/flashtex-engine/src/iso.rs'
s = open(p3).read()
old = """            self.eq("start", x.start_field, y.start_field);
            self.eq("loc", x.loc_field, y.loc_field);
            self.eq("limit", x.limit_field, y.limit_field);"""
new = """            self.eq("start", x.start_field, y.start_field);
            self.eq("loc", x.loc_field, y.loc_field);
            if std::env::var_os("RT_PROBE").is_none() {
                self.eq("limit", x.limit_field, y.limit_field);
            }"""
assert old in s
s = s.replace(old, new, 1)
open(p3, 'w').write(s)
print('patched 3')
s = open(p3).read()
old = """            } else {
                self.eq("limit", x.limit_field, y.limit_field);
            }
            let _ = PARAMETER;"""
new = """            } else if std::env::var_os("RT_PROBE").is_none() {
                self.eq("limit", x.limit_field, y.limit_field);
            }
            let _ = PARAMETER;"""
assert old in s
s = s.replace(old, new, 1)
open(p3, 'w').write(s)
print('patched 4')
