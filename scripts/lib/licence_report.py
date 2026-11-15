# Check G helper for scripts/check-license-boundary.sh: reads one
# `cargo metadata --no-deps` JSON file (argv[1]), reports unlicensed packages.
import json, sys

meta = json.load(open(sys.argv[1], encoding="utf-8"))
bad = sorted(p["name"] for p in meta["packages"] if not p.get("license") and not p.get("license_file"))
for b in bad:
    print("FAIL\t%s declares no licence (add license = \"MIT\", or license-file for other terms)" % b)
if not bad:
    print("OK\tall %d packages declare a licence" % len(meta["packages"]))
