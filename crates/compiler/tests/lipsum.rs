//! lipsum `\lipsum[<range>]`: the selected placeholder paragraphs reach the
//! page as ordinary paragraph text.
//!
//! Oracle: paragraphs 1-7 below are a checked-in copy of the real
//! `lipsum.ltd.tex` `\Lorem<N>` definitions (TeX Live 2026, lipsum v2.7
//! dated 2021-09-20; file dated 2021-09-20), transcribed independently
//! here — newlines collapsed to single spaces exactly as TeX collapses
//! them — not read out of `crate::lipsum`, so the test pins the table
//! rather than echoing it.

use flashtex_compiler::parser::{parse_project, Block, Inline, Parsed, SourceDocument};

/// Real `lipsum.ltd.tex` paragraph 1 (`\Lorem1`).
const P1: &str = "Lorem ipsum dolor sit amet, consectetuer adipiscing elit. Ut purus elit, vestibulum ut, placerat ac, adipiscing vitae, felis. Curabitur dictum gravida mauris. Nam arcu libero, nonummy eget, consectetuer id, vulputate a, magna. Donec vehicula augue eu neque. Pellentesque habitant morbi tristique senectus et netus et malesuada fames ac turpis egestas. Mauris ut leo. Cras viverra metus rhoncus sem. Nulla et lectus vestibulum urna fringilla ultrices. Phasellus eu tellus sit amet tortor gravida placerat. Integer sapien est, iaculis in, pretium quis, viverra ac, nunc. Praesent eget sem vel leo ultrices bibendum. Aenean faucibus. Morbi dolor nulla, malesuada eu, pulvinar at, mollis ac, nulla. Curabitur auctor semper nulla. Donec varius orci eget risus. Duis nibh mi, congue eu, accumsan eleifend, sagittis quis, diam. Duis eget orci sit amet orci dignissim rutrum.";

/// Real `lipsum.ltd.tex` paragraph 2 (`\Lorem2`).
const P2: &str = "Nam dui ligula, fringilla a, euismod sodales, sollicitudin vel, wisi. Morbi auctor lorem non justo. Nam lacus libero, pretium at, lobortis vitae, ultricies et, tellus. Donec aliquet, tortor sed accumsan bibendum, erat ligula aliquet magna, vitae ornare odio metus a mi. Morbi ac orci et nisl hendrerit mollis. Suspendisse ut massa. Cras nec ante. Pellentesque a nulla. Cum sociis natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Aliquam tincidunt urna. Nulla ullamcorper vestibulum turpis. Pellentesque cursus luctus mauris.";

/// Real `lipsum.ltd.tex` paragraph 3 (`\Lorem3`).
const P3: &str = "Nulla malesuada porttitor diam. Donec felis erat, congue non, volutpat at, tincidunt tristique, libero. Vivamus viverra fermentum felis. Donec nonummy pellentesque ante. Phasellus adipiscing semper elit. Proin fermentum massa ac quam. Sed diam turpis, molestie vitae, placerat a, molestie nec, leo. Maecenas lacinia. Nam ipsum ligula, eleifend at, accumsan nec, suscipit a, ipsum. Morbi blandit ligula feugiat magna. Nunc eleifend consequat lorem. Sed lacinia nulla vitae enim. Pellentesque tincidunt purus vel magna. Integer non enim. Praesent euismod nunc eu purus. Donec bibendum quam in tellus. Nullam cursus pulvinar lectus. Donec et mi. Nam vulputate metus eu enim. Vestibulum pellentesque felis eu massa.";

/// Real `lipsum.ltd.tex` paragraph 4 (`\Lorem4`).
const P4: &str = "Quisque ullamcorper placerat ipsum. Cras nibh. Morbi vel justo vitae lacus tincidunt ultrices. Lorem ipsum dolor sit amet, consectetuer adipiscing elit. In hac habitasse platea dictumst. Integer tempus convallis augue. Etiam facilisis. Nunc elementum fermentum wisi. Aenean placerat. Ut imperdiet, enim sed gravida sollicitudin, felis odio placerat quam, ac pulvinar elit purus eget enim. Nunc vitae tortor. Proin tempus nibh sit amet nisl. Vivamus quis tortor vitae risus porta vehicula.";

/// Real `lipsum.ltd.tex` paragraph 5 (`\Lorem5`).
const P5: &str = "Fusce mauris. Vestibulum luctus nibh at lectus. Sed bibendum, nulla a faucibus semper, leo velit ultricies tellus, ac venenatis arcu wisi vel nisl. Vestibulum diam. Aliquam pellentesque, augue quis sagittis posuere, turpis lacus congue quam, in hendrerit risus eros eget felis. Maecenas eget erat in sapien mattis porttitor. Vestibulum porttitor. Nulla facilisi. Sed a turpis eu lacus commodo facilisis. Morbi fringilla, wisi in dignissim interdum, justo lectus sagittis dui, et vehicula libero dui cursus dui. Mauris tempor ligula sed lacus. Duis cursus enim ut augue. Cras ac magna. Cras nulla. Nulla egestas. Curabitur a leo. Quisque egestas wisi eget nunc. Nam feugiat lacus vel est. Curabitur consectetuer.";

/// Real `lipsum.ltd.tex` paragraph 6 (`\Lorem6`).
const P6: &str = "Suspendisse vel felis. Ut lorem lorem, interdum eu, tincidunt sit amet, laoreet vitae, arcu. Aenean faucibus pede eu ante. Praesent enim elit, rutrum at, molestie non, nonummy vel, nisl. Ut lectus eros, malesuada sit amet, fermentum eu, sodales cursus, magna. Donec eu purus. Quisque vehicula, urna sed ultricies auctor, pede lorem egestas dui, et convallis elit erat sed nulla. Donec luctus. Curabitur et nunc. Aliquam dolor odio, commodo pretium, ultricies non, pharetra in, velit. Integer arcu est, nonummy in, fermentum faucibus, egestas vel, odio.";

/// Real `lipsum.ltd.tex` paragraph 7 (`\Lorem7`; the default range's last paragraph).
const P7: &str = "Sed commodo posuere pede. Mauris ut est. Ut quis purus. Sed ac odio. Sed vehicula hendrerit sem. Duis non odio. Morbi ut dui. Sed accumsan risus eget odio. In hac habitasse platea dictumst. Pellentesque non elit. Fusce sed justo eu urna porta tincidunt. Mauris felis odio, sollicitudin sed, volutpat a, ornare ac, erat. Morbi quis dolor. Donec pellentesque, erat ac sagittis semper, nunc dui lobortis purus, quis congue purus metus ultricies tellus. Proin et quam. Class aptent taciti sociosqu ad litora torquent per conubia nostra, per inceptos hymenaeos. Praesent sapien turpis, fermentum vel, eleifend faucibus, vehicula eu, lacus.";

/// Every paragraph this implementation ships, in `\lipsum` number order.
const EXPECTED: &[&str] = &[P1, P2, P3, P4, P5, P6, P7];

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
fn lipsum_every_paragraph_matches_lipsum_ltd_tex() {
    for (index, expected) in EXPECTED.iter().enumerate() {
        let parsed = parse(&format!("\\lipsum[{}]\n", index + 1));
        let (paragraphs, messages) = paragraphs_and_messages(&parsed);
        assert_eq!(messages, Vec::<String>::new(), "paragraph {}", index + 1);
        assert_eq!(paragraphs, vec![expected.to_string()], "paragraph {}", index + 1);
    }
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
