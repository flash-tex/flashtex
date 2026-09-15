#!/usr/bin/env python3
"""Render the FlashTeX website for a GitHub release.

    python3 site/render.py OUT_DIR [--tag vX.Y.Z]

Copies site/ into OUT_DIR, filling {{TAG}}, {{DATE}}, {{SIZE}}, {{SHA256}}
and the {{CLI_*}} placeholders in the templated files from the release's
FlashTeX.dmg, flashtex-cli-<version>-<platform>.tar.gz assets and
SHA256SUMS. Without --tag it uses the latest published release (drafts and
prereleases are ignored). A `{{#if HAS_LINUX_CLI}}...{{/if}}` block is kept
only when the release has a Linux CLI tarball. Needs an authenticated `gh`;
in GitHub Actions, GH_TOKEN is enough.
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from datetime import datetime
from pathlib import Path
from zoneinfo import ZoneInfo

DMG_ASSET = "FlashTeX.dmg"
SUMS_ASSET = "SHA256SUMS"
CLI_PLATFORMS = {"macos-arm64": "CLI_MACOS_ARM64", "linux-x86_64": "CLI_LINUX_X86_64"}
REQUIRED_CLI_PLATFORMS = {"macos-arm64"}  # the macOS job always packages the CLI; Linux may fail independently
TEMPLATED = ("index.html", "download/index.html", "install.sh", "install-cli.sh")
NOT_PUBLISHED = {"render.py", "README.md"}
CONDITIONAL_RE = re.compile(r"{{#if (\w+)}}(.*?){{/if}}", re.DOTALL)


def gh_api(endpoint):
    result = subprocess.run(["gh", "api", endpoint], check=True, capture_output=True, text=True)
    return json.loads(result.stdout)


def cli_asset_name(version, platform):
    return f"flashtex-cli-{version}-{platform}.tar.gz"


def find_asset(release, name):
    return next(
        (a for a in release["assets"] if a["name"] == name and a["state"] == "uploaded"),
        None,
    )


def find_release(repo, tag, wait_seconds):
    endpoint = f"repos/{repo}/releases/tags/{tag}" if tag else f"repos/{repo}/releases/latest"
    deadline = time.monotonic() + wait_seconds
    while True:
        release = gh_api(endpoint)
        version = release["tag_name"].removeprefix("v")
        required = [DMG_ASSET, SUMS_ASSET] + [cli_asset_name(version, p) for p in REQUIRED_CLI_PLATFORMS]
        missing = [name for name in required if find_asset(release, name) is None]
        if not missing:
            return release
        # A `release: published` event can arrive before every asset finishes uploading.
        if time.monotonic() >= deadline:
            sys.exit(f"error: {release['tag_name']} is missing {missing} after {wait_seconds}s")
        print(f"Waiting for {missing} on {release['tag_name']}...", file=sys.stderr)
        time.sleep(10)


def sha256_of(repo, release, asset):
    digest = asset.get("digest") or ""
    if digest.startswith("sha256:"):
        return digest.split(":", 1)[1]
    with tempfile.TemporaryDirectory() as tmp:
        subprocess.run(
            ["gh", "release", "download", release["tag_name"], "--repo", repo,
             "--pattern", asset["name"], "--dir", tmp],
            check=True,
        )
        sha = hashlib.sha256()
        with open(Path(tmp) / asset["name"], "rb") as f:
            for chunk in iter(lambda: f.read(1 << 20), b""):
                sha.update(chunk)
        return sha.hexdigest()


def download_text(repo, release, name):
    with tempfile.TemporaryDirectory() as tmp:
        subprocess.run(
            ["gh", "release", "download", release["tag_name"], "--repo", repo,
             "--pattern", name, "--dir", tmp],
            check=True,
        )
        return (Path(tmp) / name).read_text(encoding="utf-8")


def parse_sha256sums(text):
    """Parse `sha256sum`/`shasum -a 256` output: "<hex>  <filename>" per line."""
    sums = {}
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        digest, _, name = line.partition(" ")
        sums[name.strip().lstrip("*")] = digest.strip()
    return sums


def render_conditionals(text, flags):
    return CONDITIONAL_RE.sub(lambda m: m.group(2) if flags.get(m.group(1)) else "", text)


def main():
    parser = argparse.ArgumentParser(description="Render the FlashTeX site for a release.")
    parser.add_argument("out", help="directory to write the rendered site into")
    parser.add_argument("--tag", help="release tag to link (default: latest release)")
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", "flash-tex/flashtex"))
    parser.add_argument("--timezone", default=os.environ.get("SITE_TIMEZONE", "America/New_York"),
                        help="timezone used for the displayed release date")
    parser.add_argument("--wait", type=int, default=300,
                        help="seconds to wait for the release assets to finish uploading")
    args = parser.parse_args()

    release = find_release(args.repo, args.tag, args.wait)
    version = release["tag_name"].removeprefix("v")
    dmg = find_asset(release, DMG_ASSET)
    sums = parse_sha256sums(download_text(args.repo, release, SUMS_ASSET))
    published = datetime.fromisoformat(release["published_at"].replace("Z", "+00:00"))
    published = published.astimezone(ZoneInfo(args.timezone))
    values = {
        "TAG": release["tag_name"],
        "VERSION": version,
        "DATE": f"{published:%B} {published.day}, {published.year}",
        "SIZE": f"{dmg['size'] / (1024 * 1024):.1f} MB",
        "SHA256": sha256_of(args.repo, release, dmg),
    }
    flags = {}
    for platform, prefix in CLI_PLATFORMS.items():
        name = cli_asset_name(version, platform)
        asset = find_asset(release, name)
        has_asset = asset is not None
        flags[f"HAS_{prefix}"] = has_asset
        values[f"{prefix}_SIZE"] = f"{asset['size'] / (1024 * 1024):.1f} MB" if has_asset else ""
        values[f"{prefix}_SHA256"] = sums.get(name, "") if has_asset else ""
        if has_asset and name not in sums:
            sys.exit(f"error: {name} is not listed in {release['tag_name']}'s {SUMS_ASSET}")
    # Kept for the download page's Linux row, named after the tarball we actually gate on.
    flags["HAS_LINUX_CLI"] = flags["HAS_CLI_LINUX_X86_64"]
    flags["NO_LINUX_CLI"] = not flags["HAS_LINUX_CLI"]

    source = Path(__file__).resolve().parent
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    for path in sorted(source.rglob("*")):
        relative = path.relative_to(source)
        if path.is_dir() or relative.name in NOT_PUBLISHED or "__pycache__" in relative.parts:
            continue
        destination = out / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        if relative.as_posix() in TEMPLATED:
            text = path.read_text(encoding="utf-8")
            text = render_conditionals(text, flags)
            for key, value in values.items():
                text = text.replace("{{" + key + "}}", value)
            if "{{" in text:
                sys.exit(f"error: unfilled placeholder left in {relative}")
            destination.write_text(text, encoding="utf-8")
        else:
            shutil.copy2(path, destination)
    (out / ".nojekyll").touch()
    print(json.dumps({**values, **flags}))


if __name__ == "__main__":
    main()
