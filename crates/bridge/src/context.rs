//! Bounded lexical context, never a TeX interpreter or filesystem loader.
//! Includes resolve only against snapshots already supplied to the bridge.
use crate::{caret, BridgeError, Context, ContextDependency, Document, Result, MAX_CONTEXT_BYTES};

/// Upper bound on `supported_features` entries sent to a provider.
pub const MAX_SUPPORTED_FEATURES: usize = 1024;
/// Upper bound on the summed byte length of `supported_features`.
pub const MAX_SUPPORTED_FEATURE_BYTES: usize = 16 * 1024;
use std::collections::{BTreeMap, BTreeSet};

const SCAN_BYTES: usize = 256 * 1024;
const PROJECT_SCAN_BYTES: usize = 1024 * 1024;
const MAX_FILES: usize = 128;
const NOTICE: &str = "% Lexical snapshot excerpts, not evaluated TeX scope. Dynamic input, conditionals and catcode changes are not resolved; missing or oversized source may be omitted.";

/// Source windows and complete supported declarations share the 16 KiB budget.
/// Candidates are restricted to the target's literal include-connected component.
pub fn build<'a>(
    doc: &Document,
    start: usize,
    end: usize,
    snapshots: impl Iterator<Item = &'a Document>,
    supported_features: Vec<String>,
) -> Result<Context> {
    crate::range(&doc.text, start, end)?;
    // The feature list is compiler-derived (`features::supported_features`, well
    // over 100 entries today), so bound it by count and total bytes rather than
    // the former 64 entries, which rejected every real `capture_convert`.
    if end - start > MAX_CONTEXT_BYTES / 2
        || supported_features.len() > MAX_SUPPORTED_FEATURES
        || supported_features.iter().any(|s| s.len() > 128)
        || supported_features.iter().map(String::len).sum::<usize>() > MAX_SUPPORTED_FEATURE_BYTES
    {
        return Err(BridgeError::new(
            "context_too_large",
            "Selection or supported-feature list exceeds limits",
        ));
    }
    let mut omitted = false;
    let mut files = BTreeMap::new();
    files.insert(doc.path.clone(), scan(doc, SCAN_BYTES));
    let mut scanned = doc.text.len().min(SCAN_BYTES);
    for other in snapshots.filter(|d| d.project_id == doc.project_id && d.path != doc.path) {
        if files.len() == MAX_FILES || scanned >= PROJECT_SCAN_BYTES {
            omitted = true;
            break;
        }
        let limit = SCAN_BYTES.min(PROJECT_SCAN_BYTES - scanned);
        files.insert(other.path.clone(), scan(other, limit));
        scanned += other.text.len().min(limit);
    }
    let mut connected = BTreeSet::from([doc.path.clone()]);
    loop {
        let previous = connected.len();
        for (path, file) in &files {
            for input in &file.inputs {
                // TeX's working directory is typically the project root. Prefer
                // that exact uploaded path; relative-to-includer is a fallback.
                let resolved = resolve(path, input, &files);
                if let Some(child) = resolved {
                    if connected.contains(path) || connected.contains(&child) {
                        connected.insert(path.clone());
                        connected.insert(child);
                    }
                }
            }
        }
        if connected.len() == previous {
            break;
        }
    }
    // Reserve room for preamble declarations even with a large selection.
    let window = ((MAX_CONTEXT_BYTES - (end - start) - 4096) / 2).min(4096);
    let mut before = start.saturating_sub(window);
    while !doc.text.is_char_boundary(before) {
        before += 1;
    }
    let mut after = (end + window).min(doc.text.len());
    while !doc.text.is_char_boundary(after) {
        after -= 1;
    }
    let mut remaining = MAX_CONTEXT_BYTES - (after - before);
    omitted |= files.values().any(|f| f.incomplete);
    let mut definitions = vec![NOTICE.to_owned()];
    // Keep a fixed reserve for an explicit incomplete-context notice.
    remaining -= NOTICE.len() + 128;
    // Target first, then deterministic path order; never silently clip a body.
    for path in std::iter::once(&doc.path).chain(connected.iter().filter(|p| *p != &doc.path)) {
        for declaration in &files[path].declarations {
            if declaration.len() <= remaining {
                definitions.push(declaration.clone());
                remaining -= declaration.len();
            } else {
                omitted = true;
            }
        }
    }
    if omitted {
        definitions[0]
            .push_str(" Context incomplete: scan/file/declaration budget omitted source.");
    }
    Ok(Context {
        // Derived from the whole snapshot, not the excerpt: a `$` thousands of
        // bytes above the window still decides whether the caret is in math.
        caret_context: caret::derive(&doc.text, start),
        project_id: doc.project_id.clone(),
        path: doc.path.clone(),
        revision: doc.revision,
        source_before: doc.text[before..start].into(),
        selected_source: doc.text[start..end].into(),
        source_after: doc.text[end..after].into(),
        definitions,
        dependencies: connected
            .iter()
            .map(|path| files[path].dependency.clone())
            .collect(),
        supported_features,
    })
}
struct Scanned {
    dependency: ContextDependency,
    incomplete: bool,
    inputs: Vec<String>,
    declarations: Vec<String>,
}
fn resolve(parent: &str, input: &str, files: &BTreeMap<String, Scanned>) -> Option<String> {
    if crate::relative_path(input).is_err() {
        return None;
    }
    let name = if input.ends_with(".tex") {
        input.to_owned()
    } else {
        format!("{input}.tex")
    };
    if files.contains_key(&name) {
        return Some(name);
    }
    let dir = parent.rsplit_once('/')?.0;
    let name = format!("{dir}/{name}");
    files.contains_key(&name).then_some(name)
}
fn skip_space(text: &[u8], mut i: usize) -> usize {
    loop {
        while i < text.len() && text[i].is_ascii_whitespace() {
            i += 1;
        }
        if text.get(i) == Some(&b'%') {
            while i < text.len() && text[i] != b'\n' {
                i += 1;
            }
        } else {
            return i;
        }
    }
}
fn group(text: &[u8], i: usize, open: u8, close: u8) -> Option<usize> {
    if text.get(i) != Some(&open) {
        return None;
    }
    let mut depth = 1usize;
    let mut j = i + 1;
    while j < text.len() {
        match text[j] {
            b'\\' => {
                j += 1;
                if j < text.len() {
                    j += 1;
                }
                continue;
            }
            b'%' => {
                while j < text.len() && text[j] != b'\n' {
                    j += 1;
                }
                continue;
            }
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(j + 1);
                }
            }
            _ => (),
        }
        j += 1;
    }
    None
}
fn command(text: &[u8], i: usize) -> usize {
    let mut end = i + 1;
    if text.get(end).is_some_and(u8::is_ascii_alphabetic) {
        while text
            .get(end)
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'@')
        {
            end += 1;
        }
    } else if end < text.len() {
        end += 1;
        while text.get(end).is_some_and(|c| c & 0xc0 == 0x80) {
            end += 1;
        }
    }
    end
}
fn scan(doc: &Document, limit: usize) -> Scanned {
    let mut bound = doc.text.len().min(limit);
    while !doc.text.is_char_boundary(bound) {
        bound -= 1;
    }
    let source = &doc.text[..bound];
    let text = source.as_bytes();
    let mut result = Scanned {
        dependency: ContextDependency {
            path: doc.path.clone(),
            revision: doc.revision,
            source_sha256: crate::digest(doc.text.as_bytes()),
        },
        incomplete: bound < doc.text.len(),
        inputs: vec![],
        declarations: vec![],
    };
    let mut i = 0;
    while i < text.len() && result.declarations.len() < 256 && result.inputs.len() < 256 {
        if text[i] == b'%' {
            while i < text.len() && text[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if text[i] != b'\\' {
            i += 1;
            continue;
        }
        let finish = command(text, i);
        let name = &source[i + 1..finish];
        let mut j = skip_space(text, finish);
        if name == "verb" {
            if text.get(j) == Some(&b'*') {
                j += 1;
            }
            if let Some(delimiter) = text.get(j) {
                j += 1;
                while j < text.len() && text[j] != *delimiter && text[j] != b'\n' {
                    j += 1;
                }
                i = (j + 1).min(text.len());
                continue;
            }
        }
        if name == "begin" {
            if let Some(end) = group(text, j, b'{', b'}') {
                let env = &source[j + 1..end - 1];
                if ["verbatim", "verbatim*", "lstlisting", "minted", "comment"].contains(&env) {
                    let marker = format!("\\end{{{env}}}");
                    i = source[end..]
                        .find(&marker)
                        .map(|p| end + p + marker.len())
                        .unwrap_or(text.len());
                    continue;
                }
            }
        }
        if ["input", "include"].contains(&name) {
            if let Some(end) = group(text, j, b'{', b'}') {
                result.inputs.push(source[j + 1..end - 1].trim().to_owned());
                i = end;
                continue;
            }
            if name == "input" {
                let begin = j;
                while j < text.len()
                    && !text[j].is_ascii_whitespace()
                    && !b"%\\{}".contains(&text[j])
                {
                    j += 1;
                }
                if j > begin {
                    result.inputs.push(source[begin..j].to_owned());
                }
                i = j.max(finish);
                continue;
            }
        }
        let mut declaration_end = None;
        if [
            "newcommand",
            "renewcommand",
            "providecommand",
            "DeclareRobustCommand",
        ]
        .contains(&name)
        {
            if text.get(j) == Some(&b'*') {
                j = skip_space(text, j + 1);
            }
            let target_end = group(text, j, b'{', b'}')
                .or_else(|| (text.get(j) == Some(&b'\\')).then(|| command(text, j)));
            if let Some(end) = target_end {
                j = skip_space(text, end);
                for _ in 0..2 {
                    if let Some(end) = group(text, j, b'[', b']') {
                        j = skip_space(text, end);
                    }
                }
                declaration_end = group(text, j, b'{', b'}');
            }
        } else if ["def", "gdef", "edef", "xdef"].contains(&name) && text.get(j) == Some(&b'\\') {
            j = command(text, j);
            while j < text.len() && text[j] != b'{' {
                if text[j] == b'%' {
                    j = skip_space(text, j);
                } else {
                    j += 1;
                }
            }
            declaration_end = group(text, j, b'{', b'}');
        } else if [
            "usepackage",
            "RequirePackage",
            "documentclass",
            "DeclareMathOperator",
        ]
        .contains(&name)
        {
            if text.get(j) == Some(&b'*') {
                j = skip_space(text, j + 1);
            }
            if let Some(end) = group(text, j, b'[', b']') {
                j = skip_space(text, end);
            }
            declaration_end = group(text, j, b'{', b'}');
            if name == "DeclareMathOperator" {
                declaration_end =
                    declaration_end.and_then(|end| group(text, skip_space(text, end), b'{', b'}'));
            }
        }
        if let Some(end) = declaration_end {
            let line = source[..i].bytes().filter(|b| *b == b'\n').count() + 1;
            result.declarations.push(format!(
                "% snapshot {} revision {} line {} (lexical excerpt)\n{}",
                doc.path,
                doc.revision,
                line,
                &source[i..end]
            ));
            i = end;
        } else {
            i = finish;
        }
    }
    result.incomplete |= i < text.len();
    result
}
