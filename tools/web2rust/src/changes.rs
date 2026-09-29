//! WEB change files (`@x` ... `@y` ... `@z`), applied exactly as Knuth's
//! TANGLE applies them (`tangle.web` §§121-137: `prime_the_change_buffer`,
//! `check_change`, `get_line`).
//!
//! A change file is a sequence of changes. Everything outside a change is a
//! comment. In each change, the lines between `@x` and `@y` must equal
//! consecutive lines of the master file (trailing blanks ignored); they are
//! replaced by the lines between `@y` and `@z`. Blank lines right after `@x`
//! are skipped; after that every line counts. Changes apply in order, each one
//! searching forward from where the previous one ended, and every change must
//! match. The control codes may be upper case. Everything after `@x`, `@y`
//! or `@z` on its line is ignored.
//!
//! TANGLE only reports most of these problems and carries on; here each one
//! is fatal, because an engine translated from a partly applied change file
//! is never what anybody wants.
//!
//! Several change files are applied one after another, each to the result of
//! the previous one. That is how `tie -c` combines web2c's change files for
//! pdfTeX (`pdftexdir/change-files.txt`), and with a single change file it is
//! exactly TANGLE.
//!
//! Section numbers are those of the merged text, as in TANGLE: a replacement
//! that starts a new section renumbers every later one.

/// A line as TANGLE's `input_ln` sees it: trailing blanks removed.
fn trimmed(l: &str) -> &str {
    l.trim_end_matches([' ', '\t', '\r'])
}

/// `@x`, `@y` or `@z` (in either case) at the start of a line.
fn control(l: &str) -> Option<char> {
    let b = l.as_bytes();
    if b.len() >= 2 && b[0] == b'@' {
        match b[1].to_ascii_lowercase() {
            c @ (b'x' | b'y' | b'z') => Some(c as char),
            _ => None,
        }
    } else {
        None
    }
}

struct Change<'a> {
    name: &'a str,
    lines: Vec<&'a str>,
    /// Index of the next line to read.
    pos: usize,
}

impl<'a> Change<'a> {
    fn next(&mut self) -> Option<&'a str> {
        let l = self.lines.get(self.pos).map(|l| trimmed(l));
        if l.is_some() {
            self.pos += 1;
        }
        l
    }
    /// 1-based line number of the line read last, for messages.
    fn line(&self) -> usize {
        self.pos
    }
    fn err(&self, msg: &str) -> String {
        format!("{} l.{}: {msg}", self.name, self.line())
    }
    /// `prime_the_change_buffer`: the first line of the next change's `@x`
    /// part, or `None` when there are no more changes.
    fn prime(&mut self) -> Result<Option<&'a str>, String> {
        loop {
            let Some(l) = self.next() else {
                return Ok(None);
            };
            match control(l) {
                Some('x') => break,
                Some(_) => return Err(self.err("Where is the matching @x?")),
                None => {}
            }
        }
        loop {
            match self.next() {
                None => return Err(self.err("Change file ended after @x")),
                Some("") => continue,
                Some(l) => return Ok(Some(l)),
            }
        }
    }
}

/// Apply one change file to `master`. `name` is only used in messages.
pub fn apply(master: &str, change: &str, name: &str) -> Result<String, String> {
    let mut ch = Change {
        name,
        lines: change.split('\n').collect(),
        pos: 0,
    };
    // A final newline does not start another line.
    if ch.lines.last() == Some(&"") {
        ch.lines.pop();
    }
    let mut web = master.split('\n').peekable();
    let mut web_line = 0usize;
    let mut out: Vec<&str> = Vec::new();
    let mut pending = ch.prime()?;
    let mut changes = 0usize;
    while let Some(raw) = web.next() {
        web_line += 1;
        // The final newline of the master does not start a line either.
        if web.peek().is_none() && raw.is_empty() {
            break;
        }
        if pending != Some(trimmed(raw)) {
            out.push(raw);
            continue;
        }
        // `check_change`: the rest of the `@x` part must match line by line.
        let start = web_line;
        let mut mismatches = 0usize;
        loop {
            let Some(cl) = ch.next() else {
                return Err(ch.err("Change file ended before @y"));
            };
            match control(cl) {
                Some('y') => break,
                Some(_) => return Err(ch.err("Where is the matching @y?")),
                None => {}
            }
            let Some(wl) = web.next() else {
                return Err(ch.err("WEB file ended during a change"));
            };
            web_line += 1;
            if trimmed(wl) != cl {
                mismatches += 1;
            }
        }
        if mismatches > 0 {
            return Err(ch.err(&format!(
                "Hmm... {mismatches} of the preceding lines failed to match \
                 (the change begins at line {start} of the master)"
            )));
        }
        // The replacement text, up to `@z`.
        loop {
            let Some(cl) = ch.next() else {
                return Err(ch.err("Change file ended without @z"));
            };
            match control(cl) {
                Some('z') => break,
                Some(_) => return Err(ch.err("Where is the matching @z?")),
                None => out.push(cl),
            }
        }
        changes += 1;
        pending = ch.prime()?;
    }
    if let Some(l) = pending {
        return Err(format!(
            "{name}: change {} did not match (its first line is {l:?})",
            changes + 1
        ));
    }
    let mut s = out.join("\n");
    s.push('\n');
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::apply;

    const WEB: &str = "a\nb\nc\nd\n";

    #[test]
    fn replaces_deletes_and_skips_comments() {
        let ch = "comment\n@x\n\nb\nc\n@y\nB\n@z\nmore\n@X\nd\n@Y\n@Z\n";
        assert_eq!(apply(WEB, ch, "t").unwrap(), "a\nB\n");
    }

    #[test]
    fn trailing_blanks_do_not_matter() {
        let ch = "@x\nb  \n@y\nB\n@z\n";
        assert_eq!(apply("a\nb\t\n", ch, "t").unwrap(), "a\nB\n");
    }

    #[test]
    fn changes_must_be_in_order() {
        let ch = "@x\nc\n@y\n@z\n@x\nb\n@y\n@z\n";
        assert!(apply(WEB, ch, "t").is_err());
    }

    #[test]
    fn partial_match_is_an_error() {
        let ch = "@x\nb\nd\n@y\n@z\n";
        let e = apply(WEB, ch, "t").unwrap_err();
        assert!(
            e.contains("1 of the preceding lines failed to match"),
            "{e}"
        );
    }

    #[test]
    fn misplaced_control_codes() {
        assert!(apply(WEB, "@y\n", "t").is_err());
        assert!(apply(WEB, "@x\nb\n@z\n", "t").is_err());
        assert!(apply(WEB, "@x\nb\n@y\n@x\n", "t").is_err());
        assert!(apply(WEB, "@x\nb\n@y\nB\n", "t").is_err());
        assert!(apply(WEB, "@x\n\n", "t").is_err());
    }
}
