//! `\renewcommand` of commands pdflatex already has is not an error.
//!
//! pdflatex (TeX Live 2026) on [`PROBE`] reports exactly one error,
//! `LaTeX Error: Command \thetheorem undefined.` (the theorem is declared
//! after it). The pinned compiler reported fourteen: one for every line
//! below, since its expander only knows the commands it has seen defined.
//! Now the math ones are silent (their new meaning is what gets set), the
//! fixed text names are warnings that say the default is still printed, and
//! `\thetheorem` keeps its error.
//!
//! pdflatex is an oracle only and never runs in the product path.

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Severity;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const PROBE: &str = r"\documentclass{article}
\usepackage{amsmath,amssymb,amsthm}
\renewcommand{\proofname}{Solution}
\renewcommand{\abstractname}{Summary}
\renewcommand{\refname}{Bibliography}
\renewcommand{\labelenumi}{(\alph{enumi})}
\renewcommand{\labelitemi}{$\circ$}
\renewcommand{\figurename}{Fig.}
\renewcommand{\tablename}{Tab.}
\renewcommand{\epsilon}{\varepsilon}
\renewcommand{\phi}{\varphi}
\renewcommand{\vec}[1]{\mathbf{#1}}
\renewcommand{\Re}{\operatorname{Re}}
\renewcommand{\emptyset}{\varnothing}
\renewcommand{\qed}{\hfill$\blacksquare$}
\renewcommand{\thetheorem}{\arabic{theorem}}
\newtheorem{theorem}{Theorem}
\begin{document}
\begin{proof} Text. \end{proof}
\begin{enumerate}\item a\end{enumerate}
\begin{itemize}\item b\end{itemize}
\begin{equation} \epsilon \phi \vec{v} \Re z \emptyset \end{equation}
\end{document}
";

#[test]
fn renewing_a_predefined_command_is_not_an_undefined_command_error() {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text: PROBE }];
    let r = render(&docs, "main.tex", 1, "renew-predefined", &fonts, &RenderOptions::default());
    let undefined: Vec<&str> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("undefined"))
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(undefined, [r"LaTeX Error: Command \thetheorem undefined."], "pdflatex reports only \\thetheorem");
    let mut not_applied: Vec<&str> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.code == "renewcommand_not_applied")
        .map(|d| {
            assert_eq!(d.severity, Severity::Warning);
            let m = d.message.as_str();
            &m[m.find('\\').unwrap_or(0)..m.find('}').unwrap_or(m.len())]
        })
        .collect();
    not_applied.sort_unstable();
    assert_eq!(
        not_applied,
        [r"\renewcommand{\figurename", r"\renewcommand{\labelenumi", r"\renewcommand{\labelitemi", r"\renewcommand{\proofname", r"\renewcommand{\qed", r"\renewcommand{\refname", r"\renewcommand{\tablename"]
    );
}
