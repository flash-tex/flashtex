// PDF islands (display-list-v3.3 E5): an SVG image, gradient and tiling
// fills and strokes, gradient text, a gradient page fill on page 2, inside
// rotated, scaled and clipped groups. `drawing.svg` is written by
// tests/oracle.rs.
#set page(width: 240pt, height: 300pt, margin: 12pt)
#set text(size: 10pt)

SVG: #box(image("drawing.svg", width: 50pt))
#box(rotate(15deg, image("drawing.svg", width: 30pt)))

#rect(width: 80pt, height: 20pt, fill: gradient.linear(red, blue))
#circle(radius: 12pt, fill: gradient.radial(yellow, green), stroke: 2pt + gradient.conic(red, blue))
#rect(width: 60pt, height: 16pt, fill: tiling(size: (6pt, 6pt), square(size: 3pt, fill: black)))

#text(fill: gradient.linear(red, blue), size: 16pt)[Gradient text]
#text(stroke: 0.5pt + gradient.linear(green, blue), size: 16pt)[Stroked]

#box(width: 40pt, height: 14pt, clip: true, scale(x: 150%, rect(width: 60pt, height: 20pt, fill: gradient.linear(black, white))))
Plain text after the islands.

#pagebreak()
#set page(fill: gradient.linear(white, luma(80%), angle: 90deg))
A page with a gradient fill.
