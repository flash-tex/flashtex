#!/usr/bin/env python3
"""Tests for run.py's --shard K/N. Stdlib unittest only; needs no engine.

Run as `python3 -m unittest tools.lockstep.test_shard` from the repo root.
"""
import argparse
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import run as lockstep_run


class ShardTest(unittest.TestCase):
    def test_parse(self):
        self.assertEqual(lockstep_run.parse_shard("2/5"), (2, 5))
        for bad in ("0/3", "4/3", "x/3", "3", "1/0"):
            with self.assertRaises(argparse.ArgumentTypeError, msg=bad):
                lockstep_run.parse_shard(bad)

    def test_shards_partition_the_cases(self):
        names = ["%03d-case" % i for i in range(1, 24)]
        for n in (1, 2, 3, 8, 30):
            parts = [lockstep_run.shard_cases(names, (k, n)) for k in range(1, n + 1)]
            joined = sorted(x for p in parts for x in p)
            self.assertEqual(joined, names, "n=%d" % n)
            self.assertLessEqual(max(map(len, parts)) - min(map(len, parts)), 1)

    def test_no_shard_is_everything(self):
        self.assertEqual(lockstep_run.shard_cases(["a", "b"], None), ["a", "b"])

    def test_every_real_case_lands_in_one_shard(self):
        names = lockstep_run.select_cases([])
        parts = [lockstep_run.shard_cases(names, (k, 6)) for k in range(1, 7)]
        self.assertEqual(sum(map(len, parts)), len(names))
        self.assertEqual(len(set().union(*map(set, parts))), len(names))


if __name__ == "__main__":
    unittest.main()
