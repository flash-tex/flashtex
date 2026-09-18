//! The skip at a `\trivlist`-to-`\trivlist` boundary that is not two lists.
//!
//! #712 fixed the list-to-list case: `\end{<list>}` is `\endtrivlist` ->
//! `\@endparenv`'s `\addvspace\@topsepadd`, the `\begin{<list>}` beside it is
//! `\@trivlist`'s `\addvspace\@topsep`, and two `\addvspace`s keep the larger
//! natural skip rather than summing — while `\@endparenv`'s `\par` leaves TeX
//! in vertical mode, so the second `\begin` takes `\partopsep` with no blank
//! line between them.
//!
//! Neither half is about *lists*. article.cls builds `center`, `flushleft` and
//! `flushright` as `\trivlist \centering \item\relax`, `quote`, `quotation` and
//! `verse` as `\list{}{...}\item\relax`, and amsthm builds every theorem as a
//! `\trivlist`. All of them end in the same `\endtrivlist`. Measured against
//! pdflatex, three things were wrong at those boundaries:
//!
//! * `\end{<list>}` then `\begin{center|quote|quotation|verse|<theorem>}`
//!   summed the list's closing `\@topsepadd` onto the environment's opening
//!   `\@topsep`: 10 pt + 8 pt = 18 pt where pdflatex puts max(10, 10) = 10 pt,
//!   i.e. **+7.971 bp** at every such boundary;
//! * `\end{<theorem>}` then any `\trivlist` environment, and `\end{quote}`
//!   then `\begin{quote}` (and every other pair the pipeline reads as one
//!   `ParaStyle` run), dropped `\partopsep`: **-1.992 bp**;
//! * both errors accumulated. Six `center`s in a row drifted -1.992, -3.985,
//!   -5.978, -7.970, -9.963 bp; alternating `center`/`itemize` drifted
//!   +5.978, +3.985, +9.963, +7.970 bp.
//!
//! An amsthm theorem is the one case that legitimately steps by `\topsep`
//! alone: `\@thm` assigns `\@topsep`/`\@topsepadd` from `\thm@preskip`/
//! `\thm@postskip`, so it never picks up `\partopsep`. Two adjacent theorems
//! are 19.925 bp apart, and that is pinned below so a fix for the others does
//! not sweep it up.
//!
//! ## The two gaps #722 left, closed here by measurement
//!
//! #722 stopped where its measurements stopped, and said so: `verbatim` and
//! `abstract` are `\trivlist`-derived too but were never measured, and only
//! the 10 pt base size was swept. Both are now measured, over 918 probe
//! documents. Two of the three guesses one would have made were right and
//! one was wrong, which is why they were measured rather than assumed:
//!
//! * **`verbatim` diverged.** `\@verbatim` is `\trivlist \item\relax ...`
//!   and `\endverbatim` is `\endtrivlist`, so it belongs in the same set —
//!   but only the pairs the pipeline reads as one `ParaStyle` run were
//!   wrong, `verbatim`/`verbatim` and `verbatim`/`flushleft`, each short by
//!   exactly one `\partopsep`: **-1.992 bp** at 10 pt, **-2.989** at 11 and
//!   12 pt. Six `verbatim`s in a row drifted to -9.963 bp at 10 pt.
//!   `verbatim` against `center`, `quote`, `verse`, `quotation`,
//!   `flushright`, a theorem or any list was already right.
//! * **`abstract` diverged, on both sides and for two different reasons.**
//!   `\end{abstract}` is `\endquotation` -> `\endlist` -> `\endtrivlist`,
//!   and what followed it was short by one `\partopsep` (-1.992/-2.989/
//!   -0.996 bp at 10/11/12 pt). In the other direction the pipeline builds
//!   the `\small` `\abstractname` head as a block inserted in front of the
//!   body, and the boundary skip the compiler had hung on that body stayed
//!   there, firing a second time below the head: `\end{itemize}` then
//!   `\begin{abstract}` was **+5.305/+5.729/+6.273 bp** long.
//! * **No boundary diverged only at 11 pt or 12 pt.** Every one that was
//!   wrong there was wrong at 10 pt too, and the two fixes above are the
//!   whole of it at all three sizes — so the size sweep found no third bug.
//!   It did find that one boundary is not size-invariant at all, which a
//!   10 pt-only sweep would have pinned as a single rule:
//!   `\end{abstract}\begin{thm}` is `\topsep`-only at 10 pt and 11 pt but
//!   26.401 bp at 12 pt, because which of the two competing `\addvspace`s
//!   is larger changes with the class option ([`Size::abstract_to_thm`]).
//!
//! ## Known gap, not a skip
//!
//! A `\begin{abstract}` on the line *directly* after body text, with no
//! blank line, is not recognised at all: the compiler never breaks the
//! paragraph there, so it reports one run of text across the `\begin`, the
//! pipeline finds no block inside the body to restyle, and no
//! `\abstractname` head is set (the body lands 32.553/36.115/42.744 bp
//! high, on the previous line). That is a compiler-side paragraph-splitting
//! gap, not a boundary skip — `\begin{center}` in the same position does
//! break the paragraph — and fixing it needs a change under `vendor/` and a
//! re-pin, which this line of work has deliberately stayed out of. It is
//! the only divergence left in the 918-document sweep.
//!
//! ## The four #728 left unmeasured, closed here
//!
//! #728 named four things it had not measured. All four are measured now,
//! over 1 482 further probe documents, and the same discipline decided each
//! of them: nothing was added to `TRIVLIST_ENVS` for looking like a
//! `\trivlist`.
//!
//! * **`lstlisting` (listings) diverged, twice, and is *not* a
//!   `\trivlist`.** `\lst@Init` opens no list; the body is an ordinary
//!   paragraph under `\parshape`. What it does have is a `\par` at each
//!   end and a `\vspace` at each end, and both of those had been read as
//!   nothing: `\end{lstlisting}\begin{<trivlist>}` dropped `\partopsep`
//!   (-1.993/-2.989/-2.989 bp) because `\lst@DeInit`'s `\par` leaves
//!   vertical mode just as `\@endparenv`'s does, and
//!   `\end{<list>}\begin{lstlisting}` lost the whole closing `\@topsepadd`
//!   (-9.963/-11.955/-12.951) because `crate::listings` zeroed the block's
//!   `addvspace_before` along with the `flushleft` lowering, although
//!   `\lst@Init`'s skip is a `\vspace` that adds to it rather than
//!   competing. The fix is two separate constants — `VMODE_END_ENVS`, not
//!   `TRIVLIST_ENVS` — and the distinction is the point:
//!   `\end{lstlisting}\begin{thm}` was right all along, exactly as a
//!   non-`\trivlist` should be. See
//!   [`a_lstlisting_boundary_is_the_neighbour_skip_plus_listings_own`].
//! * **`Verbatim` (fancyvrb) and `alltt` are not implemented at all**, and
//!   nothing here pretends otherwise. The engine says so itself —
//!   "packages fancyvrb are recognised but not implemented", "environment
//!   'alltt' is not implemented; its body is typeset as plain text" — and
//!   the measurement agrees: six `alltt` blocks in a row come out on *one*
//!   baseline (-21.917 bp by the second, -109.589 by the sixth at 10 pt),
//!   and a body after a paragraph joins that paragraph's line. There is no
//!   `\trivlist` there to give a boundary skip to; putting them in
//!   `TRIVLIST_ENVS` would have bought the 1.992 bp at `alltt`-to-`center`
//!   while the environment's own body was still 10 bp and a line-break
//!   wrong. They are left alone, and no test pins a number for them.
//! * **`report` and `book` measure exactly like `article`.** `\topsep` and
//!   `\partopsep` come from `\@listI` in the `size1?.clo` all three
//!   classes load, and the whole 8x8 boundary matrix at all three sizes
//!   confirms it, `lstlisting` included; `book`'s only difference is its
//!   first baseline (a larger `\headsep`, not a list skip). The two
//!   class-shaped exceptions are outside a boundary and already declared:
//!   `book.cls` defines no `abstract`, and `report`'s default `titlepage`
//!   branch sets one on a page of its own (head at 317.407 bp, body at
//!   339.325, the following material on the next page), where the pipeline
//!   sets neither page nor head — `crate::abstractenv`'s
//!   `Branch::TitlePage`, which the compiler still warns about.
//!   [`report_and_book_share_the_class_boundary_skips`].
//! * **No boundary in this scope is size-dependent**, and not by luck:
//!   both of listings' skips are `\vspace`s, so at a `lstlisting` boundary
//!   there is no larger-wins comparison for a class option to flip the way
//!   `\end{abstract}\begin{thm}` flips. Forcing `belowskip` to 4, 9 and
//!   20 pt — below, at and above `\topsep` — still measures a plain sum at
//!   all three class sizes ([`the_listings_skips_add_and_never_compete`]).
//!
//! ## The two-column `abstract`, fixed rather than filed
//!
//! #728 measured a two-column `abstract` after a list at 3.985/4.483/4.981
//! bp low and confirmed it pre-existing. It was pre-existing, and it was
//! one line of #728's own argument: the head that goes in front of the body
//! takes the boundary skip with it, except that `\if@twocolumn`'s head is a
//! `Block::Heading`, which has no `addvspace_before`, so `take_lead` fell
//! out of its `let else` and left the skip below the head to fire there.
//! `\@startsection`'s before-skip is negative and has already spent that
//! `\addvspace` above the head — which is why pdflatex puts the head the
//! same distance below an `itemize`, a `center`, a `verbatim`, a theorem
//! and a plain paragraph — so here it is dropped, with a real
//! `\section*{Abstract}` in the same position as the control.
//!
//! The same sweep found a regression #728 had left in the other direction:
//! `abstract` went into `TRIVLIST_ENVS` unconditionally, but article.cls 386
//! closes the environment with `\if@twocolumn\else\endquotation\fi`, so a
//! two-column `\end{abstract}` expands to nothing and leaves no vertical
//! mode. Every two-column `\end{abstract}\begin{<trivlist>}` was one
//! `\partopsep` long (1.992/2.989/2.988 bp) against #722, which was right.
//! Whether that `\end` is an `\endtrivlist` is now a class-option question
//! ([`flashtex_render_pipeline`]'s `abstractenv::end_is_endtrivlist`), not a
//! name lookup. [`a_two_column_abstract_is_a_section_head_and_its_end_is_not_a_trivlist`].
//!
//! ## Oracle
//!
//! pdfTeX 1.40 (MacTeX 2026), T1 Latin Modern, `\pagestyle{empty}`, PyMuPDF
//! glyph origins, in bp, at the 10 pt, 11 pt and 12 pt `article` base sizes.
//! Every probe document below was run through pdflatex; the constants are
//! its measured baselines. pdflatex is an oracle only and never runs in the
//! product path.

mod common;

/// Word x/baseline gate for this project.
const WORD_TOL_BP: f64 = 0.5;

fn head(size: &str) -> String {
    head_of(size, "article", "")
}

/// [`head`] with the class and any extra packages spelled out, for the
/// `report`/`book`, two-column and `listings` sweeps.
fn head_of(options: &str, class: &str, packages: &str) -> String {
    format!(
        "\\documentclass[{options}]{{{class}}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage{{lmodern}}\n\
         \\usepackage{{amsthm}}\n{packages}\\newtheorem{{thm}}{{Theorem}}\n\\pagestyle{{empty}}\n\\begin{{document}}\n"
    )
}

/// One class base size and the four pdflatex baselines this file checks at
/// it.
///
/// Sweeping all three is not a formality. `\topsep` and `\partopsep` are
/// assigned by `\@listI`, which `\@ptsize` selects (`size1?.clo`), so they
/// change with the class option: `\topsep` is 8/9/10 pt and `\partopsep`
/// 2/3/3 pt at 10/11/12 pt, on top of a `\baselineskip` of 12/13.6/14.5 pt.
/// A boundary rule that happens to come out right at 10 pt can be wrong at
/// the other two, and #722 measured only 10 pt.
struct Size {
    /// The `\documentclass` option.
    opt: &'static str,
    /// `\baselineskip`: 12/13.6/14.5 pt. Every step below is this plus
    /// whatever glue the boundary contributes, so it is what is left when
    /// nothing contributes any — two adjacent `lstlisting`s, say, where
    /// neither side uses `\addvspace` at all.
    baseline: f64,
    /// pdflatex's first baseline on an empty `article` page. `report` shares
    /// it; `book`'s is [`Size::book_first`], because book.cls sets a larger
    /// `\headheight`/`\headsep` — a page-geometry difference, not a list
    /// one, and the *steps* below it are article's to the bp.
    first: f64,
    /// pdflatex's first baseline on an empty `book` page.
    book_first: f64,
    /// The step from one `\trivlist` environment to the next one beside it:
    /// `\baselineskip` + max(`\topsep` + `\partopsep`, the same).
    adjacent: f64,
    /// The step when only `\topsep` applies (`\baselineskip` + `\topsep`): a
    /// `\begin` read in horizontal mode, or an amsthm theorem, which sets
    /// `\@topsep`/`\@topsepadd` itself and so never takes `\partopsep`.
    topsep_only: f64,
    /// The step into an `abstract` *body*, which is a whole shape rather
    /// than one skip: the boundary's `\addvspace`, the `\small` centred
    /// `\abstractname` line, its `\vspace{-.5em}`, and the `quotation`'s own
    /// `\topsep` at `\small`.
    into_abstract: f64,
    /// The `abstract` body's own baseline below its `\abstractname` head,
    /// which is what an `abstract` at the very top of a page shows: no
    /// boundary `\addvspace` precedes it, so it is `into_abstract` minus
    /// that skip.
    abstract_body: f64,
    /// `\end{abstract}` then `\begin{thm}` — the one boundary in this file
    /// whose value is not the same *shape* at every size, and the reason
    /// the sweep is worth its runtime. Both sides are unusual: the
    /// `abstract`'s `\@topsepadd` was fixed inside `\small`, so it is
    /// `\small`'s `\topsep` + `\partopsep` (6/9/12 pt), while an amsthm
    /// theorem opens with `\thm@preskip` = `\normalsize`'s `\topsep`
    /// (8/9/10 pt) and no `\partopsep` at all. `\addvspace` keeps the
    /// larger, and which one that is *changes with the class option*: the
    /// theorem wins at 10 pt, they tie at 11 pt, the `abstract` wins at
    /// 12 pt. A 10 pt-only sweep would have pinned the wrong rule.
    abstract_to_thm: f64,
    /// `\end{abstract}` then `\begin{lstlisting}`: the `abstract`'s `\small`
    /// `\@topsepadd` (6/9/12 pt) *plus* `\lst@aboveskip`, because that one
    /// is a `\vspace` and adds rather than competing ([`LST_SKIP`]).
    abstract_to_lst: f64,
    /// The two-column `abstract`'s `\section*` head below whatever precedes
    /// it, and its body below that head. A different shape from the
    /// one-column branch entirely: `\@startsection`, `\Large\bfseries`, no
    /// `\small` and no `quotation`.
    ///
    /// `two_col_head` is one number and not a family because
    /// `\@startsection`'s before-skip is *negative*: with a list's closing
    /// `\@topsepadd` on the vertical list, `\@xaddvskip` takes its else
    /// branch and folds the two into one glue above the head, so the head
    /// lands in the same place after an `itemize`, a `center`, a `verbatim`,
    /// a theorem or a plain paragraph. Nothing of that `\addvspace` is left
    /// to fire below the head, which is the measurement the fix in
    /// `abstractenv::take_lead` rests on.
    two_col_head: f64,
    two_col_body: f64,
}

const SIZES: [Size; 3] = [
    Size { opt: "10pt", baseline: 11.955, first: 134.765, book_first: 133.836, adjacent: 21.9175, topsep_only: 19.925, into_abstract: 36.538, abstract_body: 15.616, abstract_to_thm: 19.925, abstract_to_lst: 23.910, two_col_head: 32.945, two_col_body: 21.821 },
    Size { opt: "11pt", baseline: 13.550, first: 140.742, book_first: 138.624, adjacent: 25.5045, topsep_only: 22.516, into_abstract: 42.092, abstract_body: 18.182, abstract_to_thm: 22.516, abstract_to_lst: 28.493, two_col_head: 34.372, two_col_body: 24.352 },
    Size { opt: "12pt", baseline: 14.446, first: 137.753, book_first: 135.635, adjacent: 27.398, topsep_only: 24.409, into_abstract: 46.729, abstract_body: 20.228, abstract_to_thm: 26.401, abstract_to_lst: 32.379, two_col_head: 39.934, two_col_body: 26.285 },
];

/// `\lst@aboveskip` and `\lst@belowskip`, both `\medskipamount` by default:
/// 6 pt, and 6 pt at *every* class size — `\medskipamount` is not one of
/// the `\@listI` dimensions `size1?.clo` scales.
///
/// The number matters less than what it is. Both are applied with `\vspace`
/// (`\par\penalty-50\relax\vspace\lst@aboveskip`, listings.sty 1762-1766;
/// `\par\penalty-50\vspace\lst@belowskip`, 1818-1820), never `\addvspace`,
/// so a `lstlisting` boundary is a *sum*: whatever the environment on the
/// other side contributes, plus this. Nothing competes, so nothing can flip
/// with the class option the way `\end{abstract}\begin{thm}` does
/// ([`Size::abstract_to_thm`]) — see
/// [`the_listings_skips_add_and_never_compete`], which forces `belowskip`
/// to 4, 9 and 20 pt (below, at and above `\topsep`) and still measures a
/// plain sum at all three sizes.
const LST_SKIP: f64 = 5.978;

/// Every environment measured here, as a single-`\item` block carrying `word`.
fn blk(kind: &str, word: &str) -> String {
    match kind {
        "center" => format!("\\begin{{center}}{word}\\end{{center}}\n"),
        "flushleft" => format!("\\begin{{flushleft}}{word}\\end{{flushleft}}\n"),
        "quote" => format!("\\begin{{quote}}{word}\\end{{quote}}\n"),
        "quotation" => format!("\\begin{{quotation}}{word}\\end{{quotation}}\n"),
        "verse" => format!("\\begin{{verse}}{word}\\end{{verse}}\n"),
        "thm" => format!("\\begin{{thm}}{word}\\end{{thm}}\n"),
        "itemize" => format!("\\begin{{itemize}}\\item {word}\\end{{itemize}}\n"),
        "enumerate" => format!("\\begin{{enumerate}}\\item {word}\\end{{enumerate}}\n"),
        "description" => format!("\\begin{{description}}\\item[Term] {word}\\end{{description}}\n"),
        // `\@verbatim` is `\trivlist \item\relax ...` and `\endverbatim` is
        // `\endtrivlist` (latex.ltx), so `verbatim` is one of these too.
        "verbatim" => format!("\\begin{{verbatim}}\n{word}\n\\end{{verbatim}}\n"),
        // article.cls's one-column `abstract` is `\small`, a centred
        // `\abstractname` and a `quotation`, so `\end{abstract}` is
        // `\endquotation` -> `\endlist` -> `\endtrivlist`.
        "abstract" => format!("\\begin{{abstract}}\n{word}\n\\end{{abstract}}\n"),
        // listings' `lstlisting`, which is *not* a `\trivlist` (see
        // [`a_lstlisting_boundary_is_the_neighbour_skip_plus_listings_own`]).
        "lstlisting" => format!("\\begin{{lstlisting}}\n{word}\n\\end{{lstlisting}}\n"),
        "flushright" => format!("\\begin{{flushright}}{word}\\end{{flushright}}\n"),
        other => panic!("unknown probe environment `{other}`"),
    }
}

const PROBES: [&str; 6] = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot"];

const LISTS: [&str; 3] = ["itemize", "enumerate", "description"];
/// The `\trivlist`/`\list` environments whose `\item` the compiler does not
/// report as a list item.
const SHAPES: [&str; 7] = ["center", "flushleft", "quote", "quotation", "verse", "thm", "verbatim"];

fn words(size: &str, body: &str) -> Vec<common::Word> {
    common::words_of(&common::render_one(&format!("{}{body}\\end{{document}}\n", head(size))))
}

fn at(words: &[common::Word], text: &str) -> f64 {
    words
        .iter()
        .find(|w| w.text.trim().contains(text))
        .unwrap_or_else(|| panic!("no run `{text}` in {:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>()))
        .baseline
}

fn check(label: &str, got: f64, expect: f64) {
    assert!(
        (got - expect).abs() <= WORD_TOL_BP,
        "{label}: {got:.3} bp, pdflatex {expect:.3} bp ({:+.3})",
        got - expect
    );
}

/// The baselines of `kinds` set one after another, separated by `sep`.
fn chain(kinds: &[&str], sep: &str, size: &str) -> Vec<f64> {
    let body = kinds.iter().enumerate().map(|(i, k)| blk(k, PROBES[i])).collect::<Vec<_>>().join(sep);
    let w = words(size, &body);
    kinds.iter().enumerate().map(|(i, _)| at(&w, PROBES[i])).collect()
}

/// A list that opens right after `center`/`quote`/`quotation`/`verse`/a
/// theorem takes one `\topsep` + `\partopsep`, whether or not a blank line
/// separates them. The theorem is the case that was 1.992 bp tight: its
/// `\end` is an `\endtrivlist` like the others, so the `\begin` after it is
/// read in vertical mode.
#[test]
fn a_list_after_a_paragraph_shape_environment_steps_by_topsep_plus_partopsep() {
    if !common::lm_available() {
        eprintln!("SKIP vmode_boundary_skips: Latin Modern not installed");
        return;
    }
    for s in &SIZES {
        for shape in SHAPES {
            for list in LISTS {
                for (sep, what) in [("", "adjacent"), ("\n", "blank line")] {
                    let b = chain(&[shape, list], sep, s.opt);
                    check(&format!("{} {shape} -> {list} ({what}): alpha", s.opt), b[0], s.first);
                    check(&format!("{} {shape} -> {list} ({what}): bravo", s.opt), b[1], s.first + s.adjacent);
                }
            }
        }
    }
}

/// The other direction, which is where the two skips were *summed*: a
/// `center`/`quote`/`quotation`/`verse`/theorem that opens right after
/// `\end{<list>}` was 18 pt below it instead of 10 pt (+7.971 bp).
#[test]
fn a_paragraph_shape_environment_after_a_list_shares_one_addvspace() {
    if !common::lm_available() {
        eprintln!("SKIP vmode_boundary_skips: Latin Modern not installed");
        return;
    }
    for s in &SIZES {
        for list in LISTS {
            for shape in SHAPES {
                let b = chain(&[list, shape], "", s.opt);
                check(&format!("{} {list} -> {shape}: alpha", s.opt), b[0], s.first);
                check(&format!("{} {list} -> {shape}: bravo", s.opt), b[1], s.first + s.adjacent);
            }
        }
    }
}

/// Two paragraph-shape environments beside each other, including two of the
/// same kind — which the pipeline reads as one `ParaStyle` run, and which
/// therefore lost the boundary's `\partopsep` entirely.
#[test]
fn two_adjacent_paragraph_shape_environments_step_by_topsep_plus_partopsep() {
    if !common::lm_available() {
        eprintln!("SKIP vmode_boundary_skips: Latin Modern not installed");
        return;
    }
    for s in &SIZES {
        for a in SHAPES {
            for b in SHAPES {
                // Two amsthm theorems are the documented exception below.
                if a == "thm" && b == "thm" {
                    continue;
                }
                let bl = chain(&[a, b], "", s.opt);
                check(&format!("{} {a} -> {b}: alpha", s.opt), bl[0], s.first);
                check(&format!("{} {a} -> {b}: bravo", s.opt), bl[1], s.first + s.adjacent);
            }
        }
    }
}

/// The measurement that says whether this is one mishandled skip or a rule:
/// six boundaries in a row, which is where the old error grew to -9.963 bp
/// (all `center`) and +7.970 bp (alternating `center`/`itemize`).
#[test]
fn the_boundary_error_does_not_accumulate_across_six_environments() {
    if !common::lm_available() {
        eprintln!("SKIP vmode_boundary_skips: Latin Modern not installed");
        return;
    }
    let chains: [&[&str]; 8] = [
        &["center"; 6],
        &["quote"; 6],
        &["verbatim"; 6],
        &["center", "itemize", "center", "itemize", "center", "itemize"],
        &["quote", "itemize", "quote", "itemize", "quote", "itemize"],
        &["verbatim", "itemize", "verbatim", "itemize", "verbatim", "itemize"],
        &["center", "verbatim", "quote", "verbatim", "verse", "verbatim"],
        &["center", "quote", "itemize", "verse", "thm", "quotation"],
    ];
    for s in &SIZES {
        for kinds in chains {
            let b = chain(kinds, "", s.opt);
            for (k, probe) in PROBES.iter().enumerate() {
                check(&format!("{} {kinds:?}: {probe}", s.opt), b[k], s.first + s.adjacent * k as f64);
            }
        }
    }
}

/// amsthm assigns `\@topsep`/`\@topsepadd` outright, so a theorem takes no
/// `\partopsep` however it is entered: six theorems in a row step by
/// `\topsep` alone, and a theorem after a blank line is no lower than one
/// after a paragraph. This is the case a `\partopsep` fix must not sweep up.
#[test]
fn an_amsthm_theorem_still_steps_by_topsep_alone() {
    if !common::lm_available() {
        eprintln!("SKIP vmode_boundary_skips: Latin Modern not installed");
        return;
    }
    for s in &SIZES {
        let b = chain(&["thm"; 6], "", s.opt);
        for (k, probe) in PROBES.iter().enumerate() {
            check(&format!("{} six theorems: {probe}", s.opt), b[k], s.first + s.topsep_only * k as f64);
        }
        let w = words(s.opt, &format!("Preceding text paragraph.\n\n{}", blk("thm", "alpha")));
        check(&format!("{} theorem after a blank line", s.opt), at(&w, "alpha") - at(&w, "Preceding"), s.topsep_only);
    }
}

/// The control that separates `\topsep` from `\partopsep` for everything
/// else: a `\begin` read in *horizontal* mode (no blank line after the
/// paragraph) is 1.992 bp higher than one read in vertical mode. Unchanged
/// by the boundary fix, which only adds vertical mode after an
/// `\endtrivlist`.
#[test]
fn partopsep_still_applies_only_in_vertical_mode() {
    if !common::lm_available() {
        eprintln!("SKIP vmode_boundary_skips: Latin Modern not installed");
        return;
    }
    for s in &SIZES {
        for kind in ["center", "quote", "quotation", "verse", "verbatim", "itemize", "enumerate", "description"] {
            let body = blk(kind, "alpha");
            let w = words(s.opt, &format!("Preceding text paragraph.\n{body}"));
            check(&format!("{} {kind} after a paragraph", s.opt), at(&w, "alpha") - at(&w, "Preceding"), s.topsep_only);
            let w = words(s.opt, &format!("Preceding text paragraph.\n\n{body}"));
            check(&format!("{} {kind} after a blank line", s.opt), at(&w, "alpha") - at(&w, "Preceding"), s.adjacent);
            check(&format!("{} {kind} alone", s.opt), at(&words(s.opt, &body), "alpha"), s.first);
        }
    }
}

/// `abstract` is the last `\trivlist`-derived environment #722 left
/// unmeasured, and the one whose two sides differ. Its `\end` is an
/// `\endtrivlist` like every other, so what follows takes `\partopsep`
/// (`\end{abstract}\begin{center}` was 1.992 bp tight at 10 pt, 2.989 at
/// 11 pt, 0.996 at 12 pt). Its `\begin` is a whole shape rather than one
/// skip — a `\small` centred `\abstractname` then a `quotation` — and the
/// pipeline inserts that head as a block of its own in front of the body
/// the compiler produced. The skip the compiler hung on that body (a
/// closing `\end{itemize}`'s `\addvspace\@topsepadd`) has to travel to the
/// head with the position: left behind it fired a second time below the
/// head, and `\end{itemize}\begin{abstract}` came out 5.305 bp long at
/// 10 pt, 5.729 at 11 pt and 6.273 at 12 pt. `\end{center}\begin{abstract}`
/// was already right, because a `center`'s closing skip rides on its own
/// `env_close` rather than on the next block.
#[test]
fn an_abstract_shares_one_addvspace_with_the_environment_on_either_side() {
    if !common::lm_available() {
        eprintln!("SKIP vmode_boundary_skips: Latin Modern not installed");
        return;
    }
    for s in &SIZES {
        for (sep, what) in [("", "adjacent"), ("\n", "blank line")] {
            // Into the abstract, from every `\trivlist` and every list.
            for before in SHAPES.iter().chain(LISTS.iter()) {
                let b = chain(&[before, "abstract"], sep, s.opt);
                check(&format!("{} {before} -> abstract ({what}): alpha", s.opt), b[0], s.first);
                check(&format!("{} {before} -> abstract ({what}): bravo", s.opt), b[1], s.first + s.into_abstract);
            }
            // Out of the abstract: an ordinary `\endtrivlist` boundary.
            for after in SHAPES.iter().chain(LISTS.iter()) {
                let b = chain(&["abstract", after], sep, s.opt);
                check(&format!("{} abstract -> {after} ({what}): alpha", s.opt), b[0], s.first + s.abstract_body);
                check(
                    &format!("{} abstract -> {after} ({what}): bravo", s.opt),
                    b[1],
                    s.first + s.abstract_body + if after == &"thm" { s.abstract_to_thm } else { s.adjacent },
                );
            }
        }
        // The `\abstractname` head sits where a first block does, and the
        // body one `into_abstract` below it.
        let w = words(s.opt, &blk("abstract", "alpha"));
        check(&format!("{} abstract alone: head", s.opt), at(&w, "Abstract"), s.first);
        check(&format!("{} abstract alone: body", s.opt), at(&w, "alpha"), s.first + s.abstract_body);
    }
}

/// The baselines of `kinds` set one after another in `class` with
/// `options`, `packages` loaded.
fn chain_in(kinds: &[&str], sep: &str, options: &str, class: &str, packages: &str) -> Vec<f64> {
    let body = kinds.iter().enumerate().map(|(i, k)| blk(k, PROBES[i])).collect::<Vec<_>>().join(sep);
    let w = common::words_of(&common::render_one(&format!("{}{body}\\end{{document}}\n", head_of(options, class, packages))));
    kinds.iter().enumerate().map(|(i, _)| at(&w, PROBES[i])).collect()
}

const LISTINGS: &str = "\\usepackage{listings}\n";

/// The `\addvspace` one side of a boundary contributes, in bp: what
/// `\@xaddvskip` compares the two sides' skips with.
///
/// Three values, and the whole of this file is which one an environment
/// has. A `\trivlist` (and a `\list`) contributes `\topsep + \partopsep`,
/// because an `\endtrivlist` on the other side means the `\begin` is read
/// in vertical mode. amsthm assigns `\@topsep`/`\@topsepadd` outright from
/// `\thm@preskip`/`\thm@postskip`, so a theorem contributes `\topsep` and
/// never `\partopsep`. A `lstlisting` contributes *nothing*: both of its
/// skips are `\vspace`s, which is why it is not in `TRIVLIST_ENVS`.
fn addvspace_of(kind: &str, s: &Size) -> f64 {
    match kind {
        "lstlisting" => 0.0,
        "thm" => s.topsep_only - s.baseline,
        _ => s.adjacent - s.baseline,
    }
}

/// The step pdflatex sets between two environments beside each other:
/// `\baselineskip`, the larger of the two `\addvspace`s (never their sum),
/// and listings' own `\vspace` for each `lstlisting` side (which does sum).
fn step_between(a: &str, b: &str, s: &Size) -> f64 {
    s.baseline
        + addvspace_of(a, s).max(addvspace_of(b, s))
        + LST_SKIP * [a, b].iter().filter(|k| **k == "lstlisting").count() as f64
}

/// `lstlisting` (listings) is the one of the three package environments
/// this project implements, and it is *not* a `\trivlist`: `\lst@Init`
/// opens no list, the body is an ordinary paragraph under `\parshape`
/// (which is why `crate::listings` clears the `flushleft` lowering's
/// `env_open`), and adding it to `TRIVLIST_ENVS` would have been the
/// pattern-match #722 and #728 both refused. Measured, it diverged in two
/// ways for two different reasons, and both are boundaries rather than
/// shapes:
///
/// * **After it**, `\lst@DeInit`'s `\par\removelastskip` /
///   `\par\penalty-50\vspace\lst@belowskip` (listings.sty 1802-1826) leaves
///   TeX in vertical mode exactly as `\@endparenv`'s `\par` does, so the
///   `\begin` beside it takes `\partopsep` — 1.993 bp at 10 pt, 2.989 at 11
///   and 12 pt, against `center`, `flushleft`, `flushright`, `quote`,
///   `quotation`, `verse`, `verbatim` and all three lists.
///   `\end{lstlisting}\begin{thm}`, which takes no `\partopsep` at all, was
///   already right. That is `VMODE_END_ENVS`, kept apart from
///   `TRIVLIST_ENVS` because the route to vertical mode is not an
///   `\endtrivlist`.
/// * **Before it**, the `\addvspace` the closing environment left is not
///   the listing's to drop: `\lst@Init` adds its own skip with `\vspace`,
///   which sums rather than competing. `crate::listings` was zeroing the
///   block's `addvspace_before` along with the `flushleft` lowering, and
///   `\end{itemize}\begin{lstlisting}` came out 9.963 bp short at 10 pt,
///   11.955 at 11 pt and 12.951 at 12 pt — the whole `\@topsepadd` — the
///   same after `enumerate` and `description`, and the same again after
///   `flushleft` and `verbatim`, where the pair reads as one `ParaStyle`
///   run so the closing skip is never materialised at all and the adapter
///   represents the boundary by the next block's `env_open`.
///
/// Every pair is checked in both directions, adjacent and with a blank
/// line, at all three class sizes.
#[test]
fn a_lstlisting_boundary_is_the_neighbour_skip_plus_listings_own() {
    if !common::lm_available() {
        eprintln!("SKIP vmode_boundary_skips: Latin Modern not installed");
        return;
    }
    for s in &SIZES {
        for other in ["center", "flushleft", "flushright", "quote", "quotation", "verse", "thm", "verbatim", "itemize", "enumerate", "description", "lstlisting"] {
            for (sep, what) in [("", "adjacent"), ("\n", "blank line")] {
                let b = chain_in(&["lstlisting", other], sep, s.opt, "article", LISTINGS);
                check(&format!("{} lstlisting -> {other} ({what}): alpha", s.opt), b[0], s.first);
                check(&format!("{} lstlisting -> {other} ({what}): bravo", s.opt), b[1], s.first + step_between("lstlisting", other, s));
                let b = chain_in(&[other, "lstlisting"], sep, s.opt, "article", LISTINGS);
                check(&format!("{} {other} -> lstlisting ({what}): alpha", s.opt), b[0], s.first);
                check(&format!("{} {other} -> lstlisting ({what}): bravo", s.opt), b[1], s.first + step_between(other, "lstlisting", s));
            }
        }
        // `abstract` on either side: its `\end` is an `\endtrivlist` and its
        // `\begin` is a whole shape, and `\small`'s `\@topsepadd` is not the
        // body's.
        let b = chain_in(&["lstlisting", "abstract"], "", s.opt, "article", LISTINGS);
        check(&format!("{} lstlisting -> abstract", s.opt), b[1], b[0] + s.into_abstract + LST_SKIP);
        let b = chain_in(&["abstract", "lstlisting"], "", s.opt, "article", LISTINGS);
        check(&format!("{} abstract -> lstlisting", s.opt), b[1], b[0] + s.abstract_to_lst);
        // Six in a row, and alternating with a list: the error accumulated
        // before, 25.903 bp by the sixth at 10 pt.
        for kinds in [&["lstlisting"; 6][..], &["lstlisting", "itemize", "lstlisting", "itemize", "lstlisting", "itemize"][..]] {
            let b = chain_in(kinds, "", s.opt, "article", LISTINGS);
            let mut expect = s.first;
            for (k, probe) in PROBES.iter().enumerate() {
                if k > 0 {
                    expect += step_between(kinds[k - 1], kinds[k], s);
                }
                check(&format!("{} {kinds:?}: {probe}", s.opt), b[k], expect);
            }
        }
    }
}

/// The control that says a `lstlisting` boundary can never be
/// size-dependent the way `\end{abstract}\begin{thm}` is
/// ([`Size::abstract_to_thm`]), rather than merely happening not to be at
/// these three sizes: both of listings' own skips are applied with
/// `\vspace`, so they *add* to the neighbour's `\addvspace` and there is no
/// larger-wins comparison for a class option to flip.
///
/// `belowskip` is forced below, at and above `\topsep` (4, 9 and 20 pt
/// against 8/9/10 pt). If the two competed, the 4 pt case would be absorbed
/// and the 20 pt case would replace the neighbour's skip; measured, all
/// three are a plain sum at all three class sizes.
#[test]
fn the_listings_skips_add_and_never_compete() {
    if !common::lm_available() {
        eprintln!("SKIP vmode_boundary_skips: Latin Modern not installed");
        return;
    }
    for s in &SIZES {
        for (below, pt) in [("4pt", 3.985), ("9pt", 8.966), ("20pt", 19.925)] {
            let packages = format!("{LISTINGS}\\lstset{{belowskip={below}}}\n");
            for other in ["center", "quote", "itemize", "thm"] {
                let neighbour = if other == "thm" { s.topsep_only } else { s.adjacent };
                let b = chain_in(&["lstlisting", other], "", s.opt, "article", &packages);
                check(&format!("{} belowskip={below} lstlisting -> {other}", s.opt), b[1], b[0] + neighbour + pt);
            }
        }
    }
}

/// `report` and `book`: #728 swept only `article`, and `\topsep` /
/// `\partopsep` come from `\@listI` in `size1?.clo`, which every one of the
/// three classes loads — so the question is whether that is really all of
/// it. Measured over the same boundary matrix at all three sizes, it is:
/// `report` (with `notitlepage`, so its `abstract` is the one-column form)
/// and `book` step by exactly `article`'s numbers, including the
/// `lstlisting` boundaries above.
///
/// The two class-shaped things that are *not* the same are both outside a
/// boundary skip and both already declared: `book.cls` defines no
/// `abstract` at all, and `report`'s default `titlepage` branch sets it on
/// a page of its own between `\null\vfil`s — pdflatex puts its head at
/// 317.407 bp and its body at 339.325 on a page the following material does
/// not share, where the pipeline sets neither the page nor the head
/// (`crate::abstractenv`'s `Branch::TitlePage`, which the compiler still
/// warns about).
#[test]
fn report_and_book_share_the_class_boundary_skips() {
    if !common::lm_available() {
        eprintln!("SKIP vmode_boundary_skips: Latin Modern not installed");
        return;
    }
    for s in &SIZES {
        for (class, options) in [("report", format!("{},notitlepage", s.opt)), ("book", s.opt.to_string())] {
            let first = if class == "book" { s.book_first } else { s.first };
            for a in ["center", "quote", "verse", "thm", "verbatim", "itemize", "description", "lstlisting"] {
                for b in ["center", "quote", "verse", "thm", "verbatim", "itemize", "description", "lstlisting"] {
                    let step = step_between(a, b, s);
                    let bl = chain_in(&[a, b], "", &options, class, LISTINGS);
                    check(&format!("{class} {} {a} -> {b}: alpha", s.opt), bl[0], first);
                    check(&format!("{class} {} {a} -> {b}: bravo", s.opt), bl[1], first + step);
                }
            }
            // `\@listI` itself: a list after a paragraph in horizontal mode
            // is `\topsep` alone, after a blank line `\topsep + \partopsep`.
            let w = common::words_of(&common::render_one(&format!(
                "{}Preceding text paragraph.\n{}\\end{{document}}\n",
                head_of(&options, class, LISTINGS),
                blk("itemize", "alpha")
            )));
            check(&format!("{class} {} \\@listI horizontal", s.opt), at(&w, "alpha") - at(&w, "Preceding"), s.topsep_only);
            let w = common::words_of(&common::render_one(&format!(
                "{}Preceding text paragraph.\n\n{}\\end{{document}}\n",
                head_of(&options, class, LISTINGS),
                blk("itemize", "alpha")
            )));
            check(&format!("{class} {} \\@listI vertical", s.opt), at(&w, "alpha") - at(&w, "Preceding"), s.adjacent);
        }
    }
}

/// The two-column `abstract`, which #728 measured, confirmed pre-existing
/// and filed. It is fixed here, and the fix is one line of the same
/// argument #728 made for the one-column head: the skip the compiler hung
/// on the body has to travel with the position when a head goes in front of
/// it. `\if@twocolumn`'s head is a `Block::Heading` rather than a
/// `Block::Paragraph`, so #728's `take_lead` fell out of its `let else` and
/// moved nothing, and the list's closing `\@topsepadd` stayed below the
/// head and fired there: the body came out 3.985 bp low at 10 pt, 4.483 at
/// 11 pt and 4.981 at 12 pt after `itemize`, `enumerate` and `description`.
///
/// Here it is dropped rather than moved, because `\@startsection` has
/// already spent it: its before-skip is negative, so with a list's
/// `\@topsepadd` as `\lastskip` `\@xaddvskip` folds the two into one glue
/// *above* the head. That is measurable on its own — pdflatex puts the head
/// the same distance below an `itemize`, a `center`, a `verbatim`, a
/// theorem and a plain paragraph — and the control that it is right rather
/// than merely smaller is a real `\section*{Abstract}` in the same
/// position, which the pipeline already matched exactly.
///
/// The same sweep caught a regression #728 left in the other direction:
/// `abstract` had gone into `TRIVLIST_ENVS` unconditionally, but article.cls
/// 386 ends the environment with `\if@twocolumn\else\endquotation\fi`, so a
/// two-column `\end{abstract}` expands to nothing and leaves no vertical
/// mode. Every two-column `\end{abstract}\begin{<trivlist>}` was 1.992 bp
/// long at 10 pt, 2.989 at 11 pt and 2.988 at 12 pt — one `\partopsep` —
/// against #722, which was right.
#[test]
fn a_two_column_abstract_is_a_section_head_and_its_end_is_not_a_trivlist() {
    if !common::lm_available() {
        eprintln!("SKIP vmode_boundary_skips: Latin Modern not installed");
        return;
    }
    for s in &SIZES {
        let options = format!("twocolumn,{}", s.opt);
        for before in ["itemize", "enumerate", "description", "center", "quote", "verbatim", "thm", "lstlisting"] {
            let body = format!("{}{}", blk(before, "alpha"), blk("abstract", "bravo"));
            let w = common::words_of(&common::render_one(&format!(
                "{}{body}\\end{{document}}\n",
                head_of(&options, "article", LISTINGS)
            )));
            let head_step = s.two_col_head + if before == "lstlisting" { LST_SKIP } else { 0.0 };
            check(&format!("{} {before} -> two-column abstract: head", s.opt), at(&w, "Abstract") - at(&w, "alpha"), head_step);
            check(&format!("{} {before} -> two-column abstract: body", s.opt), at(&w, "bravo") - at(&w, "Abstract"), s.two_col_body);
            // Out of it: the `\end` expands to nothing, so TeX is still in
            // horizontal mode and the environment beside it opens with its
            // own `\topsep` and no `\partopsep` — or, for a `lstlisting`,
            // which has no `\addvspace` of its own, with nothing but
            // `\lst@aboveskip`.
            let body = format!("{}{}", blk("abstract", "alpha"), blk(before, "bravo"));
            let w = common::words_of(&common::render_one(&format!(
                "{}{body}\\end{{document}}\n",
                head_of(&options, "article", LISTINGS)
            )));
            let step = if before == "lstlisting" { s.baseline + LST_SKIP } else { s.topsep_only };
            check(&format!("{} two-column abstract -> {before}", s.opt), at(&w, "bravo") - at(&w, "alpha"), step);
        }
        // The control: a real `\section*` where the environment expands to
        // one, and the head alone at the top of a column.
        let w = common::words_of(&common::render_one(&format!(
            "{}{}\\section*{{Abstract}}\nbravo\n\\end{{document}}\n",
            head_of(&options, "article", LISTINGS),
            blk("itemize", "alpha")
        )));
        check(&format!("{} itemize -> \\section* control: head", s.opt), at(&w, "Abstract") - at(&w, "alpha"), s.two_col_head);
        check(&format!("{} itemize -> \\section* control: body", s.opt), at(&w, "bravo") - at(&w, "Abstract"), s.two_col_body);
        let w = common::words_of(&common::render_one(&format!(
            "{}{}\\end{{document}}\n",
            head_of(&options, "article", LISTINGS),
            blk("abstract", "alpha")
        )));
        check(&format!("{} two-column abstract alone: head", s.opt), at(&w, "Abstract"), s.first);
        check(&format!("{} two-column abstract alone: body", s.opt), at(&w, "alpha") - at(&w, "Abstract"), s.two_col_body);
    }
}
