//! The parser edits a few glued words of the expanded stream in place
//! (`\\[2pt]Next`, a row's `\\*`, `\cmidrule(lr)`). On the cached path the
//! stream is borrowed from the thread's expansion cache and every edit is
//! undone before it goes back; a leaked edit would corrupt the next
//! keystroke's reused tokens. Each revision parsed through the warm cache
//! must equal a parse on a fresh thread (empty cache).

use flashtex_compiler::parser::{parse_project, SourceDocument};

fn document(extra: &str) -> String {
    let mut text = String::from("\\documentclass{article}\n\\usepackage{booktabs}\n\\newcommand{\\proj}{FlashTeX}\n\\begin{document}\n");
    for n in 0..40 {
        text.push_str(&format!(
            "Paragraph {n} of \\proj{{}} ends here\\\\[2pt]Next line {n}.\\\\*Starred {n}.\n\n\
             \\begin{{tabular}}{{ll}}\\toprule a & b\\\\\\cmidrule(lr){{1-2}} c{n} & d\\\\*[1pt] e & f\\\\\\bottomrule\\end{{tabular}}\n\n"
        ));
    }
    text.push_str(extra);
    text.push_str("\\end{document}\n");
    text
}

fn snapshot(text: &str) -> String {
    let parsed = parse_project(
        &[SourceDocument {
            path: "main.tex",
            text,
        }],
        "main.tex",
    );
    format!(
        "{:#?}\n{:#?}\n{:?}",
        parsed.blocks, parsed.diagnostics, parsed.preamble_source
    )
}

fn fresh(text: &str) -> String {
    let text = text.to_string();
    std::thread::spawn(move || snapshot(&text))
        .join()
        .expect("fresh parse")
}

#[test]
fn lent_stream_edits_never_leak_into_the_cache() {
    let base = document("");
    assert!(
        base.len() >= 4 * 1024,
        "document must use the incremental cache"
    );
    let anchor = base.find("Paragraph 20").expect("anchor");
    let mut text = base.clone();
    // Warm the cache, then type, delete and retype around glued words.
    assert_eq!(snapshot(&text), fresh(&text));
    for (step, ch) in "xy[2pt]z*".chars().enumerate() {
        text.insert(anchor + step, ch);
        assert_eq!(snapshot(&text), fresh(&text), "after typing {step}");
    }
    for step in 0..9 {
        text.remove(anchor);
        assert_eq!(snapshot(&text), fresh(&text), "after deleting {step}");
    }
    assert_eq!(text, base);
    // An edit far from any split, then one inside a table row.
    let row = text.find("c30 & d").expect("row");
    text.insert_str(row + 3, " more");
    assert_eq!(snapshot(&text), fresh(&text), "table edit");
    // The same revision twice (no-change path).
    assert_eq!(snapshot(&text), fresh(&text), "unchanged revision");
}
