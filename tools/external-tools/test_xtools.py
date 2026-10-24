#!/usr/bin/env python3
"""Tests for xtools.Host's client model: `held_pages()` resolves each page
with the bindings in force when it arrived (display-list-v3 §5), not the
final ones (#1328).

No host process runs: a Host is built without one and fed messages
through `cycle()` exactly as they would come off the socket.

    python3 -m unittest -v test_xtools      (from tools/external-tools)
"""
import json
import os
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import xtools  # noqa: E402

KEY_A, KEY_B, KEY_C = b"A" * 32, b"B" * 32, b"C" * 32


def font(fid, key):
    info = json.dumps({"tex_name": key[:1].decode()}).encode()
    return "font", struct.pack("<I", fid) + key + struct.pack("<I", len(info)) + info


def image(iid, f):
    return "image", {"id": iid, "file": f, "type": "png", "page": 1, "width": 10, "height": 10}


def body(idx, items):
    """A PAGE/FORM body: the 120-byte header with the index, then one
    ITEMS section."""
    hdr = bytearray(120)
    struct.pack_into("<I", hdr, 0, idx)
    struct.pack_into("<ii", hdr, 8, 612, 792)
    return bytes(hdr) + struct.pack("<I", 1) + struct.pack("<II", 3, len(items)) + items


def glyph(fid, code=65):
    return b"\x01" + struct.pack("<HHiiH", fid, code, 0, 0, 0)


def draw_image(iid):
    return b"\x05" + struct.pack("<II", iid, 0)


def draw_form(fid):
    return b"\x06" + struct.pack("<II", fid, 0)


def page(idx, items):
    return "page", body(idx, items)


def form(fid, items):
    return "form", body(fid, items)


SETTLED = ("tool", {"event": "settled", "id": 1})


def held(*messages):
    """The held_pages() of a client that is sent `messages` in one cycle."""
    h = xtools.Host.__new__(xtools.Host)
    h.init_client_state()
    queue = [("started", {"keep": False}), *messages, ("done", {"id": 1}), SETTLED]
    h.send = lambda k, obj: None
    h.recv = lambda: queue.pop(0)
    h.cycle({"id": 1, "root": "/r", "main": "main.tex"})
    return h.held_pages(), h


class HeldPagesTest(unittest.TestCase):
    def test_a_wrong_font_corrected_later_is_still_a_mismatch(self):
        # The candidate sends page 0 while id 1 stands for the wrong font,
        # then corrects the binding: the client keeps drawing page 0 with
        # the wrong font, so it must not compare equal to a fresh host.
        cand, _ = held(font(1, KEY_A), page(0, glyph(1)), font(1, KEY_B), page(1, glyph(1)))
        fresh, _ = held(font(1, KEY_B), page(0, glyph(1)), page(1, glyph(1)))
        self.assertNotEqual(cand[0], fresh[0])
        self.assertEqual(cand[1], fresh[1])

    def test_a_wrong_image_corrected_later_is_still_a_mismatch(self):
        cand, _ = held(image(4, "wrong.png"), page(0, draw_image(4)), image(4, "fig.png"), page(1, draw_image(4)))
        fresh, _ = held(image(4, "fig.png"), page(0, draw_image(4)), page(1, draw_image(4)))
        self.assertNotEqual(cand[0], fresh[0])
        self.assertEqual(cand[1], fresh[1])

    def test_a_rebinding_does_not_change_pages_already_held(self):
        # Id 1 is reused for another font between the pages, which the
        # spec allows: each page keeps what its ids meant when it came, so
        # this equals a host that used two ids.
        cand, _ = held(font(1, KEY_A), page(0, glyph(1)), font(1, KEY_B), page(1, glyph(1)))
        fresh, _ = held(font(1, KEY_A), page(0, glyph(1)), font(2, KEY_B), page(1, glyph(2)))
        self.assertEqual(cand, fresh)

    def test_a_form_sent_after_its_page_resolves(self):
        cand, h = held(font(1, KEY_A), page(0, draw_form(7)), form(7, glyph(1)))
        fresh, _ = held(font(1, KEY_A), form(7, glyph(1)), page(0, draw_form(7)))
        self.assertEqual(cand, fresh)
        self.assertEqual(h.violations, [])

    def test_a_form_keeps_the_font_bound_when_it_arrived(self):
        # The form is drawn with key A although id 1 is rebound to B before
        # the next page; a host that never rebound draws the same.
        cand, _ = held(font(1, KEY_A), form(7, glyph(1)), page(0, draw_form(7)),
                       font(1, KEY_B), page(1, glyph(1) + draw_form(7)))
        fresh, _ = held(font(1, KEY_A), form(7, glyph(1)), page(0, draw_form(7)),
                        font(2, KEY_B), page(1, glyph(2) + draw_form(7)))
        self.assertEqual(cand, fresh)
        # ... and a form whose font was wrong when it came is a mismatch
        wrong, _ = held(font(1, KEY_C), form(7, glyph(1)), page(0, draw_form(7)),
                        font(1, KEY_A), page(1, glyph(2)))
        right, _ = held(font(1, KEY_A), form(7, glyph(1)), page(0, draw_form(7)), page(1, glyph(2)))
        self.assertNotEqual(wrong[0], right[0])

    def test_held_pages_drops_pages_past_a_complete_count(self):
        cand, _ = held(font(1, KEY_A), page(0, glyph(1)), page(1, glyph(1)), ("pages", {"complete": True, "count": 1}))
        self.assertEqual(sorted(cand), [0])


if __name__ == "__main__":
    unittest.main()
