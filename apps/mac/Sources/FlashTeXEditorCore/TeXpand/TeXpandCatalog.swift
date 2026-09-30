import Foundation

extension TeXpand {
    /// The built-in catalog (PLAN §14): text and structure, as packs that
    /// can be switched off one by one (`disabled_packs`). Written in the
    /// same TOML a user or project file uses, so the built-ins exercise the
    /// loader and read as examples. Math, instant atoms, ligatures and
    /// postfix are in TeXpandMathCatalog.swift; the domain packs are M11.
    public enum Catalog {
        public static let packs: [Layer] = [
            Layer(name: "built-in:preamble", source: preamble),
            Layer(name: "built-in:sections", source: sections),
            Layer(name: "built-in:lists", source: lists),
            Layer(name: "built-in:floats", source: floats),
            Layer(name: "built-in:theorems", source: theorems),
            Layer(name: "built-in:tables", source: tables),
            Layer(name: "built-in:display-math", source: displayMath),
            Layer(name: "built-in:algorithms", source: algorithms),
            Layer(name: "built-in:listings", source: listings),
            Layer(name: "built-in:beamer", source: beamer),
            Layer(name: "built-in:references", source: references),
        ] + mathPacks

        static let preamble = #"""
        [pack]
        name = "preamble"
        summary = "Document scaffolds and \\usepackage lines"

        [[abbr]]
        name = "doc"
        scope = ["preamble", "text"]
        leaf = true
        description = "A whole document: doc:article, doc:report, doc:beamer, doc:hw"
        params = [{ name = "kind", type = "raw", default = "article" }]
        body = '''
        \documentclass{<<p.kind.value>>}
        \usepackage{amsmath}
        \usepackage{graphicx}

        \title{<<1>>}
        \author{<<2>>}

        \begin{document}
        \maketitle

        <<0>>

        \end{document}'''

        [[abbr.variant]]
        when = { param = { kind = "beamer" } }
        body = '''
        \documentclass{beamer}
        \usetheme{<<1:default>>}

        \title{<<2>>}
        \author{<<3>>}
        \date{\today}

        \begin{document}

        \begin{frame}
          \titlepage
        \end{frame}

        \begin{frame}{<<4>>}
          <<0>>
        \end{frame}

        \end{document}'''

        [[abbr.variant]]
        when = { param = { kind = "hw" } }
        body = '''
        \documentclass[11pt]{article}
        \usepackage[margin=1in]{geometry}
        \usepackage{amsmath,amssymb,amsthm}

        \newtheorem*{problem}{Problem}

        \title{<<1:Course>> --- Homework <<2:1>>}
        \author{<<3>>}
        \date{\today}

        \begin{document}
        \maketitle

        \begin{problem}
          <<4>>
        \end{problem}

        \begin{proof}
          <<0>>
        \end{proof}

        \end{document}'''

        [[abbr]]
        name = "pkg"
        scope = ["preamble"]
        leaf = true
        description = "\\usepackage{…}: pkg:amsmath,tikz"
        params = [{ name = "names", type = "list", default = "amsmath" }]
        body = '\usepackage{<<p.names.value>>}'
        """#

        static let sections = #"""
        [pack]
        name = "sections"
        summary = "Sectioning commands with titles and labels"

        [[abbr]]
        name = "ch"
        label_prefix = "ch"
        children_optional = true
        description = "\\chapter: ch{Title}#, ch! unnumbered"
        body = '''
        \chapter<<star>>{<<arg.1>>}<<label>>
        <<children>>'''

        [[abbr]]
        name = "sec"
        label_prefix = "sec"
        children_optional = true
        description = "\\section: sec{Title}#, sec! unnumbered"
        body = '''
        \section<<star>>{<<arg.1>>}<<label>>
        <<children>>'''

        [[abbr]]
        name = "sub"
        label_prefix = "sec"
        children_optional = true
        description = "\\subsection"
        body = '''
        \subsection<<star>>{<<arg.1>>}<<label>>
        <<children>>'''

        [[abbr]]
        name = "ssub"
        label_prefix = "sec"
        children_optional = true
        description = "\\subsubsection"
        body = '''
        \subsubsection<<star>>{<<arg.1>>}<<label>>
        <<children>>'''

        [[abbr]]
        name = "para"
        label_prefix = "sec"
        description = "\\paragraph"
        body = '\paragraph<<star>>{<<arg.1>>}<<label>>'
        """#

        static let lists = #"""
        [pack]
        name = "lists"
        summary = "itemize, enumerate, description"

        [[abbr]]
        name = "enum"
        shape = "children"
        default_child = "item"
        provides = ["list"]
        description = "enumerate: enum4 is enum>item*4"
        body = '''
        \begin{enumerate}<<opt>>
          <<children>>
        \end{enumerate}'''

        [[abbr]]
        name = "items"
        shape = "children"
        default_child = "item"
        provides = ["list"]
        description = "itemize: items3 is items>item*3"
        body = '''
        \begin{itemize}<<opt>>
          <<children>>
        \end{itemize}'''

        [[abbr]]
        name = "desc"
        shape = "children"
        default_child = "item"
        provides = ["list", "env:description"]
        description = "description: desc>item[Term]*3"
        body = '''
        \begin{description}<<opt>>
          <<children>>
        \end{description}'''

        [[abbr]]
        name = "item"
        scope = ["list"]
        children_optional = true
        description = "\\item, with an overlay in beamer: item*3<+->"
        body = '''
        \item<<overlay>><<opt>> <<arg.1>>
          <<children>>'''

        [[abbr]]
        name = "item"
        scope = ["env:description"]
        children_optional = true
        description = "\\item[term] in a description"
        body = '''
        \item<<overlay>>[<<opt.value>>] <<arg.1>>
          <<children>>'''
        """#

        static let floats = #"""
        [pack]
        name = "floats"
        summary = "Figures, graphics, captions, subfigures"

        [[abbr]]
        name = "fig"
        label_prefix = "fig"
        default_child = "img+cap"
        provides = ["float", "env:figure"]
        description = "figure (with an image and a caption unless given children): fig[ht]>img{a.pdf}+cap{…}#arch"
        body = '''
        \begin{figure}<<opt>>
          \centering
          <<children>>
          <<label>>
        \end{figure}'''

        [[abbr]]
        name = "img"
        requires = ["graphicx"]
        description = "\\includegraphics: img[width=.5\\linewidth]{file}"
        body = '\includegraphics[<<opt.value:width=0.8\linewidth>>]{<<arg.1>>}'

        [[abbr]]
        name = "cap"
        scope = ["float"]
        description = "\\caption"
        body = '\caption{<<arg.1>>}'

        [[abbr]]
        name = "subfig"
        label_prefix = "fig"
        shape = "children"
        default_child = "sub"
        child_separator = '\hfill'
        row_break = '\par\medskip'
        provides = ["float", "env:figure"]
        requires = ["subcaption"]
        description = "a figure of subfigures: subfig2, subfig2x2"
        body = '''
        \begin{figure}<<opt>>
          \centering
          <<children>>
          \caption{<<arg.1>>}
          <<label>>
        \end{figure}'''

        [[abbr]]
        name = "sub"
        scope = ["float"]
        label_prefix = "fig"
        requires = ["subcaption", "graphicx"]
        description = "subfigure: sub[0.3\\linewidth]{file}{caption}"
        body = '''
        \begin{subfigure}{<<opt.value:0.48\linewidth>>}
          \centering
          \includegraphics[width=\linewidth]{<<arg.1>>}
          \caption{<<arg.2>>}
          <<label>>
        \end{subfigure}'''
        """#

        static let theorems = #"""
        [pack]
        name = "theorems"
        summary = "Theorem-like environments and proofs"

        [[abbr]]
        name = "thm"
        label_prefix = "thm"
        args = [{ name = "title", optional = true, prefix = "[", suffix = "]" }]
        description = "theorem: thm{Name}#main+pf"
        body = '''
        \begin{theorem}<<arg.title>><<label>>
          <<children>>
        \end{theorem}'''

        [[abbr]]
        name = "lem"
        label_prefix = "lem"
        args = [{ name = "title", optional = true, prefix = "[", suffix = "]" }]
        description = "lemma"
        body = '''
        \begin{lemma}<<arg.title>><<label>>
          <<children>>
        \end{lemma}'''

        [[abbr]]
        name = "prop"
        label_prefix = "prop"
        args = [{ name = "title", optional = true, prefix = "[", suffix = "]" }]
        description = "proposition"
        body = '''
        \begin{proposition}<<arg.title>><<label>>
          <<children>>
        \end{proposition}'''

        [[abbr]]
        name = "cor"
        label_prefix = "cor"
        args = [{ name = "title", optional = true, prefix = "[", suffix = "]" }]
        description = "corollary"
        body = '''
        \begin{corollary}<<arg.title>><<label>>
          <<children>>
        \end{corollary}'''

        [[abbr]]
        name = "defn"
        label_prefix = "def"
        args = [{ name = "title", optional = true, prefix = "[", suffix = "]" }]
        description = "definition"
        body = '''
        \begin{definition}<<arg.title>><<label>>
          <<children>>
        \end{definition}'''

        [[abbr]]
        name = "rmk"
        label_prefix = "rmk"
        args = [{ name = "title", optional = true, prefix = "[", suffix = "]" }]
        description = "remark"
        body = '''
        \begin{remark}<<arg.title>><<label>>
          <<children>>
        \end{remark}'''

        [[abbr]]
        name = "ex"
        label_prefix = "ex"
        args = [{ name = "title", optional = true, prefix = "[", suffix = "]" }]
        description = "example"
        body = '''
        \begin{example}<<arg.title>><<label>>
          <<children>>
        \end{example}'''

        [[abbr]]
        name = "pf"
        requires = ["amsthm"]
        args = [{ name = "title", optional = true, prefix = "[", suffix = "]" }]
        description = "proof"
        body = '''
        \begin{proof}<<arg.title>>
          <<children>>
        \end{proof}'''
        """#

        static let tables = #"""
        [pack]
        name = "tables"
        summary = "tabular and booktabs tables; .float adds the table float"

        [[abbr]]
        name = "tab"
        label_prefix = "tab"
        generator = "table"
        wrap = "table"
        description = "tabular: tab:lcr:4, tab:lcr:4{A,B,C} with a header, .float for a table float"
        params = [{ name = "spec", type = "colspec", default = "ll" }, { name = "rows", type = "int", default = "3" }]

        [abbr.modifier.float]
        label_prefix = "tab"
        body = '''
        \begin{table}<<opt>>
          \centering
          \caption{<<1>>}
          <<label>>
          <<body>>
        \end{table}'''

        [[abbr]]
        name = "btab"
        label_prefix = "tab"
        generator = "table"
        wrap = "table"
        generator_opts = { booktabs = true }
        requires = ["booktabs"]
        description = "booktabs tabular: btab:lrr:5{Name,Score,Time}, .float for a table float"
        params = [{ name = "spec", type = "colspec", default = "ll" }, { name = "rows", type = "int", default = "3" }]

        [abbr.modifier.float]
        label_prefix = "tab"
        body = '''
        \begin{table}<<opt>>
          \centering
          \caption{<<1>>}
          <<label>>
          <<body>>
        \end{table}'''
        """#

        static let displayMath = #"""
        [pack]
        name = "display-math"
        summary = "equation, align, gather, cases"

        [[abbr]]
        name = "eq"
        label_prefix = "eq"
        provides = ["math"]
        description = "equation: eq#energy, eq! unnumbered"
        body = '''
        \begin{equation<<star>>}<<label>>
          <<children>>
        \end{equation<<star>>}'''

        [[abbr.variant]]
        when = { star = true }
        requires = ["amsmath"]

        [[abbr]]
        name = "align"
        label_prefix = "eq"
        wrap = "align"
        shape = "children"
        default_child = "row"
        child_separator = ' \\'
        provides = ["math", "env:align"]
        requires = ["amsmath"]
        description = "align: align3 has three rows, align!3 unnumbered"
        body = '''
        \begin{align<<star>>}<<label>>
          <<children>>
        \end{align<<star>>}'''

        [[abbr]]
        name = "gather"
        label_prefix = "eq"
        shape = "children"
        default_child = "row"
        child_separator = ' \\'
        provides = ["math", "env:gather"]
        requires = ["amsmath"]
        description = "gather: gather2"
        body = '''
        \begin{gather<<star>>}<<label>>
          <<children>>
        \end{gather<<star>>}'''

        [[abbr]]
        name = "cases"
        scope = ["math"]
        shape = "children"
        default_child = "row"
        child_separator = ' \\'
        provides = ["env:cases"]
        requires = ["amsmath"]
        description = "cases: cases3"
        body = '''
        \begin{cases}
          <<children>>
        \end{cases}'''

        [[abbr]]
        name = "row"
        scope = ["env:align"]
        description = "an align row"
        body = '<<1>> &= <<2>>'

        [[abbr]]
        name = "row"
        scope = ["env:gather"]
        description = "a gather row"
        body = '<<1>>'

        [[abbr]]
        name = "row"
        scope = ["env:cases"]
        description = "a cases row"
        body = '<<1>> & \text{if } <<2>>'
        """#

        static let algorithms = #"""
        [pack]
        name = "algorithms"
        summary = "algorithm + algpseudocode"

        [[abbr]]
        name = "alg"
        label_prefix = "alg"
        provides = ["alg"]
        requires = ["algorithm", "algpseudocode"]
        args = ["caption"]
        description = "algorithm float with algorithmic: alg{Name}#>fn{F}{x}>st*2+ret"
        body = '''
        \begin{algorithm}<<opt>>
          \caption{<<arg.caption>>}
          <<label>>
          \begin{algorithmic}[1]
            <<children>>
          \end{algorithmic}
        \end{algorithm}'''

        [[abbr]]
        name = "fn"
        scope = ["alg"]
        description = "\\Function{name}{args}"
        body = '''
        \Function{<<arg.1>>}{<<arg.2>>}
          <<children>>
        \EndFunction'''

        [[abbr]]
        name = "for"
        scope = ["alg"]
        description = "\\For{…}"
        body = '''
        \For{<<arg.1>>}
          <<children>>
        \EndFor'''

        [[abbr]]
        name = "while"
        scope = ["alg"]
        description = "\\While{…}"
        body = '''
        \While{<<arg.1>>}
          <<children>>
        \EndWhile'''

        [[abbr]]
        name = "if"
        scope = ["alg"]
        description = "\\If{…}"
        body = '''
        \If{<<arg.1>>}
          <<children>>
        \EndIf'''

        [[abbr]]
        name = "ifelse"
        scope = ["alg"]
        description = "\\If{…} … \\Else …; children go in the then-branch"
        body = '''
        \If{<<arg.1>>}
          <<children>>
        \Else
          <<1>>
        \EndIf'''

        [[abbr]]
        name = "st"
        scope = ["alg"]
        description = "\\State"
        body = '\State <<arg.1>>'

        [[abbr]]
        name = "ret"
        scope = ["alg"]
        description = "\\State \\Return"
        body = '\State \Return <<arg.1>>'
        """#

        static let listings = #"""
        [pack]
        name = "listings"
        summary = "lstlisting and minted"

        [[abbr]]
        name = "lst"
        leaf = true
        requires = ["listings"]
        params = [{ name = "lang", type = "raw", default = "Python" }]
        description = "lstlisting: lst:C"
        body = '''
        \begin{lstlisting}[language=<<p.lang.value>>]
        <<0>>
        \end{lstlisting}'''

        [[abbr]]
        name = "minted"
        leaf = true
        requires = ["minted"]
        params = [{ name = "lang", type = "raw", default = "python" }]
        description = "minted: minted:py"
        body = '''
        \begin{minted}{<<p.lang.value>>}
        <<0>>
        \end{minted}'''
        """#

        static let beamer = #"""
        [pack]
        name = "beamer"
        summary = "Frames, columns, blocks and overlays"

        [[abbr]]
        name = "frame"
        provides = ["beamer"]
        args = ["title"]
        description = "frame: frame{Title}>items3, frame<2->"
        body = '''
        \begin{frame}<<overlay>><<opt>>{<<arg.title>>}
          <<children>>
        \end{frame}'''

        [[abbr]]
        name = "cols"
        scope = ["beamer"]
        generator = "columns"
        provides = ["env:columns"]
        params = [{ name = "widths", type = "list", default = "5,5" }]
        description = "columns: cols:6,4 (tenths of \\textwidth); children go in the first column"

        [[abbr]]
        name = "col"
        scope = ["beamer"]
        params = [{ name = "width", type = "raw", default = "0.5" }]
        description = "one column: col:0.4"
        body = '''
        \begin{column}{<<p.width.value>>\textwidth}
          <<children>>
        \end{column}'''

        [[abbr]]
        name = "block"
        scope = ["beamer"]
        args = ["title"]
        description = "block{Title}"
        body = '''
        \begin{block}<<overlay>>{<<arg.title>>}
          <<children>>
        \end{block}'''

        [[abbr]]
        name = "alert"
        scope = ["beamer"]
        description = "\\alert<…>{…}"
        body = '\alert<<overlay>>{<<arg.1>>}'

        [[abbr]]
        name = "note"
        scope = ["beamer"]
        description = "\\note{…}"
        body = '\note{<<arg.1>>}'
        """#

        static let references = #"""
        [pack]
        name = "references"
        summary = "Cross-references and citations"

        [[abbr]]
        name = "ref"
        scope = ["text", "math"]
        leaf = true
        params = [{ name = "target", type = "raw" }]
        description = "a reference with profile.ref_cmd: ref:fig:arch"
        body = '<<profile.ref_cmd>>{<<p.target.value>>}'

        [[abbr.variant]]
        when = { profile = { ref_cmd = '\cref' } }
        requires = ["cleveref"]

        [[abbr.variant]]
        when = { profile = { ref_cmd = '\Cref' } }
        requires = ["cleveref"]

        [[abbr.variant]]
        when = { profile = { ref_cmd = '\autoref' } }
        requires = ["hyperref"]

        [[abbr]]
        name = "cite"
        leaf = true
        params = [{ name = "keys", type = "raw" }]
        description = "a citation with profile.cite_cmd: cite:knuth"
        body = '<<profile.cite_cmd>>{<<p.keys.value>>}'
        """#
    }
}
