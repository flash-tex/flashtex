"""Per text run: pixel diffs inside the run's box and the best sub-pixel
shift (dx, dy) of the DL rendering that matches the PDF rendering."""
import json, sys
import numpy as np
from PIL import Image

d = json.load(open(sys.argv[1]))
S = float(sys.argv[3])
pdf = np.asarray(Image.open(sys.argv[2] + "-pdf.png").convert("L")).astype(float)
dl = np.asarray(Image.open(sys.argv[2] + "-dl.png").convert("L")).astype(float)
H, W = pdf.shape
rows = []
for i, r in enumerate(d["runs"]):
    x0 = r["m"][4]; y0 = r["m"][5]; size = r["size"]
    xs = [g[1] for g in r["g"]]
    x1 = x0 + max(xs) + size
    X0, X1 = int((x0 - 1) * S), int(x1 * S) + 2
    Y0, Y1 = int((y0 - size) * S), int((y0 + 0.35 * size) * S) + 2
    X0, Y0 = max(X0, 0), max(Y0, 0)
    a = pdf[Y0:Y1, X0:X1]; b = dl[Y0:Y1, X0:X1]
    nd = int((np.abs(a - b) > 0).sum())
    # centroid of ink per column/row (sub-pixel shift estimate)
    ia = 255 - a; ib = 255 - b
    if ia.sum() == 0:
        continue
    cx = lambda im: (im.sum(0) * np.arange(im.shape[1])).sum() / im.sum()
    cy = lambda im: (im.sum(1) * np.arange(im.shape[0])).sum() / im.sum()
    rows.append((i, r["font"], size, nd, cx(ia) - cx(ib), cy(ia) - cy(ib), x0, y0))
for row in rows[:40]:
    print("run %3d font %d size %5.2f diffpx %5d  dx(pdf-dl) %+.4f px  dy %+.4f px  origin (%.4f, %.4f)" % row)
