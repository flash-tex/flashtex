#!/usr/bin/env python3
"""Unit tests for tools/fuzz/parsers/type1.py. Stdlib unittest only.

Run as `python3 -m unittest discover -s tools/fuzz` from the repo root.
Engines are small shell scripts written to a temp dir; the real candidate
is never used.
"""
import json
import os
import random
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import type1

MARKER = b"TYPE1MARKER123"
SEED_M = ("mark.pfb", b"\x80\x01\x10\x00\x00\x00hello " + MARKER + b"!!"
          + b"\x80\x03\x00\x00\x00\x00")
SEED_P = ("plain.pfb", b"\x80\x01\x05\x00\x00\x00hello"
          + b"\x80\x03\x00\x00\x00\x00")
SEED_E = ("eexec.pfb", b"\x80\x01\x05\x00\x00\x00hello"
          + b"\x80\x02\x04\x00\x00\x00\xde\xad\xbe\xef"
          + b"\x80\x03\x00\x00\x00\x00")
TFM = b"fake-tfm"

PANIC_BODY = ("#!/bin/sh\n"
              "if grep -q 'TYPE1MARKER123' fuzz.pfb 2>/dev/null; then\n"
              "  echo \"thread 'main' panicked at 't1 boom', src/t1.rs:42:7\""
              " >&2\n"
              "  exit 101\n"
              "fi\n"
              "exit 0\n")
ALWAYS_CRASH = ("#!/bin/sh\necho \"thread 'main' panicked at src/x.rs:1:2\""
                " >&2\nexit 101\n")
ALWAYS_OK = "#!/bin/sh\nexit 0\n"
ALWAYS_FAIL = "#!/bin/sh\nexit 1\n"
SLEEPER = "#!/bin/sh\nsleep 30\nexit 0\n"


def make_engine(tmpdir, name, body):
    path = os.path.join(tmpdir, name)
    with open(path, "w") as fh:
        fh.write(body)
    os.chmod(path, 0o755)
    return path


class Type1Test(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="t1test-")
        self.out = os.path.join(self.tmp, "out")

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_mutate_deterministic(self):
        data = SEED_M[1]
        self.assertEqual(type1.mutate_with_info(data, random.Random(9)),
                         type1.mutate_with_info(data, random.Random(9)))

    def test_format_aware_edits_cover_headers_and_eexec(self):
        kinds = set()
        for s in range(60):
            _b, desc = type1.mutate_with_info(SEED_E[1], random.Random(s))
            kinds.add(desc.split("@")[0].split(":")[0])
        self.assertTrue({"segtype", "seglen", "eexec-trunc"} <= kinds)

    def test_deterministic_counts_and_files(self):
        cand = make_engine(self.tmp, "cand", PANIC_BODY)
        kw = dict(seeds=[SEED_M, SEED_P], tfm=TFM, timeout=10)
        c1 = type1.run_fuzz(cand, 12, 1234, self.out + "1", **kw)
        c2 = type1.run_fuzz(cand, 12, 1234, self.out + "2", **kw)
        self.assertEqual(c1, c2)
        self.assertEqual(sum(c1.values()), 12)
        snap = lambda d: sorted(os.listdir(os.path.join(d, "crash"))) \
            if os.path.isdir(os.path.join(d, "crash")) else []
        self.assertEqual(snap(self.out + "1"), snap(self.out + "2"))
        self.assertGreater(c1["crash"], 0)

    def test_class_counts(self):
        ok = make_engine(self.tmp, "ok", ALWAYS_OK)
        self.assertEqual(type1.run_fuzz(
            ok, 4, 1, self.out, timeout=10, seeds=[SEED_P], tfm=TFM),
            {"crash": 0, "hang": 0, "ok": 4, "graceful-error": 0,
             "output-flood": 0})
        fail = make_engine(self.tmp, "fail", ALWAYS_FAIL)
        self.assertEqual(type1.run_fuzz(
            fail, 3, 1, self.out, timeout=10, seeds=[SEED_P], tfm=TFM)
            ["graceful-error"], 3)
        crash = make_engine(self.tmp, "crash", ALWAYS_CRASH)
        self.assertEqual(type1.run_fuzz(
            crash, 3, 1, self.out, timeout=10, seeds=[SEED_P], tfm=TFM)
            ["crash"], 3)

    def test_signal_signature_uses_stderr_first_line(self):
        abrt = make_engine(
            self.tmp, "abrt",
            "#!/bin/sh\n"
            "echo 'fatal runtime error: stack overflow 7, aborting' >&2\n"
            "kill -ABRT $$\n")
        cls, rc, _out, err = type1.run_once(SEED_P[1], TFM, abrt, 10)
        self.assertEqual(cls, "crash")
        self.assertEqual(rc, -6)
        self.assertEqual(
            type1.signature(cls, rc, _out, err),
            "signal:SIGABRT:fatal runtime error: stack overflow N,"
            " aborting")

    def test_hang(self):
        sleepy = make_engine(self.tmp, "sleep", SLEEPER)
        counts = type1.run_fuzz(sleepy, 1, 1, self.out, timeout=0.2,
                                seeds=[SEED_P], tfm=TFM)
        self.assertEqual(counts["hang"], 1)
        names = os.listdir(os.path.join(self.out, "hang"))
        self.assertTrue(any(n.endswith(".pfb") for n in names))
        self.assertTrue(any(n.endswith(".json") for n in names))

    def test_artifacts(self):
        crash = make_engine(self.tmp, "crash", ALWAYS_CRASH)
        type1.run_fuzz(crash, 6, 77, self.out, timeout=10, seeds=[SEED_P],
                       tfm=TFM)
        cdir = os.path.join(self.out, "crash")
        blobs = sorted(n for n in os.listdir(cdir) if n.endswith(".pfb"))
        self.assertTrue(blobs)
        for blob in blobs:
            with open(os.path.join(cdir, blob[:-4] + ".json")) as fh:
                info = json.load(fh)
            for key in ("seed", "mutation", "returncode",
                        "last_stderr_line", "panic_location"):
                self.assertIn(key, info)
            self.assertEqual(info["returncode"], 101)
            self.assertIn("src/x.rs", info["panic_location"])
        with open(os.path.join(self.out, "signatures.json")) as fh:
            sigs = json.load(fh)
        self.assertIn("panic:src/x.rs:1", sigs)


def make_seed_pfb():
    """Tiny PFB whose eexec block is really encrypted (r=55665) and holds
    two Subrs plus two CharStrings encrypted with r=4330/lenIV=4."""
    def cs(code):
        return type1.t1_encrypt(b"\x11\x22\x33\x44" + code, type1.CS_R)
    subr0 = cs(type1.encode_num(0) + b"\x0b")
    subr1 = cs(type1.encode_num(7) + type1.encode_num(8) + b"\x0b")
    notdef = cs(type1.encode_num(10) + type1.encode_num(20) + b"\x0d\x0e")
    glyph = cs(type1.encode_num(0) + b"\x0a"
               + type1.encode_num(3) + b"\x05\x0e")
    plain = (b"dup /Private 5 dict dup begin\n"
             b"dup 0 %d RD " % len(subr0) + subr0 + b" NP\n"
             b"dup 1 %d RD " % len(subr1) + subr1 + b" NP\n"
             b"2 index /CharStrings 2 dict dup begin\n"
             b"/.notdef %d RD " % len(notdef) + notdef + b" ND\n"
             b"/a %d RD " % len(glyph) + glyph + b" ND\n"
             b"end end\n")
    eexec = type1.t1_encrypt(plain, type1.EEXEC_R)
    head = b"%!PS-AdobeFont-1.0 tiny\n"
    return (b"\x80\x01" + len(head).to_bytes(4, "little") + head
            + b"\x80\x02" + len(eexec).to_bytes(4, "little") + eexec
            + b"\x80\x03\x00\x00\x00\x00")


def check_pfb_valid(tc, data):
    """Segments tile contiguously, the eexec block decrypts, and every
    RD declared length matches its body."""
    segs = type1.parse_segments(data)
    tc.assertTrue(segs)
    pos = 0
    twos = None
    for (soff, typ, ln) in segs:
        tc.assertEqual(soff, pos)
        if typ == 3:
            pos += 6
            break
        pos += 6 + ln
        if typ == 2:
            twos = (soff, ln)
    tc.assertTrue(data[pos:] in (b"", b"\x80\x03"))
    tc.assertIsNotNone(twos)
    soff, ln = twos
    plain = type1.t1_decrypt(data[soff + 6:soff + 6 + ln], type1.EEXEC_R)
    entries = type1.find_cs_entries(plain)
    tc.assertTrue(entries)
    for ent in entries:
        bs, be = ent["body"]
        tc.assertEqual(be - bs, ent["len"])
    return plain, entries


class CharStringTest(unittest.TestCase):
    def setUp(self):
        self.seed = make_seed_pfb()
        self.segs = type1.parse_segments(self.seed)

    def test_crypt_roundtrips(self):
        blobs = [b"", b"\x00", bytes(range(256)), b"hello" * 100,
                 bytes((0xD9, 0xD6) * 300)]
        for r in (type1.EEXEC_R, type1.CS_R):
            for blob in blobs:
                self.assertEqual(
                    type1.t1_encrypt(type1.t1_decrypt(blob, r), r), blob)
                self.assertEqual(
                    type1.t1_decrypt(type1.t1_encrypt(blob, r), r), blob)

    def test_encode_num_roundtrip(self):
        for v in type1.CS_BOUNDARIES + type1.HUGE_SUBRS + (42, -42,):
            toks = type1.cs_tokens(type1.encode_num(v))
            self.assertEqual(len(toks), 1)
            self.assertEqual(toks[0][:2], ("num", v))

    def test_eexec_roundtrip_real_seed(self):
        path = type1._kpse("cmr10.pfb")
        if path is None:
            self.skipTest("no cmr10.pfb in the TeX tree")
        with open(path, "rb") as fh:
            data = fh.read()
        segs = type1.parse_segments(data)
        twos = [s for s in segs if s[1] == 2]
        self.assertTrue(twos)
        off, _typ, ln = twos[0]
        seg = data[off + 6:off + 6 + ln]
        self.assertEqual(type1.t1_encrypt(type1.t1_decrypt(seg)), seg)
        plain = type1.t1_decrypt(seg)
        entries = type1.find_cs_entries(plain)
        self.assertGreater(len(entries), 100)
        for ent in entries[:8]:
            bs, be = ent["body"]
            body = plain[bs:be]
            self.assertEqual(
                type1.t1_encrypt(type1.t1_decrypt(body, type1.CS_R),
                                 type1.CS_R), body)

    def test_eexec_roundtrip_synthetic(self):
        off, _typ, ln = [s for s in self.segs if s[1] == 2][0]
        seg = self.seed[off + 6:off + 6 + ln]
        self.assertEqual(type1.t1_encrypt(type1.t1_decrypt(seg)), seg)
        plain, entries = check_pfb_valid(self, self.seed)
        self.assertEqual(len(entries), 4)

    def test_kinds_keep_container_valid(self):
        for i, kind in enumerate(
                ("operand", "operator", "subr", "recursion", "endchar")):
            got = type1._m_charstring(self.seed, random.Random(100 + i),
                                      self.segs, kind)
            self.assertIsNotNone(got, kind)
            out, desc = got
            self.assertTrue(desc.startswith("cs-" + kind + "@"), desc)
            check_pfb_valid(self, out)

    def test_mutate_code_properties(self):
        code = (type1.encode_num(10) + type1.encode_num(20) + b"\x0d\x0e")
        out, _ = type1.mutate_cs_code(code, random.Random(3), "operand")
        vals = [v for k, v, _s, _e in type1.cs_tokens(out) if k == "num"]
        self.assertEqual(len(vals), 2)
        self.assertTrue(any(v in type1.CS_BOUNDARIES for v in vals))
        out, desc = type1.mutate_cs_code(code, random.Random(4),
                                         "operator")
        self.assertIn(desc.split("->")[1], type1.CS_OPS)
        out, desc = type1.mutate_cs_code(code, random.Random(5), "subr")
        self.assertIn(b"\x0a", out)
        self.assertIn("callsubr", desc)
        out, desc = type1.mutate_cs_code(code, random.Random(6),
                                         "endchar")
        self.assertEqual(desc, "dropped-endchar")
        self.assertFalse(out.endswith(b"\x0e"))

    def test_recursion_is_self_call(self):
        out, desc = type1._m_charstring(self.seed, random.Random(11),
                                        self.segs, "recursion")
        idx = int(desc.rsplit("->", 1)[1])
        plain, _entries = check_pfb_valid(self, out)
        for ent in _entries:
            if ent["subr"] == str(idx).encode():
                bs, be = ent["body"]
                body = type1.t1_decrypt(plain[bs:be], type1.CS_R)
                self.assertEqual(body[type1.LENIV:],
                                 type1.encode_num(idx) + b"\x0a")
                return
        self.fail("mutated subr %d not found" % idx)

    def test_cs_kinds_surface(self):
        kinds = set()
        for s in range(200):
            _b, desc = type1.mutate_with_info(self.seed,
                                              random.Random(1000 + s))
            kinds.add(desc.split("@")[0].split(":")[0])
        self.assertTrue({"cs-operand", "cs-operator", "cs-subr",
                         "cs-recursion", "cs-endchar"} <= kinds)

    def test_deterministic(self):
        self.assertEqual(
            type1._m_charstring(self.seed, random.Random(21), self.segs,
                                "operand"),
            type1._m_charstring(self.seed, random.Random(21), self.segs,
                                "operand"))


if __name__ == "__main__":
    unittest.main()
