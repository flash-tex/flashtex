#!/usr/bin/env python3
"""Regenerate every raster/derived brand asset from the SVGs in this folder.

    python3 assets/brand/generate.py

Needs `rsvg-convert` (Homebrew librsvg) and, for the macOS .icns, `iconutil`
(ships with macOS). The SVGs here are the single source; everything below is
derived and committed so builds and CI need neither tool:

  assets/brand/png/flashtex-social.png        1280x640 social card (logo on Paper)
  site/brand/*.svg                            logo, mark and icon copies the site serves
  site/favicon.svg, site/favicon-{16,32}.png  favicon (brand favicon art)
  site/apple-touch-icon.png                   180x180, full-bleed (iOS rounds it)
  site/og.png                                 same image as the social card
  apps/mac/Resources/AppIcon.icns             macOS icon on Apple's 1024 grid
  apps/ios/FlashTeXPad/Assets.xcassets        iPadOS AppIcon (1024 any + dark)

The art is never edited: the macOS icon only places the rounded square on
Apple's grid (824 px body, 100 px margin at 1024, soft drop shadow in the
margin), and the iOS/touch icons only square off the corners because those
platforms apply their own mask.
"""
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
RSVG = shutil.which("rsvg-convert") or "/opt/homebrew/bin/rsvg-convert"
PAPER = "#FAFAF7"


def svg(name):
    return (HERE / name).read_text()


def inner(text):
    """The SVG's children, without the outer <svg> element."""
    return re.sub(r"^<svg[^>]*>|</svg>\s*$", "", text.strip())


def viewbox(text):
    return [float(v) for v in re.search(r'viewBox="([^"]+)"', text).group(1).split()]


def render(svg_text, out, w, h=None):
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile("w", suffix=".svg", delete=False) as f:
        f.write(svg_text)
    subprocess.run([RSVG, "-w", str(w), "-h", str(h or w), "-o", str(out), f.name], check=True)
    Path(f.name).unlink()


def square(text):
    """Full-bleed variant for platforms that mask the icon themselves."""
    return text.replace('rx="22" ', "")


def mac_grid(text):
    """Place the 100-unit rounded square on Apple's macOS grid: at 1024 px the
    body is 824 px with a 100 px margin, i.e. 12.136 units of margin here."""
    m = 100 * 100 / 824
    span = 100 + 2 * m
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{-m:.3f} {-m:.3f} {span:.3f} {span:.3f}">'
        '<defs><filter id="s" x="-20%" y="-20%" width="140%" height="140%">'
        '<feGaussianBlur in="SourceAlpha" stdDeviation="1.1"/><feOffset dy="1.0"/>'
        '<feComponentTransfer><feFuncA type="linear" slope="0.30"/></feComponentTransfer>'
        '<feMerge><feMergeNode/><feMergeNode in="SourceGraphic"/></feMerge></filter></defs>'
        f'<g filter="url(#s)">{inner(text)}</g></svg>'
    )


def social_card():
    logo = svg("flashtex-logo.svg")
    x, y, w, h = viewbox(logo)
    cw, ch, lw = 1280, 640, 820
    lh = lw * h / w
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {cw} {ch}">'
        f'<rect width="{cw}" height="{ch}" fill="{PAPER}"/>'
        f'<svg x="{(cw - lw) / 2:.2f}" y="{(ch - lh) / 2:.2f}" width="{lw}" height="{lh:.2f}" viewBox="{x:g} {y:g} {w:g} {h:g}">'
        f"{inner(logo)}</svg></svg>"
    )


def main():
    icon, icon_dark, favicon = svg("flashtex-icon.svg"), svg("flashtex-icon-dark.svg"), svg("favicon.svg")

    # Social card / og:image.
    card = HERE / "png" / "flashtex-social.png"
    render(social_card(), card, 1280, 640)

    # Website.
    site = REPO / "site"
    (site / "brand").mkdir(exist_ok=True)
    for name in ("flashtex-logo.svg", "flashtex-logo-dark.svg", "flashtex-mark.svg", "flashtex-icon.svg"):
        shutil.copy(HERE / name, site / "brand" / name)
    shutil.copy(HERE / "favicon.svg", site / "favicon.svg")
    render(favicon, site / "favicon-32.png", 32)
    render(favicon, site / "favicon-16.png", 16)
    render(square(icon), site / "apple-touch-icon.png", 180)
    shutil.copy(card, site / "og.png")

    # macOS .icns. 16 and 32 px pixels use the favicon art (bolt scaled up for
    # small sizes, per BRAND.md); everything larger uses the app icon.
    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "AppIcon.iconset"
        iconset.mkdir()
        for pt in (16, 32, 128, 256, 512):
            for scale in (1, 2):
                px = pt * scale
                art = favicon if px <= 32 else icon
                suffix = "" if scale == 1 else "@2x"
                render(mac_grid(art), iconset / f"icon_{pt}x{pt}{suffix}.png", px)
        out = REPO / "apps/mac/Resources/AppIcon.icns"
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(out)], check=True)

    # iPadOS asset catalog: single-size 1024, light (any) + dark appearances.
    xc = REPO / "apps/ios/FlashTeXPad/Assets.xcassets"
    appicon = xc / "AppIcon.appiconset"
    appicon.mkdir(parents=True, exist_ok=True)
    info = {"author": "xcode", "version": 1}
    (xc / "Contents.json").write_text(json.dumps({"info": info}, indent=2) + "\n")
    render(square(icon), appicon / "AppIcon-1024.png", 1024)
    render(square(icon_dark), appicon / "AppIcon-1024-dark.png", 1024)
    images = [
        {"filename": "AppIcon-1024.png", "idiom": "universal", "platform": "ios", "size": "1024x1024"},
        {"appearances": [{"appearance": "luminosity", "value": "dark"}],
         "filename": "AppIcon-1024-dark.png", "idiom": "universal", "platform": "ios", "size": "1024x1024"},
    ]
    (appicon / "Contents.json").write_text(json.dumps({"images": images, "info": info}, indent=2) + "\n")
    print("brand assets regenerated")


if __name__ == "__main__":
    sys.exit(main())
