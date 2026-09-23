# TikZ subset (`flashtex_vector_graphics::tikz`)

Original reader for the drawing language of TikZ (task FT-062). No TeX or
PGF code runs: statements are interpreted directly into this crate's
display items. Geometry follows PGF's own conventions (TeX points on a y-up
canvas, the same defaults and bounding-box rules) and is converted once, at
the end, into picture space: PDF points, origin at the top-left of the
picture's bounding box, y down.

Node text is measured by a caller-supplied `TextMeasurer` (the render
pipeline shapes it with Latin Modern and the TFM metrics pdfTeX uses) and
returned as `PictureText` placements; the crate itself draws no glyphs.

Anything outside the subset is **reported** as a `Diagnostic` with the byte
range of the statement, never silently dropped.

## API

```rust
let mut tikz = Tikz::new(10.0);                 // body font size in pt
let diags = tikz.read_preamble(preamble);       // \tikzset, \tikzstyle, \definecolor, \colorlet
for pic in find_pictures(document) {            // \begin{tikzpicture}[..] .. \end{tikzpicture}
    let picture = tikz.render(document, &pic, &measurer);
    // picture.items (vector items), picture.texts, picture.width_bp/height_bp, picture.diagnostics
}
```

`PictureText::after_item` gives the paint order of text relative to the
top-level items (a node's text is painted after its own border and fill).

## Supported

**Statements**: `\draw`, `\fill`, `\filldraw`, `\path`, `\clip` (until the
end of the enclosing scope), `\node`, `\coordinate`, `\foreach` (nested,
`\x/\y` pairs, `{a,b,...,z}` numeric and single-letter ranges, `[count=\i]`,
brace or single-statement bodies), `\begin{scope}[..]..\end{scope}`,
`{[..] ..}` groups, `\tikzset`, `\tikzstyle{name}=[..]` / `+=[..]`,
`\definecolor{name}{rgb|RGB|gray|cmyk}{..}`, `\colorlet`, `\def\name{..}`,
`\newcommand{\name}{..}` (no parameters), `\pgfmathsetmacro`,
`\pgfmathtruncatemacro`, `\usetikzlibrary` (accepted, ignored).

**Coordinates**: `(x,y)` with or without units (plain numbers use the
`x`/`y` vectors, 1cm by default), polar `(a:r)` and `(a:rx and ry)`,
`+(..)` and `++(..)` relative, named nodes `(n)` (lines are clipped to the
node border), anchors `(n.north east)`, `(n.30)`, `([opts]p)`, and calc
forms `($(a)!t!(b)$)`, `($(a)+(b)$)`, `($k*(a)-(b)$)`. Expressions: `+ - * /
^`, parentheses, units `pt cm mm in bp pc sp dd cc em ex`, `sin cos tan asin
acos atan atan2 deg rad sqrt abs exp ln log10 int round floor ceil mod min
max veclen`, `pi`.

**Path operations**: `--`, `-|`, `|-`, `.. controls (c1) [and (c2)] ..`,
`to` (straight, `bend left/right[=θ]`, `out`/`in`, `looseness`),
`rectangle`, `circle (r)` / `circle[radius=..]`, `ellipse (a and b)` /
`[x radius, y radius]`, `arc (s:e:r)` / `(s:e:rx and ry)` /
`[start angle, end angle | delta angle, radius]`, `grid[step|xstep|ystep]`,
`cycle`, `node ..` and `coordinate ..` on paths (before or after the target).

**Options**: `draw[=c]`, `fill[=c]`, `color=`, bare xcolor expressions
(`red`, `blue!30`, `green!60!black`, the 19 base colours with xcolor's own
per-model values), `text=`, `line width`, `ultra thin` .. `ultra thick`,
`help lines`, `solid`, `dashed`, `dotted` and their `densely`/`loosely`
forms, `dash dot`, `dash pattern=on .. off ..`, `dash phase`, `line cap`,
`line join`, `miter limit`, `rounded corners[=r]`, `sharp corners`,
`even odd rule`, `nonzero rule`, `opacity`, `draw opacity`, `fill opacity`,
`pattern=` (the eight PGF line, grid and dot patterns), `pattern color=`,
`text opacity`, `scale`, `xscale`, `yscale`, `shift={(x,y)}`, `xshift`,
`yshift`, `rotate`, `rotate around={a:(p)}`, `x=`, `y=`, arrows `->`, `<-`,
`<->`, `-stealth`, `latex-latex`, `>=stealth|latex|to`, `arrows=`; nodes:
`draw`, `fill`, `circle`, `ellipse`, `rectangle`, `shape=`, `anchor=`,
`above/below/left/right[=d]` and the four diagonal forms, `inner sep`,
`inner xsep/ysep`, `outer sep`, `minimum size/width/height`, `midway`,
`near start/end`, `very near start/end`, `at start/end`, `pos=`, `sloped`,
`transform shape`, `font=` (`\tiny`..`\Huge`, `\bfseries`, `\itshape`),
`name=`; styles: `name/.style`, `/.append style`, `/.default` with `#1`,
`every picture`, `every scope`, `every path`, `every node`,
`every circle node`, `every rectangle node`; `use as bounding box`.

**Arrow tips** are PGF's compatibility tips `to` (the default `>`), `stealth`
and `latex` with its exact geometry and line shortening; `To`, `Stealth`
and `Latex` (arrows.meta names) are drawn with the same shapes.

## PGF details reproduced on purpose

These are the places where a naive implementation is visibly off against
pdflatex at 150 dpi (each was found by the oracle harness):

- The bounding box counts every path point including Bézier control
  points, the *unrounded* corner points of `rounded corners`, half the line
  width of stroked paths, and a node's shape without its `outer sep` (plus
  half the line width when the shape is drawn).
- Arcs are split into quarter turns from the start angle plus a remainder,
  so their control points (and therefore the bounding box) match.
- `grid` lines are computed in scaled points with TeX's truncating
  division and the final "0.01pt early" line, so a line at a coordinate
  that is not an exact multiple of the (truncated) step disappears exactly as
  in PGF (`0.5cm` is 932339sp, so `(0,2) grid (1,3)` has no line at y=2).
- The `to` path's control distance uses PGF's 1/255-precision length
  estimate (`\tikz@to@compute@distance@main`), up to 0.4 % short.
- Arrow tips shorten the path by the tip's right extend; on curves the end
  point and the last control point move together.

## Not supported (reported)

`\shade`, `\pic`, `\matrix`, `\graph`, `plot`, `let`, `edge`, `sin`/`cos`
path operations, decorations and shadings, `double`, `pre/postaction`,
positioning-library `above=of`, `label=`/`pin=`, multi-line node text
(`align`, `\\`), math in node text (set as italic text with a warning),
reversed arrow tips (drawn forward with a warning), `|` and other
arrows.meta tips, `\tikz ...;` inline commands, `baseline=` (the picture's
bottom is its baseline). Text in a clipped scope is not clipped.

## Evidence

`crates/render-pipeline/fixtures/tikz/*.tex` (30 fixtures) are compiled by
pdflatex (oracle only) and by `flashtex-tikz-pdf`, rasterised at 150 dpi and
compared; see `crates/render-pipeline/docs/evidence/tikz/README.md` for the
method, tolerance and per-fixture numbers. Unit tests: `src/tikz/tests.rs`.
