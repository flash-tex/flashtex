"""corpus.py's download path: the content-addressed blob store that keeps
verified e-prints between runs, and the retries (429 with Retry-After, a cut
read resumed with a range request, giving up after FETCH_TRIES). No network:
corpus.http_get and corpus.http_range are replaced, and so is time.sleep."""

import http.client
import io
import json
import os
import sys
import tempfile
import unittest
import urllib.error
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import corpus  # noqa: E402

GOOD = b"a pinned e-print, all of it" * 50
SHA = corpus.sha256_bytes(GOOD)


def http_error(code, retry_after=None):
    headers = {"Retry-After": str(retry_after)} if retry_after is not None else {}
    return urllib.error.HTTPError("https://x/e-print/1", code, "throttled", headers, io.BytesIO(b""))


class Net:
    """Scripted answers for http_get and http_range, in call order. An answer
    is an exception to raise, bytes (http_get), or (offset, bytes) (http_range)."""

    def __init__(self, *answers):
        self.answers, self.calls, self.sleeps = list(answers), [], []

    def _next(self):
        a = self.answers.pop(0)
        if isinstance(a, BaseException):
            raise a
        return a

    def get(self, url, timeout=120):
        self.calls.append(("get", url))
        return self._next()

    def rng(self, url, start, timeout=120):
        self.calls.append(("range", start))
        return self._next()

    def patches(self):
        return [mock.patch.object(corpus, "http_get", self.get), mock.patch.object(corpus, "http_range", self.rng),
                mock.patch.object(corpus.time, "sleep", self.sleeps.append)]

    def __enter__(self):
        self._p = self.patches()
        for p in self._p:
            p.start()
        return self

    def __exit__(self, *exc):
        for p in self._p:
            p.stop()


def cut(partial, offset=None):
    e = http.client.IncompleteRead(partial, 99)
    if offset is not None:
        e.offset = offset
    return e


def no_wait():
    return None


class BlobStore(unittest.TestCase):
    def test_miss_hit_and_a_corrupt_entry_is_dropped(self):
        with tempfile.TemporaryDirectory() as d:
            blobs = os.path.join(d, "blobs")
            self.assertIsNone(corpus.read_blob(blobs, SHA))
            corpus.write_blob(blobs, SHA, GOOD)
            self.assertEqual(corpus.read_blob(blobs, SHA), GOOD)
            with open(os.path.join(blobs, SHA), "wb") as f:  # a truncated copy
                f.write(GOOD[:10])
            self.assertIsNone(corpus.read_blob(blobs, SHA))
            self.assertFalse(os.path.exists(os.path.join(blobs, SHA)))

    def test_bytes_of_another_hash_are_refused_and_names_must_be_hashes(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(corpus.ShaMismatch):
                corpus.write_blob(d, SHA, GOOD + b"x")
            self.assertEqual(os.listdir(d), [])
            self.assertIsNone(corpus.read_blob(d, "../etc/passwd"))

    def test_writes_are_atomic_and_leave_no_temporary_file(self):
        with tempfile.TemporaryDirectory() as d:
            corpus.write_blob(d, SHA, GOOD)
            self.assertEqual(os.listdir(d), [SHA])
            with mock.patch.object(corpus.os, "replace", side_effect=OSError("disk full")):
                with self.assertRaises(OSError):
                    corpus.write_atomic(os.path.join(d, "other"), b"x")
            self.assertEqual(os.listdir(d), [SHA])  # neither `other` nor its temporary file

    def test_blob_dir_follows_the_environment(self):
        with mock.patch.dict(os.environ, {"FLASHTEX_PARITY_BLOBS": "/store"}):
            self.assertEqual(corpus.blob_dir("/c"), "/store")
        with mock.patch.dict(os.environ, {"FLASHTEX_PARITY_BLOBS": ""}):
            self.assertEqual(corpus.blob_dir("/c"), os.path.join("/c", "blobs"))


class FetchManifestUsesTheStore(unittest.TestCase):
    def manifest(self, d):
        man = os.path.join(d, "m.json")
        with open(man, "w") as f:
            json.dump({"tier": "nightly-5k", "entries": [{"id": "2207.06626v1", "url": "https://x/e-print/2207.06626v1",
                                                          "sha256": SHA}]}, f)
        return man

    def fetch(self, man, cache):
        with mock.patch.object(corpus, "unpack", lambda data, dest: os.makedirs(dest, exist_ok=True)):
            return corpus.fetch_manifest(man, cache, log=lambda *_: None, delay=0)

    def test_a_fetch_is_kept_by_content_and_a_new_cache_reads_it_with_no_request(self):
        with tempfile.TemporaryDirectory() as d, \
                mock.patch.dict(os.environ, {"FLASHTEX_PARITY_BLOBS": os.path.join(d, "store")}):
            man = self.manifest(d)
            with Net(GOOD) as net:
                (rec,) = self.fetch(man, os.path.join(d, "run1"))
            self.assertIsNone(rec["problem"])
            self.assertEqual(len(net.calls), 1)
            self.assertEqual(corpus.read_blob(os.path.join(d, "store"), SHA), GOOD)
            # the next run's cache is empty; the store answers, the network is never asked
            with Net() as net:
                (rec,) = self.fetch(man, os.path.join(d, "run2"))
            self.assertIsNone(rec["problem"])
            self.assertEqual(net.calls, [])
            with open(os.path.join(d, "run2", "eprints", "2207.06626v1"), "rb") as f:
                self.assertEqual(f.read(), GOOD)

    def test_a_corrupt_store_entry_is_fetched_again(self):
        with tempfile.TemporaryDirectory() as d, \
                mock.patch.dict(os.environ, {"FLASHTEX_PARITY_BLOBS": os.path.join(d, "store")}):
            os.makedirs(os.path.join(d, "store"))
            with open(os.path.join(d, "store", SHA), "wb") as f:
                f.write(GOOD[:-1])
            with Net(GOOD) as net:
                (rec,) = self.fetch(self.manifest(d), os.path.join(d, "c"))
            self.assertIsNone(rec["problem"])
            self.assertEqual(len(net.calls), 1)
            self.assertEqual(corpus.read_blob(os.path.join(d, "store"), SHA), GOOD)


class Retries(unittest.TestCase):
    def test_429_waits_at_least_its_retry_after(self):
        with Net(http_error(429, retry_after=700), GOOD) as net:
            self.assertEqual(corpus.fetch_verified("u", SHA, no_wait), GOOD)
        self.assertEqual(len(net.sleeps), 1)
        self.assertGreaterEqual(net.sleeps[0], 700)

    def test_429_without_retry_after_still_backs_off_minutes(self):
        with Net(http_error(429), http_error(503), GOOD) as net:
            self.assertEqual(corpus.fetch_verified("u", SHA, no_wait), GOOD)
        self.assertEqual(net.sleeps, [300, 600])

    def test_a_cut_read_is_resumed_with_a_range_request(self):
        half = len(GOOD) // 2
        with Net(cut(GOOD[:half]), (half, GOOD[half:])) as net:
            self.assertEqual(corpus.fetch_verified("u", SHA, no_wait), GOOD)
        self.assertEqual(net.calls, [("get", "u"), ("range", half)])

    def test_cut_reads_accumulate_over_ranged_attempts(self):
        a, b = 100, 300
        with Net(cut(GOOD[:a]), cut(GOOD[a:b], offset=a), (b, GOOD[b:])) as net:
            self.assertEqual(corpus.fetch_verified("u", SHA, no_wait), GOOD)
        self.assertEqual(net.calls, [("get", "u"), ("range", a), ("range", b)])

    def test_a_server_that_ignores_the_range_replaces_the_partial_bytes(self):
        with Net(cut(b"stale prefix"), (0, GOOD)):
            self.assertEqual(corpus.fetch_verified("u", SHA, no_wait), GOOD)

    def test_a_spliced_body_of_another_hash_starts_again_from_scratch(self):
        with Net(cut(b"wrong"), (5, GOOD[5:]), GOOD) as net:
            self.assertEqual(corpus.fetch_verified("u", SHA, no_wait), GOOD)
        self.assertEqual(net.calls, [("get", "u"), ("range", 5), ("get", "u")])

    def test_gives_up_after_fetch_tries_and_raises_the_last_error(self):
        answers = [http_error(429)] * (corpus.FETCH_TRIES - 1) + [cut(b"x")]
        with Net(*answers) as net, self.assertRaises(http.client.IncompleteRead):
            corpus.fetch_verified("u", SHA, no_wait)
        self.assertEqual(len(net.calls), corpus.FETCH_TRIES)
        self.assertEqual(len(net.sleeps), corpus.FETCH_TRIES - 1)
        self.assertEqual(net.answers, [])

    def test_404_is_not_retried(self):
        with Net(http_error(404), GOOD) as net, self.assertRaises(urllib.error.HTTPError):
            corpus.fetch_verified("u", SHA, no_wait)
        self.assertEqual(len(net.calls), 1)


if __name__ == "__main__":
    unittest.main()
