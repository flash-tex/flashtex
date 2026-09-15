//! A long run of spaces before a non-space character is lexed in linear
//! time. `Lexer::rest_of_line_blank` ran at every space and scanned to the
//! end of the run, so `k` spaces cost `k²/2` byte compares: the render
//! pipeline's float bodies (`floats::isolate_all` blanks the rest of the
//! document, newlines included) made a 100-figure, 78 KB article take 72 s
//! to render. Found by `crates/render-pipeline/examples/fuzz_render.rs`.

use std::time::{Duration, Instant};

use flashtex_tex_expansion::{expand_str, is_group_token, tokens_to_display_string, Token};

fn text(tokens: &[Token]) -> String {
    let content: Vec<Token> = tokens.iter().filter(|t| !is_group_token(t)).cloned().collect();
    tokens_to_display_string(&content)
}

#[test]
fn a_long_space_run_before_text_is_linear() {
    let spaces = " ".repeat(400_000);
    let source = format!("a{spaces}b");
    let started = Instant::now();
    let result = expand_str(&source);
    let elapsed = started.elapsed();
    assert_eq!(text(&result.tokens), "a b");
    // Linear is milliseconds even in a debug build; the quadratic scan was
    // 8e10 byte compares (minutes).
    assert!(elapsed < Duration::from_secs(5), "400k spaces took {elapsed:?}");
}

#[test]
fn space_runs_keep_their_meaning() {
    // Trailing spaces are still stripped before the end of the line,
    // however long the run, and interior runs still make one space.
    let spaces = " ".repeat(5_000);
    assert_eq!(text(&expand_str(&format!("a{spaces}\nb")).tokens), "a b");
    assert_eq!(text(&expand_str(&format!("\\endlinechar=-1 a{spaces}\nb")).tokens), "ab");
    assert_eq!(text(&expand_str(&format!("a{spaces}b{spaces}c{spaces}\n\nd")).tokens), "a b c \\par d");
    // An active space (`\obeyspaces`) is one token per space mid-line, but
    // trailing active spaces are stripped too.
    assert_eq!(
        text(&expand_str("\\catcode`\\ =13 \\def {.}%\na   b   \nc").tokens),
        "a...b c"
    );
}
