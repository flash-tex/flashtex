//! lipsum `\lipsum[<range>]`: the selected placeholder paragraphs reach the
//! page as ordinary paragraph text.
//!
//! Oracle: the well-known published lipsum paragraph text, transcribed
//! independently here (not read out of `crate::lipsum`, so the test pins
//! the table rather than echoing it). No TeX installation is available in
//! this environment (`kpsewhich` is absent), so this has NOT been verified
//! byte-for-byte against a live pdflatex `lipsum.sty` run.

use flashtex_compiler::parser::{parse_project, Block, Inline, Parsed, SourceDocument};

/// Published lipsum paragraph 1.
const P1: &str = "Lorem ipsum dolor sit amet, consectetuer adipiscing elit. Ut purus elit, vestibulum ut, placerat ac, adipiscing vitae, felis. Curabitur dictum gravida mauris. Nam arcu libero, nonummy eget, consectetuer id, vulputate a, magna. Donec vehicula augue eu neque. Pellentesque habitant morbi tristique senectus et netus et malesuada fames ac turpis egestas. Mauris ut leo. Cras viverra metus rhoncus sem. Nulla et lectus vestibulum urna fringilla ultrices. Phasellus eu tellus sit amet tortor gravida placerat. Integer sapien est, iaculis in, pretium quis, viverra ac, nunc. Praesent eget sem vel leo ultrices bibendum. Aenean faucibus. Morbi dolor nulla, malesuada eu, pulvinar at, mollis ac, nulla. Curabitur auctor semper nulla. Donec varius orci eget risus. Duis nibh mi, congue eu, accumsan eleifend, sagittis quis, diam. Duis eget orci sit amet orci dignissim rutrum.";

/// Published lipsum paragraph 2.
const P2: &str = "Nam dui ligula, fringilla a, euismod sodales, sollicitudin vel, wisi. Morbi auctor lorem vitae tortor. Quisque ullamcorper placerat ipsum. Cras nibh. Morbi vel justo vitae lacus tincidunt ultrices. Lorem ipsum dolor sit amet, consectetuer adipiscing elit. In hac habitasse platea dictumst. Integer tempus convallis augue. Etiam facilisis. Nunc elementum fermentum wisi. Aenean placerat. Ut imperdiet, enim sed gravida sollicitudin, felis odio placerat quam, ac pulvinar elit purus eget enim. Nunc vitae tortor. Proin tempus nibh sit amet nisl. Vivamus quis tortor vitae risus porta vehicula.";

/// Published lipsum paragraph 3.
const P3: &str = "Fusce mauris. Vestibulum luctus nibh at lectus. Sed bibendum, nulla a faucibus semper, leo velit ultricies tellus, ac venenatis arcu wisi vel nisl. Vestibulum diam. Aliquam pellentesque, augue quis sagittis posuere, turpis lacus congue quam, in hendrerit risus eros eget felis. Maecenas eget erat in sapien mattis porttitor. Vestibulum porttitor. Nulla facilisi. Sed a turpis eu lacus commodo facilisis. Morbi fringilla, wisi in dignissim interdum, justo lectus sagittis dui, et vehicula libero dui cursus dui. Mauris tempor ligula sed lacus. Duis cursus enim ut augue. Cras ac magna. Cras nulla. Nulla egestas. Curabitur a leo. Quisque egestas wisi eget nunc. Nam feugiat lacus vel est. Curabitur consectetuer.";

/// Published lipsum paragraph 7 (the default range's last paragraph).
const P7: &str = "Morbi interdum mollis sapien. Sed ac risus. Phasellus lacinia, magna a ullamcorper laoreet, lectus arcu pulvinar risus, vitae facilisis libero dolor a purus. Sed vel lacus. Mauris nibh felis, adipiscing varius, adipiscing in, lacinia vel, tellus. Suspendisse ac urna. Etiam pellentesque mauris ut lectus. Nunc tellus ante, mattis eget, gravida vitae, ultricies ac, leo. Integer leo pede, ornare a, lacinia eu, vulputate vel, nisl.";

fn document(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{lipsum}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

/// Every paragraph's text (word runs joined on their `space_before` glue,
/// exactly as the parser models typed text) with the diagnostic messages.
fn paragraphs_and_messages(parsed: &Parsed) -> (Vec<String>, Vec<String>) {
    let mut paragraphs = Vec::new();
    for block in &parsed.blocks {
        let Block::Paragraph(inlines) = block else {
            continue;
        };
        let mut text = String::new();
        for inline in inlines {
            if let Inline::Text { text: word, space_before, .. } = inline {
                if *space_before && !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(word);
            }
        }
        paragraphs.push(text);
    }
    let messages = parsed.diagnostics.iter().map(|d| d.message.clone()).collect();
    (paragraphs, messages)
}

fn parse(body: &str) -> Parsed {
    let main = document(body);
    parse_project(&[SourceDocument { path: "main.tex", text: &main }], "main.tex")
}

#[test]
fn lipsum_single_paragraph_is_byte_identical() {
    let parsed = parse("\\lipsum[1]\n");
    let (paragraphs, messages) = paragraphs_and_messages(&parsed);
    assert_eq!(messages, Vec::<String>::new());
    assert_eq!(paragraphs, vec![P1.to_string()]);
}

#[test]
fn lipsum_range_is_byte_identical() {
    let parsed = parse("\\lipsum[1-3]\n");
    let (paragraphs, messages) = paragraphs_and_messages(&parsed);
    assert_eq!(messages, Vec::<String>::new());
    assert_eq!(paragraphs, vec![P1.to_string(), P2.to_string(), P3.to_string()]);
}

#[test]
fn lipsum_bare_command_sets_the_default_range() {
    let parsed = parse("\\lipsum\n");
    let (paragraphs, messages) = paragraphs_and_messages(&parsed);
    assert_eq!(messages, Vec::<String>::new());
    assert_eq!(paragraphs.len(), 7);
    assert_eq!(paragraphs[0], P1);
    assert_eq!(paragraphs[6], P7);
}

#[test]
fn lipsum_out_of_range_is_diagnosed_and_sets_nothing() {
    let parsed = parse("\\lipsum[8]\n");
    let (paragraphs, messages) = paragraphs_and_messages(&parsed);
    assert!(paragraphs.is_empty());
    assert_eq!(messages.len(), 1);
    assert!(messages[0].contains("\\lipsum paragraph 8 is out of range"), "{messages:?}");
}

#[test]
fn lipsum_without_the_package_is_diagnosed() {
    let main = "\\documentclass{article}\n\\begin{document}\n\\lipsum[1]\n\\end{document}\n";
    let parsed = parse_project(&[SourceDocument { path: "main.tex", text: main }], "main.tex");
    let (paragraphs, messages) = paragraphs_and_messages(&parsed);
    assert!(paragraphs.is_empty());
    assert!(messages.iter().any(|m| m.contains("\\lipsum needs \\usepackage{lipsum}")), "{messages:?}");
}
