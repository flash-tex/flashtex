// Plain prose: headings, paragraphs (ligatures, kerning), emphasis, a list,
// page numbers, two pages.
#set page(paper: "a5", numbering: "1")
#set text(font: "Libertinus Serif", size: 11pt, lang: "en")
#set par(justify: true)

= Typst support in FlashTeX

The office affords fluffy waffles: ligatures such as ffi, fl and ff, and kerning
pairs like AV, To and Wa exercise the shaper. _Emphasis_, *strong text* and
`raw code` switch fonts.

- first item
- second item with a longer line that wraps onto the next line of the list
- third

#lorem(80)

#pagebreak()

== Second page

#lorem(60)
