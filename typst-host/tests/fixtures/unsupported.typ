// Constructs v3 cannot express yet: each page must arrive INCOMPLETE with
// an UNSUPPORTED entry (spec §4.7), never approximated.
#set page(width: 8cm, height: 6cm, margin: 5mm)
#set text(font: "Libertinus Serif")

#rect(width: 2cm, height: 1cm, fill: gradient.linear(red, blue))
#rect(width: 2cm, height: 1cm, fill: rgb(255, 0, 0, 50%))
#text(stroke: 0.3pt + red)[Stroked]
