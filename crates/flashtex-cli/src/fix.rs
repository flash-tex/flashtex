//! `check --fix`: apply diagnostic suggestions as exact byte-range edits.
//!
//! Edits are grouped per project file, overlapping ranges are skipped,
//! a SHA-256 of the compiled snapshot is compared to the on-disk bytes
//! just before write, and nothing outside the project root or behind a
//! symlink is touched. Writes go through a sibling temp file + rename,
//! preserving the original mode.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::compile::Diagnostic;
use crate::project::Project;

/// One replacement of `start..end` in a project-relative path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub path: String,
    pub start: usize,
    pub end: usize,
    pub replacement: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skip {
    Overlap { path: String, line: Option<usize>, column: Option<usize> },
    Changed { path: String },
    Symlink { path: String },
    OutsideRoot { path: String },
    Unreadable { path: String, detail: String },
}

impl Skip {
    pub fn line_text(&self) -> String {
        match self {
            Skip::Overlap { path, line, column } => {
                let at = match (line, column) {
                    (Some(l), Some(c)) => format!("{path}:{l}:{c}"),
                    _ => path.clone(),
                };
                format!("flashtex: skipped overlapping edit in {at}")
            }
            Skip::Changed { path } => format!("flashtex: skipped {path}: file changed since compile"),
            Skip::Symlink { path } => format!("flashtex: skipped {path}: refusing to write through a symlink"),
            Skip::OutsideRoot { path } => format!("flashtex: skipped {path}: outside the project root"),
            Skip::Unreadable { path, detail } => format!("flashtex: skipped {path}: {detail}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FilePlan {
    pub path: String,
    pub original: String,
    pub digest: [u8; 32],
    pub edits: Vec<Edit>,
}

#[derive(Debug)]
pub struct Plan {
    pub files: Vec<FilePlan>,
    pub skipped: Vec<Skip>,
}

impl Plan {
    #[cfg(test)]
    fn issue_count(&self) -> usize {
        self.files.iter().map(|f| f.edits.len()).sum()
    }
}

/// Edits that have both a suggestion and a source span in a project document.
pub fn collect_edits(diagnostics: &[Diagnostic], project: &Project) -> Vec<Edit> {
    diagnostics
        .iter()
        .filter_map(|d| {
            let replacement = d.suggestion.clone()?;
            // A suggestion replaces its span; without both ends it would turn
            // into an insertion at `start`, which is not what the compiler offered.
            let start = d.start_byte?;
            let end = d.end_byte?;
            if end <= start {
                return None;
            }
            if !project.documents.iter().any(|doc| doc.path == d.path) {
                return None;
            }
            Some(Edit {
                path: d.path.clone(),
                start,
                end,
                replacement,
                line: d.line,
                column: d.column,
            })
        })
        .collect()
}

/// Group by file, drop overlapping ranges, snapshot the compiled bytes.
pub fn plan(edits: Vec<Edit>, project: &Project) -> Plan {
    let mut by_path: BTreeMap<String, Vec<Edit>> = BTreeMap::new();
    for e in edits {
        by_path.entry(e.path.clone()).or_default().push(e);
    }
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    for (path, mut group) in by_path {
        group.sort_by_key(|e| (e.start, e.end));
        let n = group.len();
        let mut drop = vec![false; n];
        for i in 0..n {
            for j in i + 1..n {
                if group[j].start >= group[i].end {
                    break;
                }
                drop[i] = true;
                drop[j] = true;
            }
        }
        let mut kept = Vec::new();
        for (e, skip) in group.into_iter().zip(drop) {
            if skip {
                skipped.push(Skip::Overlap { path: e.path.clone(), line: e.line, column: e.column });
            } else {
                kept.push(e);
            }
        }
        if kept.is_empty() {
            continue;
        }
        let original = project
            .documents
            .iter()
            .find(|d| d.path == path)
            .map(|d| d.text.clone())
            .unwrap_or_default();
        let digest = flashtex_project_files::sha256(original.as_bytes());
        files.push(FilePlan { path, original, digest, edits: kept });
    }
    Plan { files, skipped }
}

#[derive(Debug)]
pub struct Applied {
    pub issues: usize,
    pub files: usize,
    pub skipped: Vec<Skip>,
    pub diffs: Vec<String>,
}

/// Apply (or dry-run) a plan. `dry_run` prints unified diffs and writes nothing.
pub fn apply(project: &Project, plan: Plan, dry_run: bool) -> Applied {
    let mut skipped = plan.skipped;
    let mut issues = 0;
    let mut files = 0;
    let mut diffs = Vec::new();
    for file in plan.files {
        match apply_one(project, file, dry_run) {
            Ok((n, diff)) => {
                issues += n;
                if n > 0 {
                    files += 1;
                }
                if let Some(d) = diff {
                    diffs.push(d);
                }
            }
            Err(s) => skipped.push(s),
        }
    }
    Applied { issues, files, skipped, diffs }
}

fn apply_one(project: &Project, file: FilePlan, dry_run: bool) -> Result<(usize, Option<String>), Skip> {
    let dest = resolve_writable(project, &file.path)?;
    let on_disk = fs::read(&dest).map_err(|e| Skip::Unreadable { path: file.path.clone(), detail: e.to_string() })?;
    if flashtex_project_files::sha256(&on_disk) != file.digest {
        return Err(Skip::Changed { path: file.path.clone() });
    }
    let rewritten = apply_edits_back_to_front(&file.original, &file.edits)
        .ok_or_else(|| Skip::Unreadable { path: file.path.clone(), detail: "edit range is out of bounds".into() })?;
    let n = file.edits.len();
    let diff = unified_diff(&file.path, &file.original, &rewritten);
    if dry_run {
        return Ok((n, Some(diff)));
    }
    write_atomic_preserving_mode(&dest, rewritten.as_bytes()).map_err(|detail| Skip::Unreadable { path: file.path.clone(), detail })?;
    Ok((n, Some(diff)))
}

fn apply_edits_back_to_front(original: &str, edits: &[Edit]) -> Option<String> {
    let mut bytes = original.as_bytes().to_vec();
    let mut ordered = edits.to_vec();
    ordered.sort_by_key(|e| std::cmp::Reverse(e.start));
    for e in &ordered {
        if e.end < e.start || e.end > original.len() {
            return None;
        }
        if !original.is_char_boundary(e.start) || !original.is_char_boundary(e.end) {
            return None;
        }
        bytes.splice(e.start..e.end, e.replacement.as_bytes().iter().copied());
    }
    String::from_utf8(bytes).ok()
}

fn resolve_writable(project: &Project, rel: &str) -> Result<PathBuf, Skip> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() || rel.split('/').any(|c| c == ".." || c.is_empty()) {
        return Err(Skip::OutsideRoot { path: rel.to_string() });
    }
    let mut cur = project.root.clone();
    let mut leaf_meta = None;
    for component in rel_path.components() {
        match component {
            std::path::Component::Normal(name) => cur.push(name),
            std::path::Component::CurDir => continue,
            _ => return Err(Skip::OutsideRoot { path: rel.to_string() }),
        }
        let meta = fs::symlink_metadata(&cur).map_err(|e| Skip::Unreadable { path: rel.to_string(), detail: e.to_string() })?;
        if meta.file_type().is_symlink() {
            return Err(Skip::Symlink { path: rel.to_string() });
        }
        leaf_meta = Some(meta);
    }
    let meta = leaf_meta.ok_or_else(|| Skip::OutsideRoot { path: rel.to_string() })?;
    if !meta.is_file() {
        return Err(Skip::Unreadable { path: rel.to_string(), detail: "not a regular file".into() });
    }
    Ok(cur)
}

fn write_atomic_preserving_mode(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mode = fs::metadata(path).ok().map(|m| m.permissions());
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("out");
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    fs::write(&tmp, bytes).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    if let Some(mode) = mode {
        let _ = fs::set_permissions(&tmp, mode);
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("cannot write {}: {e}", path.display())
    })
}

/// A single hunk covering the first differing region, with 3 lines of context.
pub fn unified_diff(path: &str, before: &str, after: &str) -> String {
    let a: Vec<&str> = before.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    let mut start = 0;
    while start < a.len() && start < b.len() && a[start] == b[start] {
        start += 1;
    }
    let mut end_a = a.len();
    let mut end_b = b.len();
    while end_a > start && end_b > start && a[end_a - 1] == b[end_b - 1] {
        end_a -= 1;
        end_b -= 1;
    }
    const CTX: usize = 3;
    let hunk_a0 = start.saturating_sub(CTX);
    let hunk_b0 = start.saturating_sub(CTX);
    let hunk_a1 = (end_a + CTX).min(a.len());
    let hunk_b1 = (end_b + CTX).min(b.len());
    let mut out = format!(
        "--- {path}\n+++ {path}\n@@ -{},{} +{},{} @@\n",
        hunk_a0 + 1,
        hunk_a1.saturating_sub(hunk_a0),
        hunk_b0 + 1,
        hunk_b1.saturating_sub(hunk_b0)
    );
    for line in &a[hunk_a0..start] {
        out.push(' ');
        out.push_str(line);
        out.push('\n');
    }
    for line in &a[start..end_a] {
        out.push('-');
        out.push_str(line);
        out.push('\n');
    }
    for line in &b[start..end_b] {
        out.push('+');
        out.push_str(line);
        out.push('\n');
    }
    for line in &a[end_a..hunk_a1] {
        out.push(' ');
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub fn summary_line(issues: usize, files: usize, skipped: usize) -> String {
    format!("flashtex: fixed {issues} issue(s) in {files} file(s); {skipped} skipped")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::Diagnostic;
    use crate::project::{Document, Project};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    fn diag_at(path: &str, text: &str, needle: &str, replacement: &str) -> Diagnostic {
        let start = text.find(needle).expect("needle");
        let (line, column) = crate::compile::line_col(text, start);
        Diagnostic {
            path: path.into(),
            line: Some(line),
            column: Some(column),
            start_byte: Some(start),
            end_byte: Some(start + needle.len()),
            error: true,
            code: "unknown_command".into(),
            message: format!("{needle} is not supported"),
            recovery: None,
            suggestion: Some(replacement.into()),
        }
    }

    fn project_at(root: PathBuf, docs: Vec<(&str, &str)>) -> Project {
        Project {
            root,
            entry: docs[0].0.to_string(),
            documents: docs
                .iter()
                .map(|(p, t)| Document { path: (*p).into(), text: (*t).into() })
                .collect(),
            diagnostics: Vec::new(),
            files: docs.iter().map(|(p, _)| (*p).to_string()).collect(),
        }
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("flashtex-cli-fix-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn names_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn overlapping_edits_are_skipped_and_the_rest_apply() {
        let text = "aaaa bbbb cccc\n";
        let d1 = diag_at("main.tex", text, "aaaa", "AAAA");
        let mut d2 = diag_at("main.tex", text, "aaaa", "XXXX");
        d2.end_byte = Some(d1.start_byte.unwrap() + 2);
        let d3 = diag_at("main.tex", text, "cccc", "CCCC");
        let dir = tmp("overlap");
        fs::write(dir.join("main.tex"), text).unwrap();
        let project = project_at(dir.clone(), vec![("main.tex", text)]);
        let plan = plan(collect_edits(&[d1, d2, d3], &project), &project);
        assert_eq!(plan.skipped.len(), 2, "{:?}", plan.skipped);
        assert!(plan.skipped.iter().all(|s| matches!(s, Skip::Overlap { .. })));
        assert_eq!(plan.issue_count(), 1);
        let applied = apply(&project, plan, false);
        assert_eq!(applied.issues, 1);
        assert_eq!(applied.files, 1);
        assert_eq!(applied.skipped.len(), 2);
        assert_eq!(fs::read_to_string(dir.join("main.tex")).unwrap(), "aaaa bbbb CCCC\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_changed_after_compile_is_refused() {
        let text = "Hello \\alpah world.\n";
        let d = diag_at("main.tex", text, "\\alpah", "\\alpha");
        let dir = tmp("changed");
        fs::write(dir.join("main.tex"), text).unwrap();
        let project = project_at(dir.clone(), vec![("main.tex", text)]);
        let plan = plan(collect_edits(&[d], &project), &project);
        fs::write(dir.join("main.tex"), "Hello \\alpah CHANGED world.\n").unwrap();
        let applied = apply(&project, plan, false);
        assert_eq!(applied.issues, 0);
        assert_eq!(applied.files, 0);
        assert!(applied.skipped.iter().any(|s| matches!(s, Skip::Changed { .. })), "{:?}", applied.skipped);
        assert_eq!(fs::read_to_string(dir.join("main.tex")).unwrap(), "Hello \\alpah CHANGED world.\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_symlink_target_is_refused() {
        let text = "Hello \\alpah world.\n";
        let d = diag_at("main.tex", text, "\\alpah", "\\alpha");
        let dir = tmp("symlink");
        fs::write(dir.join("real.tex"), text).unwrap();
        std::os::unix::fs::symlink(dir.join("real.tex"), dir.join("main.tex")).unwrap();
        let project = project_at(dir.clone(), vec![("main.tex", text)]);
        let plan = plan(collect_edits(&[d], &project), &project);
        let applied = apply(&project, plan, false);
        assert!(applied.skipped.iter().any(|s| matches!(s, Skip::Symlink { .. })), "{:?}", applied.skipped);
        assert_eq!(fs::read_to_string(dir.join("real.tex")).unwrap(), text);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_symlinked_parent_directory_is_refused() {
        let text = "Hello \\alpah world.\n";
        let d = diag_at("link/main.tex", text, "\\alpah", "\\alpha");
        let dir = tmp("symlink-parent");
        fs::create_dir(dir.join("src")).unwrap();
        let target = dir.join("src").join("main.tex");
        fs::write(&target, text).unwrap();
        std::os::unix::fs::symlink(dir.join("src"), dir.join("link")).unwrap();
        let project = project_at(dir.clone(), vec![("link/main.tex", text)]);
        let plan = plan(collect_edits(&[d], &project), &project);
        let applied = apply(&project, plan, false);
        assert_eq!(applied.issues, 0, "{:?}", applied.skipped);
        assert!(applied.skipped.iter().any(|s| matches!(s, Skip::Symlink { .. })), "{:?}", applied.skipped);
        assert_eq!(fs::read(&target).unwrap(), text.as_bytes());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_parent_escape_and_an_absolute_path_are_refused() {
        let text = "Hello \\alpah world.\n";
        let dir = tmp("escape");
        fs::write(dir.join("main.tex"), text).unwrap();
        let outside = std::env::temp_dir().join(format!("flashtex-cli-fix-{}-outside.tex", std::process::id()));
        fs::write(&outside, text).unwrap();
        let rel = format!("../{}", outside.file_name().unwrap().to_str().unwrap());
        let abs = outside.to_str().unwrap().to_string();
        let d_rel = diag_at(&rel, text, "\\alpah", "\\alpha");
        let d_abs = diag_at(&abs, text, "\\alpah", "\\alpha");
        let project = project_at(dir.clone(), vec![("main.tex", text), (rel.as_str(), text), (abs.as_str(), text)]);
        let plan = plan(collect_edits(&[d_rel, d_abs], &project), &project);
        let applied = apply(&project, plan, false);
        assert_eq!(applied.issues, 0);
        assert_eq!(applied.skipped.len(), 2, "{:?}", applied.skipped);
        assert!(applied.skipped.iter().all(|s| matches!(s, Skip::OutsideRoot { .. })), "{:?}", applied.skipped);
        assert_eq!(fs::read(&outside).unwrap(), text.as_bytes());
        assert_eq!(fs::read(dir.join("main.tex")).unwrap(), text.as_bytes());
        let _ = fs::remove_file(&outside);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn crlf_bytes_are_preserved_outside_the_edited_span() {
        let text = "Hello \\alpah world.\r\nNext line.\r\n";
        let d = diag_at("main.tex", text, "\\alpah", "\\alpha");
        let dir = tmp("crlf");
        fs::write(dir.join("main.tex"), text.as_bytes()).unwrap();
        let project = project_at(dir.clone(), vec![("main.tex", text)]);
        let plan = plan(collect_edits(&[d], &project), &project);
        let applied = apply(&project, plan, false);
        assert_eq!(applied.issues, 1);
        assert_eq!(fs::read(dir.join("main.tex")).unwrap(), b"Hello \\alpha world.\r\nNext line.\r\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_span_on_a_utf8_multibyte_boundary_is_refused() {
        // Two U+00E9 (c3 a9). start=1,end=3 with an empty replacement splices
        // out a9 c3 and leaves a coincidentally valid é; from_utf8 would accept
        // the result, so the refusal has to be a char-boundary check.
        let text = "éé \\alpah\n";
        assert_eq!(text.as_bytes()[..4], [0xc3, 0xa9, 0xc3, 0xa9]);
        let mut d = diag_at("main.tex", text, "\\alpah", "\\alpha");
        d.start_byte = Some(1);
        d.end_byte = Some(3);
        d.suggestion = Some(String::new());
        let dir = tmp("utf8-boundary");
        fs::write(dir.join("main.tex"), text.as_bytes()).unwrap();
        let project = project_at(dir.clone(), vec![("main.tex", text)]);
        let plan = plan(collect_edits(&[d], &project), &project);
        let applied = apply(&project, plan, false);
        assert_eq!(applied.issues, 0, "{:?}", applied.skipped);
        assert!(!applied.skipped.is_empty(), "{:?}", applied.skipped);
        assert_eq!(fs::read(dir.join("main.tex")).unwrap(), text.as_bytes());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn non_overlapping_edits_apply_back_to_front() {
        let text = "AA BB CC\n";
        let d1 = diag_at("main.tex", text, "AA", "AAAA");
        let d2 = diag_at("main.tex", text, "CC", "C");
        let dir = tmp("back-to-front");
        fs::write(dir.join("main.tex"), text).unwrap();
        let project = project_at(dir.clone(), vec![("main.tex", text)]);
        let plan = plan(collect_edits(&[d1, d2], &project), &project);
        assert_eq!(plan.issue_count(), 2);
        let applied = apply(&project, plan, false);
        assert_eq!(applied.issues, 2);
        assert_eq!(fs::read(dir.join("main.tex")).unwrap(), b"AAAA BB C\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dry_run_creates_no_temp_files() {
        let text = "Hello \\alpah world.\n";
        let d = diag_at("main.tex", text, "\\alpah", "\\alpha");
        let dir = tmp("dry-list");
        fs::write(dir.join("main.tex"), text).unwrap();
        let before = names_in(&dir);
        let project = project_at(dir.clone(), vec![("main.tex", text)]);
        let plan = plan(collect_edits(&[d], &project), &project);
        let applied = apply(&project, plan, true);
        assert_eq!(applied.issues, 1);
        let after = names_in(&dir);
        assert_eq!(after, before);
        assert!(!after.iter().any(|n| n.ends_with(".tmp") || n.contains(".tmp")));
        assert_eq!(fs::read(dir.join("main.tex")).unwrap(), text.as_bytes());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dry_run_writes_nothing_and_builds_a_unified_diff() {
        let text = "Hello \\alpah world.\n";
        let d = diag_at("main.tex", text, "\\alpah", "\\alpha");
        let dir = tmp("dry");
        fs::write(dir.join("main.tex"), text).unwrap();
        let project = project_at(dir.clone(), vec![("main.tex", text)]);
        let plan = plan(collect_edits(&[d], &project), &project);
        let applied = apply(&project, plan, true);
        assert_eq!(applied.issues, 1);
        assert_eq!(fs::read_to_string(dir.join("main.tex")).unwrap(), text);
        let diff = applied.diffs.join("");
        assert!(diff.contains("--- main.tex") && diff.contains("+++ main.tex"), "{diff}");
        assert!(diff.contains("-Hello \\alpah world."), "{diff}");
        assert!(diff.contains("+Hello \\alpha world."), "{diff}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write_preserves_mode() {
        let text = "Hello \\alpah world.\n";
        let d = diag_at("main.tex", text, "\\alpah", "\\alpha");
        let dir = tmp("mode");
        let path = dir.join("main.tex");
        fs::write(&path, text).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        let project = project_at(dir.clone(), vec![("main.tex", text)]);
        let plan = plan(collect_edits(&[d], &project), &project);
        let applied = apply(&project, plan, false);
        assert_eq!(applied.issues, 1);
        assert_eq!(fs::read_to_string(&path).unwrap(), "Hello \\alpha world.\n");
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640, "mode {mode:o}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_suggestion_omits_an_edit() {
        let mut d = diag_at("main.tex", "x\n", "x", "y");
        d.suggestion = None;
        let dir = tmp("none");
        let project = project_at(dir.clone(), vec![("main.tex", "x\n")]);
        assert!(collect_edits(&[d], &project).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_suggestion_without_a_full_span_is_not_turned_into_an_insertion() {
        let mut d = diag_at("main.tex", "x\n", "x", "y");
        d.end_byte = None;
        let dir = tmp("no-end");
        let project = project_at(dir.clone(), vec![("main.tex", "x\n")]);
        assert!(collect_edits(&[d.clone()], &project).is_empty());
        d.end_byte = d.start_byte;
        assert!(collect_edits(&[d], &project).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
