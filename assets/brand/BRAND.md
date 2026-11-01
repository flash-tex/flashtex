# FlashTeX brand

## Files
| File | Use |
|---|---|
| `flashtex-logo.svg` | Full logo (mark + wordmark) on light backgrounds |
| `flashtex-logo-dark.svg` | Full logo on dark backgrounds |
| `flashtex-mark.svg` | Bolt mark alone, transparent background |
| `flashtex-icon.svg` | App icon: white bolt on orange, rounded square |
| `flashtex-icon-dark.svg` | App icon: orange bolt on near-black |
| `favicon.svg` | Favicon: bolt scaled up slightly for small sizes |

All files are self-contained: "Flash" is converted to outlines, so no font is needed to display them.

## Colors
| Token | Hex | Use |
|---|---|---|
| Bolt orange | `#FF5A1F` | The mark; icon background |
| Ink | `#121212` | Wordmark on light; dark icon background |
| Paper | `#FAFAF7` | Light background; wordmark on dark |

Orange is for the mark only. Never set the wordmark in orange.

## Typography
- Wordmark: "Flash" in **Latin Modern Roman** (regular), followed by the classic TeX logo: T, then E lowered 0.215em with kerns of −0.1667em and −0.125em, then X.
- "TeX" is the standard TeX logo outline, thickened so its vertical strokes match Latin Modern's (0.089em).
- If live text needs the wordmark (e.g. HTML), set "Flash" in Latin Modern Roman and build "TeX" the TeX way. Don't substitute another serif.

## Mark construction (100-unit grid)
- Five bars, each 10 units tall, with ends slanted 5 units per 10 (about 27°), stacked 20.5 units apart.
- The bars step down and to the left to form a bolt, and the middle bar sticks out on both sides for the zig-zag.
- In the lockup, the mark runs from the top of the capitals to the bottom of the dropped E, with a gap of 0.26em before "Flash".

## Don'ts
- Don't stretch, rotate or recolor the bars individually.
- Don't swap "TeX" for plain text in a different font.
- Don't place the light logo on dark backgrounds; use `-dark`.
