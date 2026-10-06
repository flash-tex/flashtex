#!/usr/bin/env python3
"""Tests for retirement_refs.py (unittest; scripts/gate.sh's parity self-tests run them)."""
import json
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import retirement_refs as rr  # noqa: E402

FILES = {
    'scripts/gate.sh': 'cmp crates/gone/manifest.json\n',
    'crates/old/src/lib.rs': '// ported from flashtex_gone\n',
    'docs/evidence/x/README.md': 'crates/gone was here\n',
    'apps/clean.swift': 'nothing to see\n',
    'bin.dat': None,  # binary: never read
}
CONFIG = {
    'always_allowed': ['docs/evidence/'],
    'stages': {
        'S2': {'landed': True, 'names': ['crates/gone', 'flashtex_gone'],
               'allow': [{'path': 'crates/old/src/lib.rs', 'until': 'S7', 'why': 'a comment'}]},
        'S6': {'landed': False, 'names': ['apps/clean'], 'allow': []},
    },
}


def check(config=CONFIG, files=FILES):
    return rr.check(config, list(files), files.get)


class RetirementRefsTests(unittest.TestCase):
    def test_a_live_reference_fails_and_says_where(self):
        problems, report = check()
        self.assertEqual(len(problems), 1)
        self.assertIn('scripts/gate.sh names crates/gone', problems[0])
        self.assertEqual(report, ['S2: allowed until S7: crates/old/src/lib.rs: flashtex_gone'])

    def test_always_allowed_roots_and_unlanded_stages_are_skipped(self):
        files = {k: v for k, v in FILES.items() if k != 'scripts/gate.sh'}
        self.assertEqual(check(files=files)[0], [])  # docs/evidence and S6 (not landed) never fail

    def test_a_stale_allow_entry_fails(self):
        files = dict(FILES, **{'crates/old/src/lib.rs': '// rewritten\n'})
        files.pop('scripts/gate.sh')
        problems, _ = check(files=files)
        self.assertEqual(len(problems), 1)
        self.assertIn('stale allow entry crates/old/src/lib.rs', problems[0])

    def test_an_allow_entry_needs_until_and_why(self):
        bad = json.loads(json.dumps(CONFIG))
        bad['stages']['S2']['allow'][0]['why'] = ''
        with self.assertRaises(ValueError):
            check(config=bad)

    def test_the_repository_config_is_well_formed(self):
        cfg = json.load(open(rr.DEFAULT_CONFIG))
        for sid, st in cfg['stages'].items():
            self.assertTrue(st['names'], sid)
            paths = [a['path'] for a in st.get('allow', [])]
            self.assertEqual(len(paths), len(set(paths)), f'{sid}: duplicate allow entries')
            for a in st.get('allow', []):
                self.assertTrue(a.get('until') and a.get('why'), a)


if __name__ == '__main__':
    unittest.main()
