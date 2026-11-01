//! The `letter` document class.
//!
//! Before this, a correct letter was told its commands did not exist:
//! `fixtures/real-world/letter/main.tex` raised **seven `unknown_command`
//! errors** (`\signature`, `\address`, `\opening`, `\closing`, `\encl`,
//! `\cc`, `\ps`) plus a warning that the `letter` environment was not
//! implemented, and its body was typeset as undifferentiated prose.
//!
//! The oracle for everything below is the committed
//! `fixtures/real-world/letter/reference.pdf`, produced by pdflatex from
//! that exact source. pdflatex is an oracle only and never runs here or in
//! the product. Its 26 typeset lines, as word positions in bp with y from
//! the top of the page:
//!
//! ```text
//!  124.907 x 437.195  123 Example Street
//!  138.456 x 437.195  Pittsburgh, PA 15213
//!  167.278 x 437.195  September 12, 2026
//!  202.761 x  72.000  Admissions Committee        (+3 more recipient lines)
//!  279.867 x  72.000  Dear Members of the Committee,
//!  ...                body paragraphs
//!  520.091 x 251.328  Sincerely,
//!  579.458 x 251.328  Jordan Example
//!  601.640 x  72.000  encl: Curriculum vitae; transcript; writing sample
//!  622.826 x  72.000  cc: Professor A. Author
//!  644.011 x  72.000  P.S. My application number is 2027-0412.
//! ```
//!
//! Three things that reads off directly, and that these tests pin:
//!
//! * the return address is a **left-aligned box pushed right**, not a
//!   ragged-left column: all three of its lines start at the same x
//!   (437.195) although they are different lengths. `\opening` sets them in
//!   a `tabular{l@{}}` inside `\raggedleft`, and `\raggedleft` moves the
//!   box, not its lines.
//! * `\longindentation` is **180pt**, half the *class's* `\textwidth`
//!   (360pt at 11pt) — not half the measure, which `[margin=1in]` geometry
//!   made 469.75502pt. 251.328bp − 72bp = 180pt exactly.
//! * `\ps` takes **no argument** (letter.cls 245 is
//!   `\newcommand*\ps{\par\startbreaks}`), so `\ps{P.S. ...}` typesets its
//!   brace group as ordinary text at the left margin — which the reference's
//!   last line is. Consuming it would delete the author's sentence.
//!
//! This compiler's own layout is a fixed 612x792 page with 72pt margins and
//! Core-14 Times metrics, so absolute positions cannot match pdflatex and
//! are not compared. The *skips between blocks* can and do: see
//! `letter_block_skips_follow_the_class`.

use flashtex_compiler::diagnostics::DiagnosticCode;
use flashtex_compiler::incremental::compile_full;
use flashtex_compiler::layout::{LayoutConstraints, Page};
use flashtex_compiler::parser::{self, Block, LetterPart};

/// The corpus fixture, byte for byte.
const FIXTURE: &str = include_str!("../../../fixtures/real-world/letter/main.tex");

/// `\parskip` at 11pt: letter.cls's `0.7em`, as pdflatex prints it.
const PARSKIP_11PT: f64 = 7.66498;
/// This layout's `\baselineskip` at 11pt (`LINE_SPACING` 1.2).
const BASELINESKIP_11PT: f64 = 13.2;

fn compiled(text: &str) -> flashtex_compiler::incremental::CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

/// One typeset line: its baseline and the x of its leftmost item.
fn lines(pages: &[Page]) -> Vec<(usize, f64, f64, String)> {
    let mut out: Vec<(usize, f64, f64, String)> = Vec::new();
    for (index, page) in pages.iter().enumerate() {
        let mut by_baseline: Vec<(f64, Vec<&flashtex_compiler::layout::TextItem>)> = Vec::new();
        for item in &page.items {
            match by_baseline
                .iter_mut()
                .find(|(y, _)| (*y - item.baseline_y_pt).abs() < 1e-6)
            {
                Some((_, items)) => items.push(item),
                None => by_baseline.push((item.baseline_y_pt, vec![item])),
            }
        }
        by_baseline.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("finite baselines"));
        for (y, mut items) in by_baseline {
            items.sort_by(|a, b| a.x_pt.partial_cmp(&b.x_pt).expect("finite x"));
            let text = items
                .iter()
                .map(|i| i.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            out.push((index, y, items[0].x_pt, text));
        }
    }
    out
}

fn line_starting_with<'a>(
    lines: &'a [(usize, f64, f64, String)],
    prefix: &str,
) -> &'a (usize, f64, f64, String) {
    lines
        .iter()
        .find(|(_, _, _, text)| text.starts_with(prefix))
        .unwrap_or_else(|| panic!("no line starting {prefix:?} in {lines:#?}"))
}

fn close(got: f64, want: f64, what: &str) {
    assert!(
        (got - want).abs() < 0.01,
        "{what}: got {got}, want {want} (delta {})",
        got - want
    );
}

#[test]
fn the_corpus_letter_compiles_with_no_diagnostics_at_all() {
    let out = compiled(FIXTURE);
    assert!(
        out.diagnostics.is_empty(),
        "{:#?}",
        out.diagnostics
            .iter()
            .map(|d| (&d.message, d.code))
            .collect::<Vec<_>>()
    );
    assert_eq!(out.pages.len(), 1);
    // The same 26 lines pdflatex sets, in the same order. Line breaking
    // inside the body differs (Times metrics, a different measure), so only
    // the letter-structure lines are named.
    let lines = lines(&out.pages);
    assert_eq!(lines.len(), 26, "{lines:#?}");
    for (index, prefix) in [
        (0usize, "123 Example Street"),
        (1, "Pittsburgh, PA 15213"),
        (2, "September 12, 2026"),
        (3, "Admissions Committee"),
        (7, "Dear Members of the Committee,"),
        (21, "Sincerely,"),
        (22, "Jordan Example"),
        (23, "encl: Curriculum vitae"),
        (24, "cc: Professor"),
        (25, "P.S. My application number is 2027-0412."),
    ] {
        assert!(
            lines[index].3.starts_with(prefix),
            "line {index}: {:?} does not start {prefix:?}",
            lines[index].3
        );
    }
}

/// `\parskip` is the class's, not the standard classes' `0pt plus 1pt`.
/// Every inter-paragraph gap in the reference is 21.185bp = 21.264pt =
/// `\baselineskip` 13.6pt + `\parskip` 7.66498pt, which is how this value is
/// confirmed rather than assumed.
#[test]
fn the_letter_class_sets_its_own_parskip() {
    let parsed = parser::parse(FIXTURE);
    assert_eq!(parsed.parskip_pt, Some(PARSKIP_11PT));
    assert_eq!(parsed.document_class.as_deref(), Some("letter"));
    // 10pt is the class default (`\ExecuteOptions{letterpaper,10pt,...}`).
    let ten = parser::parse("\\documentclass{letter}\\begin{document}x\\end{document}");
    assert_eq!(ten.parskip_pt, Some(6.99997));
    let twelve = parser::parse("\\documentclass[12pt]{letter}\\begin{document}x\\end{document}");
    assert_eq!(twelve.parskip_pt, Some(8.22487));
    // An explicit `\setlength` still wins, as it would in real LaTeX.
    let overridden = parser::parse(
        "\\documentclass[11pt]{letter}\\setlength{\\parskip}{3pt}\\begin{document}x\\end{document}",
    );
    assert_eq!(overridden.parskip_pt, Some(3.0));
}

/// Every skip letter.cls writes, checked against the class's own
/// definitions. Six of the nine are also exact against pdflatex once this
/// layout's `\baselineskip` (13.2pt) is swapped for the class's (13.6pt);
/// the three that are not are named in the test, and every one of them is a
/// TeX *box* height or depth that a flat line model has no representation
/// for.
#[test]
fn letter_block_skips_follow_the_class() {
    let out = compiled(FIXTURE);
    let lines = lines(&out.pages);
    let y = |prefix: &str| line_starting_with(&lines, prefix).1;

    // `\\*[2\parskip]` between the address and the date (letter.cls 227).
    // pdflatex: 28.822bp = 28.930pt; class model 13.6 + 2 x 7.66498 =
    // 28.930pt. Exact.
    close(
        y("September 12, 2026") - y("Pittsburgh, PA 15213"),
        BASELINESKIP_11PT + 2.0 * PARSKIP_11PT,
        "address -> date",
    );
    // `\vspace{2\parskip}` plus the recipient paragraph's own `\parskip`
    // (letter.cls 230-231). pdflatex: 35.483bp = 35.616pt against the class
    // model's 36.595pt. The 0.98pt is the `tabular` box's depth: TeX sets
    // interline glue from the *box* above, not from a text line.
    close(
        y("Admissions Committee") - y("September 12, 2026"),
        BASELINESKIP_11PT + 3.0 * PARSKIP_11PT,
        "date -> recipient",
    );
    // `\vspace{2\parskip}` then `#1\par` (letter.cls 232-234). pdflatex:
    // 36.458bp = 36.595pt; class model 36.595pt. Exact. Indices, not text:
    // "Pittsburgh, PA 15213" is both the second address line and the last
    // recipient line, and it is the recipient's that matters here.
    assert_eq!(lines[6].3, "Pittsburgh, PA 15213");
    assert!(lines[7].3.starts_with("Dear Members"));
    close(
        lines[7].1 - lines[6].1,
        BASELINESKIP_11PT + 3.0 * PARSKIP_11PT,
        "recipient -> salutation",
    );
    // `\closing`'s `\vspace{\parskip}` on top of the paragraph's own
    // (letter.cls 235). pdflatex: 25.903bp = 26.000pt against the class
    // model's 28.930pt. The 2.93pt is `\parbox`'s default `[c]` centring:
    // the closing box is centred on its baseline, so its first line sits
    // higher than an ordinary paragraph's would.
    close(
        y("Sincerely,") - y("Thank you for your consideration."),
        BASELINESKIP_11PT + 2.0 * PARSKIP_11PT,
        "body -> closing",
    );
    // `\\[6\medskipamount]` with `\medskipamount = \parskip` (letter.cls
    // 236, 242). pdflatex: 59.367bp = 59.590pt; class model 13.6 + 6 x
    // 7.66498 = 59.590pt. Exact.
    close(
        y("Jordan Example") - y("Sincerely,"),
        BASELINESKIP_11PT + 6.0 * PARSKIP_11PT,
        "closing -> signature",
    );
    // `\cc` and `\encl` are ordinary paragraphs. pdflatex: 21.186bp and
    // 21.185bp = 21.265pt; class model 13.6 + 7.66498 = 21.265pt. Exact.
    close(
        y("cc: Professor") - y("encl: Curriculum vitae"),
        BASELINESKIP_11PT + PARSKIP_11PT,
        "encl -> cc",
    );
    close(
        y("P.S. My application number is 2027-0412.") - y("cc: Professor"),
        BASELINESKIP_11PT + PARSKIP_11PT,
        "cc -> ps",
    );
}

/// `\raggedleft` around a `tabular` moves the box, not its lines: in the
/// reference all three lines of the return address start at 437.195bp even
/// though they are different lengths. A `flushright` paragraph would put
/// each line's *right* edge at the margin instead, which is a different
/// picture.
#[test]
fn the_return_address_is_one_right_aligned_box_of_left_aligned_lines() {
    let out = compiled(FIXTURE);
    let lines = lines(&out.pages);
    let street = line_starting_with(&lines, "123 Example Street").2;
    let city = line_starting_with(&lines, "Pittsburgh, PA 15213").2;
    let date = line_starting_with(&lines, "September 12, 2026").2;
    close(city, street, "address lines share a left edge");
    close(date, street, "the date shares the address's left edge");
    let recipient = line_starting_with(&lines, "Admissions Committee").2;
    assert!(
        street > recipient,
        "the address box sits right of the recipient ({street} vs {recipient})"
    );
    // And its right edge is at the measure, not beyond it.
    let page = &out.pages[0];
    let right = page
        .items
        .iter()
        .filter(|i| (i.baseline_y_pt - line_starting_with(&lines, "123 Example Street").1).abs() < 1e-6)
        .map(|i| i.x_pt)
        .fold(f64::MIN, f64::max);
    assert!(right < page.width_pt, "{right} vs {}", page.width_pt);
}

/// `\longindentation` is a class length — `.5\textwidth` of the *class's*
/// 360pt at 11pt — assigned once when letter.cls loads and never updated by
/// `geometry`. The reference puts "Sincerely," at 251.328bp, exactly 180pt
/// right of the 72bp margin, with a 469.75502pt measure.
#[test]
fn the_closing_sits_at_longindentation_not_half_the_measure() {
    let out = compiled(FIXTURE);
    let lines = lines(&out.pages);
    let closing = line_starting_with(&lines, "Sincerely,").2;
    let signature = line_starting_with(&lines, "Jordan Example").2;
    close(signature, closing, "closing and signature share a left edge");
    let margin = line_starting_with(&lines, "Admissions Committee").2;
    close(closing - margin, 180.0, "\\longindentation at 11pt");
    assert!(
        (closing - margin - out.pages[0].width_pt / 2.0).abs() > 1.0,
        "must not be half the measure"
    );
}

/// letter.cls 239: the `\hspace*{\longindentation}` is conditional on
/// `\fromaddress`. A letter with no `\address` closes at the left margin.
#[test]
fn a_letter_without_an_address_closes_at_the_margin() {
    let source = "\\documentclass[11pt]{letter}\n\\signature{A. Author}\n\\begin{document}\n\\begin{letter}{A Name}\n\\opening{Dear reader,}\nBody.\n\\closing{Yours,}\n\\end{letter}\n\\end{document}\n";
    let out = compiled(source);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let lines = lines(&out.pages);
    let closing = line_starting_with(&lines, "Yours,").2;
    let body = line_starting_with(&lines, "Body.").2;
    close(closing, body, "no address, so no \\longindentation");
    // With no `\address` the box holds only `\@date` (letter.cls 224).
    let blocks = parser::parse(source).blocks;
    let address = blocks
        .iter()
        .find_map(|b| match b {
            Block::LetterBlock {
                part: LetterPart::ReturnAddress,
                lines,
                ..
            } => Some(lines.len()),
            _ => None,
        })
        .expect("a return-address block");
    assert_eq!(address, 1, "the date alone");
}

/// `\ps` is `\par\startbreaks`: no argument. Consuming one would delete the
/// author's postscript, which the reference typesets in full.
#[test]
fn ps_takes_no_argument_and_never_swallows_its_brace_group() {
    let source = "\\documentclass[11pt]{letter}\n\\begin{document}\n\\begin{letter}{A Name}\n\\opening{Dear reader,}\nBody.\n\\ps{P.S. Do not eat me.}\n\\end{letter}\n\\end{document}\n";
    let out = compiled(source);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let lines = lines(&out.pages);
    let ps = line_starting_with(&lines, "P.S. Do not eat me.");
    let body = line_starting_with(&lines, "Body.");
    close(ps.2, body.2, "the postscript is at the left margin");
    assert!(ps.1 > body.1, "and it is its own paragraph, below the body");
}

/// `\signature` wins over `\name`; `\name` is the fallback (letter.cls 241).
#[test]
fn the_signature_falls_back_to_the_name() {
    let with_name = "\\documentclass[11pt]{letter}\n\\name{From Name}\n\\begin{document}\n\\begin{letter}{A Name}\n\\opening{Dear reader,}\nBody.\n\\closing{Yours,}\n\\end{letter}\n\\end{document}\n";
    let only_name = lines(&compiled(with_name).pages);
    assert!(
        only_name.iter().any(|(_, _, _, t)| t == "From Name"),
        "{only_name:#?}"
    );

    let both = with_name.replace("\\name{From Name}", "\\name{From Name}\n\\signature{From Sig}");
    let with_both = lines(&compiled(&both).pages);
    assert!(
        with_both.iter().any(|(_, _, _, t)| t == "From Sig"),
        "{with_both:#?}"
    );
    assert!(
        !with_both.iter().any(|(_, _, _, t)| t == "From Name"),
        "\\signature replaces \\name, it does not add to it"
    );
}

/// letter.cls 174-176: `\begin{letter}` opens with `\newpage`. The first
/// letter does not get a blank page before it, and `\end{letter}` does not
/// add a trailing one — `\end{document}`'s own `\clearpage` absorbs its
/// `\pagebreak` in real LaTeX.
#[test]
fn each_letter_after_the_first_starts_a_new_page() {
    let one = "\\documentclass[11pt]{letter}\n\\begin{document}\n\\begin{letter}{First}\n\\opening{Dear one,}\nBody one.\n\\end{letter}\n\\end{document}\n";
    assert_eq!(compiled(one).pages.len(), 1);
    let two = "\\documentclass[11pt]{letter}\n\\begin{document}\n\\begin{letter}{First}\n\\opening{Dear one,}\nBody one.\n\\end{letter}\n\\begin{letter}{Second}\n\\opening{Dear two,}\nBody two.\n\\end{letter}\n\\end{document}\n";
    let out = compiled(two);
    assert_eq!(out.pages.len(), 2, "{:#?}", out.diagnostics);
    let lines = lines(&out.pages);
    assert_eq!(line_starting_with(&lines, "Body one.").0, 0);
    assert_eq!(line_starting_with(&lines, "Body two.").0, 1);
    // The second letter's recipient really is the second one.
    assert!(lines.iter().any(|(page, _, _, t)| *page == 1 && t == "Second"));
}

/// Every one of these is defined by letter.cls alone. In an `article`
/// pdflatex answers "Undefined control sequence", so this compiler must say
/// the command is not available — and must NOT eat the brace group, which
/// real LaTeX typesets after the error.
#[test]
fn letter_commands_outside_the_letter_class_are_diagnosed_without_losing_text() {
    for (command, argument) in [
        ("address", "{1 Example Street}"),
        ("signature", "{A. Author}"),
        ("opening", "{Dear reader,}"),
        ("closing", "{Yours,}"),
        ("cc", "{Someone}"),
        ("encl", "{Something}"),
        ("ps", "{A postscript.}"),
    ] {
        let source = format!(
            "\\documentclass{{article}}\n\\begin{{document}}\n\\{command}{argument}\n\\end{{document}}\n"
        );
        let out = compiled(&source);
        let named: Vec<_> = out
            .diagnostics
            .iter()
            .filter(|d| d.message.contains(&format!("\\{command}")))
            .collect();
        assert_eq!(named.len(), 1, "{command}: {:#?}", out.diagnostics);
        assert_eq!(
            named[0].code,
            Some(DiagnosticCode::UnknownCommand),
            "{command}: pdflatex's own answer is an undefined control sequence"
        );
        let text: String = out
            .pages
            .iter()
            .flat_map(|p| p.items.iter())
            .map(|i| i.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let inner = argument.trim_matches(|c| c == '{' || c == '}');
        let first = inner.split_whitespace().next().expect("a word");
        assert!(
            text.contains(first),
            "{command}: the argument text was swallowed ({text:?})"
        );
    }
}

/// `\opening` outside a `letter` environment has no `\toname`/`\toaddress`
/// to set. That is worth saying, but it must not stop the return address,
/// the date and the salutation from being typeset.
#[test]
fn opening_outside_a_letter_environment_reports_the_missing_recipient() {
    let source = "\\documentclass[11pt]{letter}\n\\address{1 Example Street}\n\\begin{document}\n\\opening{Dear reader,}\nBody.\n\\end{document}\n";
    let out = compiled(source);
    let warnings: Vec<_> = out
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("\\begin{letter}"))
        .collect();
    assert_eq!(warnings.len(), 1, "{:#?}", out.diagnostics);
    let lines = lines(&out.pages);
    assert!(lines.iter().any(|(_, _, _, t)| t == "1 Example Street"));
    assert!(lines
        .iter()
        .any(|(_, _, _, t)| t.starts_with("Dear reader,")));
}
