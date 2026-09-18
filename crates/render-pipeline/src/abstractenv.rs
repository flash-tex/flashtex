//! The `abstract` environment (article.cls 366-388, report.cls 441-463,
//! v1.4n / TeX Live 2025).
//!
//! ```tex
//! \newenvironment{abstract}{%
//!     \if@twocolumn
//!       \section*{\abstractname}%
//!     \else
//!       \small
//!       \begin{center}%
//!         {\bfseries \abstractname\vspace{-.5em}\vspace{\z@}}%
//!       \end{center}%
//!       \quotation
//!     \fi}
//!     {\if@twocolumn\else\endquotation\fi}
//! ```
//!
//! with `\quotation` (article.cls 356-364)
//!
//! ```tex
//! \newenvironment{quotation}
//!     {\list{}{\listparindent 1.5em
//!              \itemindent    \listparindent
//!              \rightmargin   \leftmargin
//!              \parsep        \z@ \@plus\p@}%
//!      \item\relax}
//!     {\endlist}
//! ```
//!
//! The compiler has no model for the environment (it reports it and sets
//! the body as plain text), so the pipeline reads it from the source bytes,
//! the way it already reads `tikzpicture`, a list's `\begin` keys and a
//! paragraph's `\newpage`. What the class file says, in order, checked
//! against pdfTeX's own vertical list for `fixtures/divergence-probes/
//! min-abstract` (`\showoutput`, TeX Live 2025, `article`, 10pt):
//!
//! ```text
//! \penalty -51                      \@beginparpenalty of center's \@item
//! \glue 10.0 plus 4.0 minus 5.0     \@topsep: \normalsize \topsep 8pt+2-4,
//!                                   \partopsep 2pt+1-1, \parskip 0pt+1.
//!                                   \small only \def s \@listi, it never
//!                                   executes it, so \topsep is still
//!                                   \normalsize's here.
//! \glue(\parskip) 0.0 plus 1.0      the head paragraph starts
//! \glue(\baselineskip) 4.78902      11pt: \small's own \baselineskip
//! \hbox(6.21098+0.0)x469.75502      \small\bfseries \abstractname, centred
//!                                   at the FULL measure (center is a
//!                                   \trivlist: \leftmargin is 0)
//! \glue -5.3237                     \vspace{-.5em}: half a \bfseries quad
//!                                   at \small (cmbx9), through \vadjust
//! \glue 0.0  \glue 0.0  \glue 0.0   \vspace{\z@} and \@vspace's \z@skip s
//! \penalty -51                      \@endparpenalty of \end{center}
//! \glue 10.0 plus 3.0 minus 5.0     \@endparenv's \@topsepadd (no \parskip)
//! \glue -10.0 ... / \penalty -51 / \glue 10.0 ...
//!                                   \addpenalty of quotation's \@item,
//!                                   which lifts the skip, adds the penalty
//!                                   and puts the skip back
//!                                   (\addvspace\@topsep then adds nothing:
//!                                   quotation's 6pt loses to the 10pt
//!                                   already there)
//! \glue(\parskip) 0.0 plus 1.0      \list set \parskip\parsep
//! \glue(\baselineskip) 4.25165      still 11pt
//! \hbox(...)x419.75496, shifted 25.00003        \leftmargini both sides
//! .\hbox(0.0+0.0)x13.87161                      \itemindent = 1.5em of
//!                                               \small (cmr9)
//! ...
//! \penalty -51
//! \glue 6.0 plus 3.0 minus 3.0      \endlist's \@topsepadd, now from the
//!                                   \@listi \small redefined: \topsep 4pt
//!                                   plus \partopsep 2pt, NOT 8pt+2pt
//! ```
//!
//! The `\if@twocolumn` branch (article.cls 378-379) is set too, as what it
//! is: `\section*{\abstractname}`, an unnumbered `\Large\bfseries` head at
//! the *column* measure with the body as ordinary `\normalsize` paragraphs.
//! Its reference is `fixtures/real-world/conf-paper`, where pdfLaTeX's head
//! is `SFBX1440` at the column's left edge and the body `SFRM1000`.
//!
//! The `titlepage` branch (report's and book's default, or article's
//! `titlepage` option) is a page of its own (report.cls 441-450):
//!
//! ```tex
//! \newenvironment{abstract}{%
//!     \titlepage
//!     \null\vfil
//!     \@beginparpenalty\@lowpenalty
//!     \begin{center}%
//!       \bfseries \abstractname
//!       \@endparpenalty\@M
//!     \end{center}}%
//!    {\par\vfil\null\endtitlepage}
//! ```
//!
//! It is a *different environment*, not a variant of the one above: no
//! `\small`, no `\vspace{-.5em}` under the head, no `quotation` — the body
//! is ordinary `\normalsize` paragraphs at the full measure, and the first
//! of them is unindented (`\@doendpe` after `\end{center}`). What places it
//! is the `titlepage` environment around it (report.cls 485-513):
//! `\newpage`, `\thispagestyle{empty}`, `\setcounter{page}\@ne` on the way
//! in and `\newpage` (plus, one-sided, `\setcounter{page}\@ne` again) on
//! the way out. So the abstract is alone on its page and the material after
//! `\end{abstract}` starts the next one — the page break is half of what
//! this branch is.
//!
//! Between them the two `\vfil`s centre it vertically, and `\newpage`'s own
//! `\vfil` is a third claimant on the same room, exactly as in the
//! `titlepage` `\maketitle` ([`crate::typeset::TitleForm::Page`]). The page
//! builder has no stretchable vertical glue, so — as that `\maketitle` form
//! already does — the fil is resolved into a rigid skip from the page's
//! natural height: `typeset::build` lays the abstract's own blocks out, and
//! `(\textheight - natural) / 3` goes above the head and below the body.
//! The `\null`s matter and are set: the first fixes the head's distance
//! from the text top at `\topskip` plus the skips rather than at
//! `max(\topskip, height)`, and the last carries the interline glue that
//! ends the page.
//!
//! `book.cls` defines no `abstract` environment at all, so its
//! `\begin{abstract}` keeps the compiler's "not implemented" warning.
//!
//! One thing this environment made visible that is not specific to it: the
//! opening `\addvspace{\@topsep}` of every `\list`/`\trivlist` obeys
//! `\@xaddvskip`, and right after a heading `\@nbitem` replaces it
//! altogether. `\@maketitle`'s trailing `\vskip 1.5em` is larger than
//! `\topsep + \partopsep` at every class size, so an `abstract` opened
//! right after `\maketitle` contributes no skip of its own. See
//! `tests/abstract_env.rs`.

use flashtex_compiler::Span;

use crate::adapter::{Block, CharSrc, EnvOpen, Item, ParaPart, ParaStyle, Segment, SizedPara, TextStyle, Word};
use crate::style::Stylesheet;

/// `\listparindent` of `quotation`, in `em` of the size in force
/// (article.cls 358).
const LISTPARINDENT_EM: f64 = 1.5;

/// `\vspace{-.5em}` after `\abstractname` (article.cls 383).
const HEAD_VSPACE_EM: f64 = -0.5;

/// `\abstractname` (article.cls 447, report.cls 722).
const ABSTRACTNAME: &str = "Abstract";

/// One `\begin{abstract}...\end{abstract}` in one document's source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    /// Byte range of the `\begin{abstract}` command itself.
    pub begin: (usize, usize),
    /// Byte range of the body between the two commands.
    pub body: (usize, usize),
    /// Byte range of the `\end{abstract}` command itself.
    pub end: (usize, usize),
}

/// Every `abstract` environment of `text`, in order. Nested or unclosed
/// environments are skipped: the environment takes no argument, so a
/// `\begin` whose `\end` is missing has no body to set.
pub fn ranges(text: &str) -> Vec<Range> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = crate::adapter::find_command(&text[from..], "begin") {
        let begin = from + at;
        let after = begin + "\\begin".len();
        from = begin + 1;
        let Some(open) = argument_at(text, after, "abstract") else { continue };
        let mut scan = open;
        let mut close = None;
        while let Some(rel) = crate::adapter::find_command(&text[scan..], "end") {
            let end = scan + rel;
            scan = end + 1;
            if let Some(after_end) = argument_at(text, end + "\\end".len(), "abstract") {
                close = Some((end, after_end));
                break;
            }
        }
        let Some((end_start, end_end)) = close else { continue };
        out.push(Range {
            begin: (begin, open),
            body: (open, end_start),
            end: (end_start, end_end),
        });
        from = end_end;
    }
    out
}

/// The end byte of `{name}` when that is exactly what follows byte `at`
/// (spaces allowed before the brace, as `\begin`'s argument scan does).
fn argument_at(text: &str, at: usize, name: &str) -> Option<usize> {
    let rest = text.get(at..)?;
    let trimmed = rest.trim_start();
    let skipped = rest.len() - trimmed.len();
    let inner = trimmed.strip_prefix('{')?;
    let close = inner.find('}')?;
    (inner[..close].trim() == name).then_some(at + skipped + 1 + close + 1)
}

/// Which of the environment's three forms this document takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Branch {
    /// article.cls 380-386: `\small`, a centred head, a `quotation`.
    OneColumn,
    /// article.cls 378-379: `\section*{\abstractname}` — an unnumbered
    /// `\Large\bfseries` head at `\normalsize` and the *column* measure,
    /// and the body as ordinary paragraphs. A different shape, not a
    /// narrower one: no `\small`, no `quotation`, no centring.
    TwoColumn,
    /// article.cls 367-375 / report.cls 441-450 (report's and book's
    /// default, or article's `titlepage` option): a page of its own
    /// between `\null\vfil`s, inside `titlepage`. A `\normalsize` centred
    /// `\bfseries` head, ordinary full-measure paragraphs under it, and a
    /// page break on each side.
    TitlePage,
}

/// Which branch the `\begin{abstract}` at `at` in document `document`
/// takes.
///
/// `\if@twocolumn` is a *flag the document sets*, not a class option:
/// `\twocolumn` and `\onecolumn` change it wherever they stand
/// ([`crate::columns`]). Reading `options.twocolumn` here meant a document
/// that asked for two columns with the command took the one-column branch
/// — a centred `\small` head over a `quotation` where pdflatex sets a
/// `\section*` over ordinary paragraphs (GH#743). So the question is asked
/// at the environment's own position, and the class option is only where
/// the answer starts.
fn branch(style: &Stylesheet, document: usize, at: usize) -> Branch {
    let Some(g) = style.class_geometry.as_ref() else { return Branch::OneColumn };
    if g.options.titlepage {
        // `\if@titlepage` is tested first (article.cls 366).
        return Branch::TitlePage;
    }
    if style.columns.at(document, at) {
        return Branch::TwoColumn;
    }
    Branch::OneColumn
}

/// Whether the `\end{abstract}` at `at` in document `document` is an
/// `\endtrivlist`, so the `\begin` beside it is read in vertical mode and
/// takes `\partopsep` ([`crate::adapter`]'s `gap_has_trivlist_end`).
///
/// Only [`Branch::OneColumn`] is. article.cls 386 closes the environment
/// with `\if@twocolumn\else\endquotation\fi`: in two columns the body is
/// ordinary paragraphs under a `\section*` and the `\end` expands to
/// *nothing at all*, so there is no `\@endparenv` and no `\par`. #728 put
/// `abstract` in `TRIVLIST_ENVS` unconditionally, having swept only the
/// one-column branch, which made every two-column
/// `\end{abstract}\begin{center|quote|verbatim|itemize|enumerate|
/// description}` 1.992 bp long at 10 pt, 2.989 at 11 pt and 2.988 at 12 pt
/// — one `\partopsep` that pdflatex does not put there. (`\begin{thm}`,
/// which takes no `\partopsep` at all, was right either way.) The
/// `titlepage` branch is not `\endtrivlist` either — its `\end` is
/// `\par\vfil\null\endtitlepage`, so what follows starts a page rather
/// than taking a boundary skip — and `book`, which has no `abstract` at
/// all, keeps the compiler's warning.
pub(crate) fn end_is_endtrivlist(style: &Stylesheet, document: usize, at: usize) -> bool {
    !style.class_geometry.as_ref().is_some_and(|g| g.options.kind == flashtex_class_geometry::ClassKind::Book)
        && branch(style, document, at) == Branch::OneColumn
}

/// Rewrites the plain paragraphs the compiler produced for each `abstract`
/// body into the class's own shape and inserts its head before them.
/// Returns the source spans whose compiler diagnostic the pipeline now
/// supersedes — one per environment actually set, so a `titlepage`
/// document's warning (and `book`'s) survives untouched.
pub fn apply(texts: &[&str], blocks: &mut Vec<Block>, style: &Stylesheet) -> Vec<Span> {
    let mut superseded = Vec::new();
    // `book.cls` has no `abstract`; the compiler's warning is the truth.
    if style.class_geometry.as_ref().is_some_and(|g| g.options.kind == flashtex_class_geometry::ClassKind::Book) {
        return superseded;
    }
    let small = style.small();
    // Innermost-last so the inserted head never moves a range not yet done.
    let mut found: Vec<(usize, Range)> = Vec::new();
    for (d, text) in texts.iter().enumerate() {
        found.extend(ranges(text).into_iter().map(|r| (d, r)));
    }
    for (document, range) in found.into_iter().rev() {
        let span = Span::in_document(flashtex_compiler::DocumentId(document), range.begin.0, range.begin.1);
        // Per environment: `\twocolumn` may stand between two of them.
        let form = branch(style, document, range.begin.0);
        if form == Branch::TitlePage && !page_form_is_set(style) {
            continue;
        }
        let inside = |b: &Block| -> bool {
            block_span(b).is_some_and(|s| s.document.0 == document && s.start >= range.body.0 && s.start < range.body.1)
        };
        let Some(first) = blocks.iter().position(inside) else {
            // An empty `abstract`: nothing was set, so nothing to fix.
            continue;
        };
        let last = blocks.iter().rposition(inside).unwrap_or(first);
        if form == Branch::TitlePage {
            // The body is ordinary `\normalsize` paragraphs at the full
            // measure: whatever the compiler (and the environment/list
            // detection that already ran over the source before this) set
            // for each of them is right as it stands, including a nested
            // `\noindent` and a nested list's own `env_open`/`env_close`/
            // `list`. Only the outer `abstract` wrapper's own trapping
            // comes off, and only on the paragraph right after
            // `\begin{abstract}`: `\end{center}` leaves `\@endpetrue`, so
            // `\@doendpe` takes the `\parindent` box off it, and whatever
            // the compiler thought `\begin{abstract}` itself opened is
            // superseded by the head block's own `env_open`, inserted
            // below.
            if let Some(Block::Paragraph { indent, env_open, .. }) = blocks.get_mut(first) {
                *indent = false;
                *env_open = None;
            }
            // `\titlepage`'s `\newpage` discards whatever skip stood
            // between the previous block and `\begin{abstract}`: the glue
            // stays on the page that ends, and the page this opens starts
            // at `\topskip`. (`typeset::build` supplies the `\null`s, the
            // page break on each side and the `\vfil` centring; they are
            // page-level, not block-level.) A `\vspace` written *inside*
            // the environment, after `\begin{abstract}` and before the
            // body's own first character, is not discarded: it is real
            // glue between the head and the body, and stays on the body.
            drop_lead(&mut blocks[first], texts, range, style);
            blocks.insert(first, page_head_block(texts, document, range));
            superseded.push(span);
            continue;
        }
        if form == Branch::TwoColumn {
            // `\section*`'s before-skip is negative, so `\@startsection`
            // leaves `\@afterindentfalse`: the first paragraph of the body
            // is not indented. Everything else the class sets for this
            // branch is what the compiler already produced — `\normalsize`
            // paragraphs at the column measure.
            if let Some(Block::Paragraph { indent, .. }) = blocks.get_mut(first) {
                *indent = false;
            }
            let mut head = section_head_block(texts, document, range);
            take_lead(&mut blocks[first], &mut head, texts, range, style);
            blocks.insert(first, head);
            superseded.push(span);
            continue;
        }
        for block in &mut blocks[first..=last] {
            let Block::Paragraph { indent, style: para_style, env_open, list, sized, .. } = block else { continue };
            // `\list` sets `\parindent\listparindent`, and `\@item`'s
            // `\everypar` drops the `\parindent` box of the first paragraph
            // only to re-add the same width as `\itemindent`: every
            // paragraph of a `quotation` is indented by `\listparindent`.
            *indent = true;
            *para_style = ParaStyle::Quote;
            // The `\list`'s own `\addvspace\@topsep` is absorbed whole by
            // the `\end{center}` skip already on the vertical list: at
            // every class size `\normalsize`'s `\topsep + \partopsep`
            // (10/12/13pt) is larger than `\small`'s plus `\parskip`
            // (6/9/12pt), and `\@xaddvskip` adds nothing to a larger
            // `\lastskip`.
            *env_open = None;
            *list = None;
            *sized = Some(SizedPara {
                size_pt: small.size_pt,
                baselineskip_pt: small.baselineskip_pt,
                parindent_em: Some(LISTPARINDENT_EM),
                vspace_after_em: 0.0,
                close_skip: Some(small.topsepadd()),
            });
        }
        let mut head = head_block(texts, document, range, &small);
        take_lead(&mut blocks[first], &mut head, texts, range, style);
        blocks.insert(first, head);
        superseded.push(span);
    }
    superseded
}

/// Moves the leading vertical skip of the abstract's first body paragraph
/// onto the head inserted in front of it.
///
/// The compiler hangs whatever stood between the previous block and
/// `\begin{abstract}` — a closing `\end{itemize}`'s `\addvspace\@topsepadd`,
/// a `\vspace`, a `\newpage` — on the first block of the body, because that
/// block *was* what followed. Once the head goes in front of it, the head is
/// what follows, and the skip has to travel with that position: left on the
/// body it fires a second time below the head, adding the list's closing
/// `\@topsepadd` on top of the head's own opening `\addvspace` instead of
/// sharing one with it (`\end{itemize}\begin{abstract}` was 5.305 bp long at
/// 10 pt, 5.729 at 11 pt, 6.273 at 12 pt; `\end{center}\begin{abstract}`,
/// whose skip rides on the `center`'s `env_close` rather than on the body,
/// was already right).
///
/// The `\if@twocolumn` branch inserts a `\section*` head the same way, and
/// the same argument applies to it — but its head is a [`Block::Heading`],
/// which has no `addvspace_before` to take the skip, so #728's version of
/// this function fell straight out of its `let else` and left the body
/// carrying it. That is the whole of the two-column `abstract`-after-a-list
/// error #728 measured and filed: `\end{itemize}\begin{abstract}` set the
/// *body* 3.985 bp low at 10 pt, 4.483 at 11 pt and 4.981 at 12 pt while
/// the head itself was exact to the bp, and `center`, `verbatim`,
/// `lstlisting` and a plain paragraph before it — none of which leave a
/// skip on the body — were exact too. `\@startsection` has already spent
/// that `\addvspace`: with `\lastskip` at the list's `\@topsepadd` and a
/// negative before-skip, `\@xaddvskip` takes the else branch and folds the
/// two into one glue *above* the head, which is why the head lands right
/// and nothing of it is left to fire below. So here the skip is dropped
/// rather than moved, and the control that says the drop is right rather
/// than merely smaller is a real `\section*{Abstract}` in the same
/// position, which pdflatex and the pipeline already agree on exactly.
///
/// `\vspace` written *inside* the environment (`\begin{abstract}
/// \vspace{20pt}Body\end{abstract}`) is not "whatever stood between the
/// previous block and `\begin{abstract}`" — it comes after the command, and
/// the compiler's own gap re-read ([`crate::adapter::vspace_in_gap`], used
/// to build `vspace_before` in the first place) cannot tell the two apart:
/// it sums every `\vspace` between the previous block and this one, whether
/// they fall before `\begin{abstract}` or after it. [`inside_vspace`]
/// re-derives the after-the-command part from the source bytes directly, so
/// only the before part travels with the head; the rest is left on the body,
/// between the head and the first line, which is where it belongs.
fn take_lead(body: &mut Block, head: &mut Block, texts: &[&str], range: Range, style: &Stylesheet) {
    let inside = inside_vspace(texts, range, body, style);
    let Block::Paragraph {
        eject_before: b_eject,
        vspace_before: b_vspace,
        addvspace_before: b_addvspace,
        addvspace_flex: b_addflex,
        vspace_flex: b_vflex,
        endlist_adjust: b_endlist,
        ..
    } = body
    else {
        return;
    };
    let total_vspace = *b_vspace;
    let inside = inside.clamp(0.0, total_vspace.max(0.0));
    let lead_vspace = total_vspace - inside;
    // The stretch/shrink of a `\vspace` travels with its own natural part;
    // split it in the same proportion (1.0, i.e. all of it, when nothing of
    // the skip is from inside the environment — the common case, and the
    // one every other branch's flex already assumed).
    let ratio = if total_vspace.abs() > f64::EPSILON { lead_vspace / total_vspace } else { 1.0 };
    *b_vspace = inside;
    let lead_vflex = (b_vflex.0 * ratio, b_vflex.1 * ratio);
    b_vflex.0 -= lead_vflex.0;
    b_vflex.1 -= lead_vflex.1;
    let (eject, vspace, addvspace, addflex, vflex, endlist) = (
        std::mem::replace(b_eject, false),
        lead_vspace,
        std::mem::replace(b_addvspace, 0.0),
        std::mem::replace(b_addflex, (0.0, 0.0)),
        lead_vflex,
        std::mem::replace(b_endlist, 0.0),
    );
    match head {
        Block::Paragraph {
            eject_before: h_eject,
            vspace_before: h_vspace,
            addvspace_before: h_addvspace,
            addvspace_flex: h_addflex,
            vspace_flex: h_vflex,
            endlist_adjust: h_endlist,
            ..
        } => {
            *h_eject = eject;
            *h_vspace = vspace;
            *h_addvspace = addvspace;
            *h_addflex = addflex;
            *h_vflex = vflex;
            *h_endlist = endlist;
        }
        Block::Heading {
            eject_before: h_eject,
            vspace_before: h_vspace,
            ..
        } => {
            // `\newpage` and `\vspace` still travel with the position; the
            // `\addvspace` does not, for the reason above.
            *h_eject = eject;
            *h_vspace = vspace;
        }
        _ => {}
    }
}

/// Whether the [`Branch::TitlePage`] form is set at all for this document.
///
/// `\titlepage` opens with `\if@twocolumn \@restonecoltrue\onecolumn`: in a
/// two-column document the abstract's page is a *one-column* page and the
/// document goes back to two columns after it. This pipeline sets the
/// abstract in the column it finds, which would be a worse answer than
/// none, so a two-column `titlepage` document keeps the compiler's own
/// "not implemented" warning instead.
fn page_form_is_set(style: &Stylesheet) -> bool {
    !style.class_geometry.as_ref().is_some_and(|g| g.options.twocolumn)
}

/// Every [`Branch::TitlePage`] abstract of `blocks`, as the inclusive block
/// index range its page holds (the head this module inserted and the body
/// paragraphs under it).
///
/// Recomputed from the source rather than remembered from [`apply`]:
/// `listings::apply` runs between the two and may insert blocks of its own,
/// so an index recorded there would not survive. Block spans do.
pub fn page_ranges(texts: &[&str], blocks: &[Block], style: &Stylesheet) -> Vec<(usize, usize)> {
    // `titlepage` is a class option (`branch`'s first check, ahead of any
    // per-location `\twocolumn` state), so the document/position passed
    // here are inert whenever this guard actually matters.
    if branch(style, 0, 0) != Branch::TitlePage || !page_form_is_set(style) {
        return Vec::new();
    }
    if style.class_geometry.as_ref().is_some_and(|g| g.options.kind == flashtex_class_geometry::ClassKind::Book) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (d, text) in texts.iter().enumerate() {
        for range in ranges(text) {
            let inside = |b: &Block| -> bool {
                block_span(b).is_some_and(|s| s.document.0 == d && s.start >= range.begin.0 && s.start < range.end.1)
            };
            if let (Some(first), Some(last)) = (blocks.iter().position(inside), blocks.iter().rposition(inside)) {
                out.push((first, last));
            }
        }
    }
    out.sort_unstable();
    out
}

/// Drops the vertical skip and page break the compiler hung on the first
/// block of a [`Branch::TitlePage`] body: the page break the environment
/// opens with supersedes both. A `\vspace` written *inside* the environment
/// (between `\begin{abstract}` and the body's own first character) is not
/// part of what the page break supersedes — it is real glue between the
/// head and the body — so [`inside_vspace`] re-derives it from the source
/// and it is left on `body` rather than dropped with the rest.
fn drop_lead(body: &mut Block, texts: &[&str], range: Range, style: &Stylesheet) {
    let inside = inside_vspace(texts, range, body, style);
    let Block::Paragraph {
        eject_before,
        vspace_before,
        addvspace_before,
        addvspace_flex,
        vspace_flex,
        endlist_adjust,
        ..
    } = body
    else {
        return;
    };
    let inside = inside.clamp(0.0, vspace_before.max(0.0));
    *eject_before = false;
    *vspace_before = inside;
    *addvspace_before = 0.0;
    *addvspace_flex = (0.0, 0.0);
    // The stretch/shrink is dropped along with the rest whenever none of
    // the natural skip survived (the common case); when some did, it is a
    // single `\vspace` inside the environment and its own flex stays whole.
    if inside <= 0.0 {
        *vspace_flex = (0.0, 0.0);
    }
    *endlist_adjust = 0.0;
}

/// The part of `body`'s own `vspace_before` that the source places *after*
/// `\begin{abstract}` (`range.begin.1`) rather than before it — e.g. the
/// `20pt` of `\begin{abstract}\vspace{20pt}Body\end{abstract}`.
///
/// The compiler has no concept of `abstract`'s boundary: `vspace_before` is
/// the sum of every `\vspace` between the previous block and this one,
/// wherever in that gap they fall ([`crate::adapter::vspace_in_gap`]), so
/// [`take_lead`] and [`drop_lead`] cannot tell a `\vspace` that belongs to
/// whatever came before the environment from one the document put inside
/// it. Re-scanning just the part of the gap after `\begin{abstract}` itself
/// answers that directly from source position, which is all `vspace_before`
/// was ever built from.
fn inside_vspace(texts: &[&str], range: Range, body: &Block, style: &Stylesheet) -> f64 {
    let Some(span) = block_span(body) else { return 0.0 };
    if span.start < range.begin.1 {
        return 0.0;
    }
    let Some(text) = texts.get(span.document.0) else { return 0.0 };
    let Some(gap) = text.get(range.begin.1..span.start) else { return 0.0 };
    // The compiler evaluates `em`/`ex` in `\vspace` at a fixed 12pt; LaTeX
    // uses the class's `\normalsize` (see the re-read this mirrors, in
    // `crate::adapter`'s own unit-building loop).
    let size = if style.body_size_pt >= 11.5 {
        12
    } else if style.body_size_pt >= 10.5 {
        11
    } else {
        10
    };
    crate::adapter::vspace_in_gap(gap, size).unwrap_or(0.0)
}

/// `\begin{center}\bfseries \abstractname\end{center}` (report.cls 446-449),
/// the `titlepage` head: one centred bold word at `\normalsize` and the
/// full measure. No `\small` and no `\vspace{-.5em}` — those belong to the
/// other branch, and neither is in this one.
fn page_head_block(texts: &[&str], document: usize, range: Range) -> Block {
    let name = abstract_name(texts).unwrap_or_else(|| ABSTRACTNAME.to_string());
    let src = CharSrc {
        document: flashtex_compiler::DocumentId(document),
        start: range.begin.0,
        end: range.begin.1,
    };
    let word = Word {
        segments: vec![Segment {
            chars: name.chars().map(|_| src).collect(),
            text: name,
            style: TextStyle { bold: true, ..TextStyle::default() },
        }],
    };
    Block::Paragraph {
        parts: vec![ParaPart::Lines(vec![Item::Word(word)])],
        indent: false,
        style: ParaStyle::Center,
        // `\null\vfil` leaves vertical mode, so `center`'s `\@trivlist`
        // takes `\partopsep` as well as `\topsep`; `\end{center}` is an
        // `\@endparenv`, whose `\addvspace{\@topsepadd}` is the whole of
        // the gap between the head and the body (10/12/13 pt at 10/11/12
        // pt, plus the body's own `\baselineskip`).
        env_open: Some(EnvOpen { vmode: true, skips: None }),
        env_close: true,
        eject_before: false,
        vspace_before: 0.0,
        addvspace_before: 0.0,
        addvspace_flex: (0.0, 0.0),
        vspace_flex: (0.0, 0.0),
        endlist_adjust: 0.0,
        list: None,
        sized: None,
        leading_pt: None,
    }
}

/// `\begin{center}{\bfseries \abstractname\vspace{-.5em}\vspace{\z@}}
/// \end{center}` at `\small`: one centred bold word at the full measure,
/// its characters pointing at the `\begin{abstract}` command.
fn head_block(texts: &[&str], document: usize, range: Range, small: &crate::style::SmallSize) -> Block {
    let name = abstract_name(texts).unwrap_or_else(|| ABSTRACTNAME.to_string());
    let src = CharSrc {
        document: flashtex_compiler::DocumentId(document),
        start: range.begin.0,
        end: range.begin.1,
    };
    let word = Word {
        segments: vec![Segment {
            chars: name.chars().map(|_| src).collect(),
            text: name,
            style: TextStyle { bold: true, ..TextStyle::default() },
        }],
    };
    Block::Paragraph {
        parts: vec![ParaPart::Lines(vec![Item::Word(word)])],
        indent: false,
        style: ParaStyle::Center,
        // `\begin{abstract}` is always read in vertical mode: the compiler
        // only emits a body block for it after `\par`, and `\@trivlist`
        // takes `\partopsep` whenever it is.
        env_open: Some(EnvOpen { vmode: true, skips: None }),
        env_close: true,
        eject_before: false,
        vspace_before: 0.0,
        addvspace_before: 0.0,
        addvspace_flex: (0.0, 0.0),
        vspace_flex: (0.0, 0.0),
        endlist_adjust: 0.0,
        list: None,
        sized: Some(SizedPara {
            size_pt: small.size_pt,
            baselineskip_pt: small.baselineskip_pt,
            parindent_em: None,
            vspace_after_em: HEAD_VSPACE_EM,
            // `\end{center}` is a `\trivlist`: `\@topsepadd` is whatever
            // `\topsep`/`\partopsep` hold, which `\small` has not touched.
            close_skip: None,
        }),
        // The head's leading travels on its `SizedPara`, which resizes the
        // whole paragraph; nothing here is a compiler-observed `\par`.
        leading_pt: None,
    }
}

/// `\section*{\abstractname}` (article.cls 379), the `\if@twocolumn`
/// branch: an unnumbered level-1 heading, so `\Large\bfseries` at the
/// column measure with `\@startsection`'s own skips. Starred, so `number`
/// is empty and no `\sectionmark` is issued.
fn section_head_block(texts: &[&str], document: usize, range: Range) -> Block {
    let name = abstract_name(texts).unwrap_or_else(|| ABSTRACTNAME.to_string());
    let src = CharSrc {
        document: flashtex_compiler::DocumentId(document),
        start: range.begin.0,
        end: range.begin.1,
    };
    let word = Word {
        segments: vec![Segment {
            chars: name.chars().map(|_| src).collect(),
            text: name.clone(),
            style: TextStyle::default(),
        }],
    };
    Block::Heading {
        level: 1,
        items: vec![Item::Word(word)],
        eject_before: false,
        vspace_before: 0.0,
        number: String::new(),
        title: name,
        span: Span::in_document(flashtex_compiler::DocumentId(document), range.begin.0, range.begin.1),
    }
}

/// `\renewcommand{\abstractname}{...}` (or the unbraced form) in any of
/// the documents' source; the last one wins, as the last definition does.
fn abstract_name(texts: &[&str]) -> Option<String> {
    let mut name = None;
    for text in texts {
        let mut from = 0;
        while let Some(at) = crate::adapter::find_command(&text[from..], "renewcommand") {
            let abs = from + at;
            from = abs + 1;
            let rest = text[abs + "\\renewcommand".len()..].trim_start();
            let rest = match rest.strip_prefix('{') {
                Some(r) => match r.trim_start().strip_prefix("\\abstractname").map(|r| r.trim_start().strip_prefix('}')) {
                    Some(Some(r)) => r,
                    _ => continue,
                },
                None => match rest.strip_prefix("\\abstractname") {
                    Some(r) => r,
                    None => continue,
                },
            };
            let Some(inner) = rest.trim_start().strip_prefix('{') else { continue };
            let Some(close) = inner.find('}') else { continue };
            name = Some(inner[..close].to_string());
        }
    }
    name
}

/// The first source position a block sets material at.
pub(crate) fn block_span(block: &Block) -> Option<Span> {
    match block {
        Block::Paragraph { parts, .. } => parts.iter().find_map(part_span),
        Block::Heading { span, .. } | Block::Chapter { span, .. } | Block::Part { span, .. } | Block::Title { span, .. } | Block::Rule { span, .. } => Some(*span),
        _ => None,
    }
}

fn part_span(part: &ParaPart) -> Option<Span> {
    match part {
        ParaPart::Lines(items) => items.iter().find_map(|i| match i {
            Item::Word(w) => Some(w.span()),
            Item::Math { span, .. } | Item::Logo { span, .. } | Item::Rule { span, .. } | Item::Footnote { span, .. } => Some(*span),
            _ => None,
        }),
        ParaPart::Display { span, .. } | ParaPart::Rows { span, .. } => Some(*span),
    }
}
