#!/usr/bin/env python3
"""Compares PDFKit word boxes of two PDFs rendered from the same runtime-v1
compile_result (flashtex-pdf vs the CoreText renderer), and both against the
item origins in the JSON. Standard library only.

usage: compare_word_boxes.py compile_result.json a.json b.json [label_a label_b]
where a.json / b.json are `oracle_extract` outputs for the two PDFs.
"""
import json, sys

def load_words(path):
    d = json.load(open(path))
    out = []
    for p in d['pages']:
        for w in p['words']:
            out.append((p['number'], w['text'], w['x_pt'], w['bottom_pt'], w['top_pt'], w['right_pt']))
    return out

def load_items(path):
    d = json.load(open(path))
    out = []
    for p in d['payload']['pages']:
        for it in p['items']:
            if it['kind'] == 'text':
                out.append((p['number'], it['text'], it['x_pt'], it['baseline_y_pt'], it['font_size_pt']))
    return out

def main():
    items = load_items(sys.argv[1])
    a = load_words(sys.argv[2])
    b = load_words(sys.argv[3])
    la, lb = (sys.argv[4], sys.argv[5]) if len(sys.argv) > 5 else ('A', 'B')
    print(f"items {len(items)}; {la} words {len(a)}; {lb} words {len(b)}")
    n = min(len(a), len(b), len(items))
    seq_ok = all(a[i][1] == b[i][1] == items[i][1] for i in range(n)) and len(a) == len(b) == len(items)
    print(f"word sequences identical: {seq_ok}")
    def stats(pairs, name):
        dx = [abs(p[0] - p[1]) for p in pairs]
        return f"{name}: n={len(dx)} mean {sum(dx)/len(dx):.4f} max {max(dx):.4f} pt"
    # A vs B: x, bottom, right.
    print(stats([(a[i][2], b[i][2]) for i in range(n)], f"|x {la} - x {lb}|"))
    print(stats([(a[i][3], b[i][3]) for i in range(n)], f"|bottom {la} - bottom {lb}|"))
    print(stats([(a[i][5], b[i][5]) for i in range(n)], f"|right {la} - right {lb}|"))
    print(stats([(a[i][4], b[i][4]) for i in range(n)], f"|top {la} - top {lb}|"))
    # Each vs the compiler's origins: x exactly; bottom = baseline + descent (font-dependent constant).
    for words, name in ((a, la), (b, lb)):
        print(stats([(words[i][2], items[i][2]) for i in range(n)], f"|x {name} - x item|"))
        desc = [words[i][3] - items[i][3] for i in range(n)]
        print(f"bottom - baseline ({name}): min {min(desc):.4f} max {max(desc):.4f} pt (glyph-box descent of the embedded font at 11.955 bp)")
    worst = sorted(range(n), key=lambda i: -abs(a[i][5] - b[i][5]))[:5]
    print("largest right-edge differences (word, right A, right B):")
    for i in worst:
        print(f"  {a[i][1]!r}: {a[i][5]:.3f} vs {b[i][5]:.3f} ({a[i][5]-b[i][5]:+.3f})")

if __name__ == '__main__':
    main()
