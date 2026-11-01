// Shapes, strokes, colours and transforms v3 expresses exactly: paths with
// fills and strokes (dashes, caps, joins), luma/rgb/cmyk colour, rotation,
// scaling and a clipped box.
#set page(width: 10cm, height: 8cm, margin: 5mm, fill: luma(250))
#set text(font: "Libertinus Serif", size: 10pt)

#rect(width: 3cm, height: 1cm, fill: rgb("#1f77b4"), stroke: 1pt + black)
#line(length: 4cm, stroke: (paint: cmyk(0%, 80%, 80%, 10%), thickness: 2pt, dash: "dashed", cap: "round"))
#circle(radius: 5mm, fill: luma(40%), stroke: (thickness: 0.5pt, join: "bevel"))
#polygon(fill: rgb(20, 160, 60), (0pt, 0pt), (1cm, 5mm), (5mm, 1cm))
#rotate(15deg)[Rotated text]
#scale(x: 150%, y: 80%)[Scaled]
#box(width: 2cm, height: 6mm, clip: true)[#text(size: 20pt)[Clipped content]]
#table(columns: 2, [a], [b], [c], [d])
