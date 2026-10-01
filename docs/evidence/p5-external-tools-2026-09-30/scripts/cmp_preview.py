import os, sys, subprocess
sys.path.insert(0, os.path.expanduser("~/code/flashtex-p5tools/tools/parity"))
sys.path.insert(0, os.path.expanduser("~/code/flashtex-p5tools/tools/visual-oracle"))
sys.path.insert(0, os.path.expanduser("~/code/flashtex-p5tools/tools/real-world-corpus"))
import tiers
a, b = sys.argv[1], sys.argv[2]
r = tiers.compare_pt2(a, b, "/tmp/p5x-cmpwork")
print({k: r.get(k) for k in ("ok", "why", "pages", "first", "fonts_equal", "pages_equal")})
