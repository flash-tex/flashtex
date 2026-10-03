#!/usr/bin/env python3
"""Bounded actual Linux producer bundle-layout probe; no native app claim.

Requires an already built unchanged pinned producer and existing official assets.
No downloads, font installation, source editing, or reference renderer.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

# The verifier moved to apps/mac/scripts in old-engine retirement S1 (#1236).
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "apps/mac/scripts"))
import verify_bundle_resources as verifier  # noqa: E402

SOURCE = "7ca34cec09bc4b06714f7cc58720a5b24923cafa"
CORE = Path(__file__).resolve().parents[1]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", type=Path, required=True, help="owned build directory with exact source.tar and build-provenance.json")
    parser.add_argument("--metrics", type=Path, required=True, help="previously acquired LM2.004 files and LICENSE")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    receipt = json.loads((args.build / "build-provenance.json").read_text())
    assert receipt["source_commit"] == SOURCE
    assert sha((args.build / "source.tar").read_bytes()) == receipt["source_archive_sha256"]
    binary = args.build / "source/crates/render-pipeline/target/release/flashtex-render"
    binary_hash = sha(binary.read_bytes())
    assert receipt["producer_binary_sha256"] == binary_hash
    source_text = (args.build / "source/crates/render-pipeline/src/fonts.rs").read_text()
    defaults = source_text.split("pub const DEFAULT_FONT_DIRS", 1)[1].split("];", 1)[0]
    host_paths = re.findall(r'"(/[^"\n]+)"', defaults)
    assert host_paths and all(not Path(p).exists() for p in host_paths), "host TeX path present; isolated assumption refused"
    args.output.mkdir(parents=True, exist_ok=False)
    manifest = verifier.pinned_manifest()
    fonts = {
        "lmroman10-regular.otf": CORE / "tests/fixtures/helper-multidoc-eca6ab25/lmroman10-regular.otf",
        "lmroman12-regular.otf": CORE / "tests/fixtures/original-reference/lmroman12-regular.otf",
        "latinmodern-math.otf": CORE / "tests/fixtures/math-reference/latinmodern-math.otf",
    }
    requests = {
        "10pt": (CORE / "tests/fixtures/helper-multidoc-eca6ab25/request.jsonl").read_bytes(),
        "12pt": (CORE / "tests/fixtures/original-reference/request.jsonl").read_bytes(),
    }
    results = {}
    # /home-owned build directory, not the nearly-full tmpfs.
    with tempfile.TemporaryDirectory(prefix="linux-bundle-", dir=args.build) as temporary:
        stage = Path(temporary).resolve()
        exe = stage / "FlashTeX.app/Contents/MacOS/flashtex-render"
        resources = exe.parent.parent / "Resources"
        exe.parent.mkdir(parents=True)
        shutil.copyfile(binary, exe)
        exe.chmod(0o755)
        for entry in manifest["required_for_existing_fixtures"]:
            name = Path(entry["path"]).name
            source = fonts.get(name, args.metrics / ("LICENSE" if name == "GUST-FONT-LICENSE.TXT" else name))
            data = source.read_bytes()
            assert len(data) == entry["byte_length"] and sha(data) == entry["sha256"]
            destination = resources / entry["path"]
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(data)
        coverage = verifier.verify(resources)
        assert coverage["status"] == "verified"
        tfms = resources / "texmf/fonts/tfm/public/lm"
        flat = stage / "explicit-flat"
        flat.mkdir()
        for entry in manifest["required_for_existing_fixtures"]:
            if entry["path"].startswith("texmf/"):
                shutil.copyfile(resources / entry["path"], flat / Path(entry["path"]).name)
        originals = {p: p.read_bytes() for p in list(tfms.glob("*.tfm")) + list(flat.iterdir())}

        def run(label, request_key, override=False):
            request = requests[request_key]
            env = {"PATH": "/usr/bin:/bin", "LANG": "C"}
            if override:
                env["FLASHTEX_TFM_DIRS"] = str(flat)
            v2 = args.output.resolve() / (label + ".v2.json")
            completed = subprocess.run([str(exe), "--v2", str(v2)], input=request,
                                       capture_output=True, env=env, cwd=stage, timeout=10)
            (args.output / (label + ".stdout.jsonl")).write_bytes(completed.stdout)
            (args.output / (label + ".stderr.txt")).write_bytes(completed.stderr)
            assert completed.returncode == 0, completed.stderr
            reply = json.loads(completed.stdout.splitlines()[0])
            diagnostics = reply["payload"]["diagnostics"]
            result = {"request_sha256": sha(request), "status": reply["payload"]["status"],
                      "diagnostics": diagnostics, "stdout_sha256": sha(completed.stdout),
                      "v2_sha256": sha(v2.read_bytes()), "explicit_tfm_override": override,
                      "producer_exit": completed.returncode}
            results[label] = result
            return {d["code"] for d in diagnostics}

        assert not run("bundled-10pt", "10pt")
        assert not run("bundled-12pt", "12pt")
        p = tfms / "ec-lmr10.tfm"
        p.unlink()
        assert "tfm_missing" in run("missing-bundled-10pt", "10pt")
        p.write_bytes(originals[p])
        # Same bytes count, invalid TFM table lengths: explicit override selected
        # before a valid bundled non-required10pt metric; never fabricate metrics.
        p = flat / "ec-lmr10.tfm"
        p.write_bytes(b"\x00" * len(originals[p]))
        assert "tfm_missing" in run("corrupt-explicit-10pt", "10pt", True)
        p.write_bytes(originals[p])
        p = flat / "ec-lmr12.tfm"
        p.write_bytes(b"\x00" * len(originals[p]))
        # Required metrics intentionally try rooted sets before flat directories.
        assert not run("corrupt-flat-12pt-valid-root", "12pt", True)
        rooted = tfms / "ec-lmr12.tfm"
        rooted.unlink()
        assert "required_metrics_unavailable" in run("corrupt-flat-12pt-missing-root", "12pt", True)
        p.write_bytes(originals[p])
        assert not run("valid-flat-12pt-missing-root", "12pt", True)
        assert "required_metrics_unavailable" in run("missing-root-12pt-no-override", "12pt")
        rooted.write_bytes(originals[rooted])
        assert verifier.verify(resources)["status"] == "verified"
    report = {"format": "flashtex-linux-bundle-discovery-v1", "source_commit": SOURCE,
              "source_archive_sha256": receipt["source_archive_sha256"],
              "producer_binary_sha256": binary_hash, "manifest_sha256": verifier.MANIFEST_SHA256,
              "script_sha256": sha(Path(__file__).read_bytes()), "toolchain": receipt["toolchain"],
              "host_default_paths_checked_absent": host_paths, "environment": "only PATH/LANG plus declared TFM override; no inherited font/TeX variables",
              "coverage": coverage, "cases": results,
              "scope": "actual unchanged Linux binary in sibling app directory layout; temporary stage removed; no signed app/native paint/reference parity or syscall trace; path-selection evidence comes from controlled missing/corrupt outcomes"}
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({k: v["status"] for k, v in results.items()}))


if __name__ == "__main__":
    main()
