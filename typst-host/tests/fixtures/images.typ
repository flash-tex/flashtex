// Raster images (display-list-v3.3 E6): PNG with alpha (a soft mask),
// grey, 16 bits per component (written as 8), a JPEG passed through, smooth and pixelated
// scaling, transformed and clipped, one image drawn twice. The files are
// written by tests/oracle.rs.
#set page(width: 260pt, height: 320pt, margin: 12pt)
#set text(size: 9pt)

Alpha: #box(image("rgba.png", width: 40pt))
grey: #box(image("grey.png", width: 30pt, scaling: "pixelated"))
16-bit: #box(image("deep.png", width: 24pt, scaling: "smooth"))

JPEG: #box(image("photo.jpg", width: 60pt))
again: #box(image("rgba.png", width: 20pt))

#rotate(25deg, image("photo.jpg", width: 50pt))
#scale(x: -100%, image("rgba.png", width: 30pt))
#box(width: 30pt, height: 12pt, clip: true, image("grey.png", width: 50pt))
#image("deep.png", height: 30pt, width: 60pt, fit: "stretch")
#box(width: 40pt, height: 20pt, image("photo.jpg", width: 40pt, height: 20pt, fit: "cover"))
