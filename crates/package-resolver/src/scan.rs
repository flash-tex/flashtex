//! The package and class names a document asks for: `\usepackage[…]{a,b}`,
//! `\RequirePackage[…]{a}`, `\documentclass[…]{x}`, `\LoadClass[…]{x}` and
//! the `…WithOptions` forms, outside `%` comments. A light scanner in the
//! spirit of project-files' `scan_references`: it does not expand macros,
//! so an argument containing `\` or `#` is skipped (the compiler reports
//! those itself).

/// A `\usepackage`-style reference found in a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRef {
    pub name: String,
    pub kind: RefKind,
    /// Byte offset of the command in the text.
    pub at: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    /// `<name>.sty` (`\usepackage`, `\RequirePackage`).
    Package,
    /// `<name>.cls` (`\documentclass`, `\LoadClass`).
    Class,
}

impl RefKind {
    /// The file the reference resolves to.
    pub fn file_name(self, name: &str) -> String {
        match self {
            RefKind::Package => format!("{name}.sty"),
            RefKind::Class => format!("{name}.cls"),
        }
    }
}

const COMMANDS: &[(&str, RefKind)] = &[
    ("usepackage", RefKind::Package),
    ("RequirePackageWithOptions", RefKind::Package),
    ("RequirePackage", RefKind::Package),
    ("documentclass", RefKind::Class),
    ("LoadClassWithOptions", RefKind::Class),
    ("LoadClass", RefKind::Class),
];

/// Every package/class reference in `text`, in order, duplicates kept.
pub fn package_references(text: &str) -> Vec<PackageRef> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' => {
                // Comment to end of line.
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'\\' => {
                let start = i;
                i += 1;
                let name_start = i;
                while i < b.len() && b[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let name = &text[name_start..i];
                // `\\` or `\%`: a control symbol, skip it whole.
                if name.is_empty() {
                    i += 1;
                    continue;
                }
                let Some(&(_, kind)) = COMMANDS.iter().find(|(c, _)| *c == name) else { continue };
                let mut j = i;
                skip_spaces(b, &mut j);
                if j < b.len() && b[j] == b'[' {
                    // Options may nest braces (`key={a,b}`).
                    let mut depth = 0i32;
                    while j < b.len() {
                        match b[j] {
                            b'{' => depth += 1,
                            b'}' => depth -= 1,
                            b']' if depth <= 0 => break,
                            _ => {}
                        }
                        j += 1;
                    }
                    if j < b.len() {
                        j += 1;
                    }
                    skip_spaces(b, &mut j);
                }
                if j >= b.len() || b[j] != b'{' {
                    continue;
                }
                let arg_start = j + 1;
                let Some(len) = text[arg_start..].find('}') else { break };
                let arg = &text[arg_start..arg_start + len];
                i = arg_start + len + 1;
                if arg.contains(['\\', '#', '{']) {
                    continue;
                }
                for item in arg.split(',') {
                    let item = item.trim();
                    if !item.is_empty() && crate::is_valid_name(item) {
                        out.push(PackageRef { name: item.to_string(), kind, at: start });
                    }
                }
            }
            _ => i += 1,
        }
    }
    out
}

fn skip_spaces(b: &[u8], j: &mut usize) {
    while *j < b.len() && (b[*j] == b' ' || b[*j] == b'\t') {
        *j += 1;
    }
}

/// The package names a compiler diagnostic of the form
/// `packages a, b are recognised but not implemented` names (the
/// `unsupported_feature` diagnostic `\usepackage` produces for every
/// package the engine does not model); empty for any other message.
pub fn unimplemented_packages(message: &str) -> Vec<String> {
    let Some(rest) = message.strip_prefix("packages ") else { return Vec::new() };
    let Some(list) = rest.strip_suffix(" are recognised but not implemented") else { return Vec::new() };
    list.split(',').map(str::trim).filter(|n| crate::is_valid_name(n)).map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_packages_and_classes_outside_comments() {
        let text = "\\documentclass[11pt]{article}\n% \\usepackage{commented}\n\\usepackage[utf8]{inputenc}\n\\usepackage{siunitx, cancel}\n\\usepackage[output-decimal-marker={,}]{siunitx}\n\\RequirePackage{x-y}\\LoadClass{base}\n\\usepackage{\\macro}\n\\newcommand\\usepackagefoo{}\n";
        let refs = package_references(text);
        let names: Vec<(String, RefKind)> = refs.iter().map(|r| (r.name.clone(), r.kind)).collect();
        assert_eq!(
            names,
            [
                ("article".to_string(), RefKind::Class),
                ("inputenc".to_string(), RefKind::Package),
                ("siunitx".to_string(), RefKind::Package),
                ("cancel".to_string(), RefKind::Package),
                ("siunitx".to_string(), RefKind::Package),
                ("x-y".to_string(), RefKind::Package),
                ("base".to_string(), RefKind::Class),
            ]
        );
        assert_eq!(refs[0].at, 0);
        assert_eq!(RefKind::Package.file_name("a"), "a.sty");
        assert_eq!(RefKind::Class.file_name("a"), "a.cls");
        assert!(package_references("\\usepackage{unterminated").is_empty());
        assert!(package_references("\\\\usepackage{x}").is_empty(), "`\\\\` is a control symbol");
    }

    #[test]
    fn reads_the_compilers_unimplemented_list() {
        assert_eq!(unimplemented_packages("packages siunitx, tikz are recognised but not implemented"), ["siunitx", "tikz"]);
        assert_eq!(unimplemented_packages("packages cancel are recognised but not implemented"), ["cancel"]);
        assert!(unimplemented_packages("\\foo is recognised but not implemented").is_empty());
        assert!(unimplemented_packages("packages ../x are recognised but not implemented").is_empty());
    }
}
