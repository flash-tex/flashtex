#set page(width: 12cm, height: 9cm, margin: 8mm, bleed: 3mm)
#set text(font: "Libertinus Serif", size: 10.7pt)

Upright text, then #text(size: 13.37pt)[a larger size] and #text(size: 6.1pt)[a tiny one],
with H#sub[2]O, x#super[n+1] and #text(tracking: 0.73pt)[tracked letters] and
#text(spacing: 180%)[wide word spacing here].

#rotate(17deg)[Rotated by seventeen degrees.]
#rotate(-90deg, reflow: true)[Sideways]
#scale(x: 137%, y: 81%)[Scaled unevenly]
#skew(ax: 13deg)[Skewed text]
#box(inset: 2.3pt, stroke: 0.4pt)[#move(dx: 0.37pt, dy: -1.13pt)[moved by fractions]]

$ sum_(k=1)^n k^2 = (n (n+1) (2n+1)) / 6 $
