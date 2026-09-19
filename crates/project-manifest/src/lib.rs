//! The optional `flashtex.toml` project manifest
//! (docs/proposals/packages-fonts-manifest.md §S2, docs/user/project-manifest.md).
//!
//! A project is the directory holding the entry document; without a
//! manifest that is all FlashTeX knows, and everything behaves exactly as
//! before this crate existed. A manifest next to (or above) the entry adds
//! what the directory cannot say by itself: which file is the entry when a
//! directory is opened, extra `texinputs` directories whose `.sty`/`.cls`/
//! `.tex`/`.bib`/`.def`/`.clo` files join the compiler's document set, where
//! output goes, the fonts and the package-resolution policy for later slices.
//!
//! Design rules, all of them deliberate:
//!
//! - **Absent is default.** [`Manifest::load`] on a missing file is
//!   [`Manifest::default()`], never an error, so no caller has to special-case
//!   "no manifest".
//! - **Unknown is a warning.** A key or section this version does not know
//!   ([`ManifestWarning`], with the dotted key path) is reported and ignored,
//!   never fatal: a manifest written for a newer FlashTeX still builds on an
//!   older one. Only TOML syntax errors are [`ManifestError::Syntax`], because
//!   nothing can be read past them. A known key of the wrong type is also a
//!   warning and takes its default.
//! - **No reads here.** `texinputs` entries are classified lexically
//!   ([`TexInputLocation`]) with the same rules as project-files'
//!   `ProjectPath` (forward slashes, no absolute paths, no `~`, no `\`/`:`/
//!   control characters, `..` pops); the *reader* (CLI, helper) opens each
//!   directory through its rooted, symlink-refusing primitive. This crate
//!   only says where a directory is and whether it is inside the project.

use std::collections::BTreeMap;
use std::fmt;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use toml::{Table, Value};

/// The manifest's file name, always at the project root.
pub const FILE_NAME: &str = "flashtex.toml";

/// Extensions of the files a `texinputs` directory contributes to the
/// project: what `\usepackage`/`\documentclass`/`\input`/`\bibliography`
/// and the class/package kernel (`.def`, `.clo`) can ask for. Anything else
/// in such a directory (a README, a PDF) is not a document.
pub const TEXINPUT_EXTENSIONS: &[&str] = &["sty", "cls", "tex", "bib", "def", "clo"];

/// Whether a file name is one a `texinputs` directory contributes.
pub fn is_texinput_file(name: &str) -> bool {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => TEXINPUT_EXTENSIONS.contains(&ext),
        _ => false,
    }
}

/// Where a `texinputs` directory that lies *outside* the project root is
/// presented inside the project's path space: `texinputs/<index>/<file>`,
/// `index` being the entry's position in `[project] texinputs`. Stable
/// across runs (it depends only on the manifest), never colliding with a
/// real project file unless the project has a directory literally called
/// `texinputs`, in which case that directory's own files still win because
/// the entry closure is listed before any `texinputs` file.
pub const OUTSIDE_PREFIX: &str = "texinputs";

/// `texinputs/<index>`: the virtual project directory an outside
/// `texinputs` entry is mounted at (see [`OUTSIDE_PREFIX`]).
pub fn outside_virtual_dir(index: usize) -> String {
    format!("{OUTSIDE_PREFIX}/{index}")
}

/// `[project]`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Project {
    /// The document to compile, relative to the manifest's directory.
    /// `None`: the file the user opened / named on the command line.
    pub entry: Option<String>,
    /// Extra directories searched after the project directory, in order,
    /// relative to the manifest's directory. See [`Manifest::texinputs`].
    pub texinputs: Vec<String>,
    /// Where the PDF and auxiliary files go, relative to the manifest's
    /// directory. `None`: each tool's own default (the CLI writes
    /// `<entry>.pdf` next to the entry).
    pub output: Option<String>,
}

/// `[fonts]` — a setting, not a package (proposal §2.4). Read by S4; this
/// crate only carries the names.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Fonts {
    pub text: Option<String>,
    pub math: Option<String>,
    pub mono: Option<String>,
    pub sans: Option<String>,
}

impl Fonts {
    pub fn is_empty(&self) -> bool {
        self.text.is_none() && self.math.is_none() && self.mono.is_none() && self.sans.is_none()
    }
}

/// `[packages] source`: where unresolved packages may be fetched from
/// (proposal §2.3; the fetch itself is S3's, never this crate's).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PackageSource {
    /// A CTAN mirror's `tex-archive` layout.
    #[default]
    Ctan,
    /// Never fetch.
    None,
    /// A registry URL.
    Url(String),
}

impl PackageSource {
    /// The TOML string form (`"ctan"`, `"none"`, or the URL).
    pub fn as_str(&self) -> &str {
        match self {
            PackageSource::Ctan => "ctan",
            PackageSource::None => "none",
            PackageSource::Url(u) => u,
        }
    }
}

/// `[packages] fetch`: what to do when `\usepackage` names something that
/// is neither in the project nor in the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FetchPolicy {
    #[default]
    Ask,
    Always,
    Never,
}

impl FetchPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            FetchPolicy::Ask => "ask",
            FetchPolicy::Always => "always",
            FetchPolicy::Never => "never",
        }
    }
}

/// `[packages]`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Packages {
    pub source: PackageSource,
    pub fetch: FetchPolicy,
    /// Exact versions, `name = "version"`.
    pub pin: BTreeMap<String, String>,
    /// Local libraries, `name = "../dir"` (a directory with its own
    /// `[library]` manifest), relative to this manifest's directory.
    pub path: BTreeMap<String, String>,
}

/// `[library]`: present only when this directory *is* a library that other
/// projects reference through `[packages] path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    pub name: String,
}

/// A parsed `flashtex.toml`. Every field has a default; `Manifest::default()`
/// is exactly "no manifest".
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Manifest {
    pub project: Project,
    pub fonts: Fonts,
    pub packages: Packages,
    pub library: Option<Library>,
}

/// Something the manifest said that this version does not understand, or
/// understands differently. Never fatal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestWarning {
    /// Dotted key path (`project.texinput`, `packages.pin.siunitx`, `fonts`).
    pub key: String,
    pub message: String,
}

impl fmt::Display for ManifestWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.message)
    }
}

/// Why a manifest could not be read at all.
#[derive(Debug)]
pub enum ManifestError {
    /// Reading the file failed for a reason other than "does not exist".
    Io(io::Error),
    /// The text is not TOML. Nothing can be salvaged past a syntax error,
    /// so this is the one case that is not a warning.
    Syntax(String),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::Io(e) => write!(f, "cannot read {FILE_NAME}: {e}"),
            ManifestError::Syntax(e) => write!(f, "{FILE_NAME} is not valid TOML: {e}"),
        }
    }
}

impl std::error::Error for ManifestError {}

impl From<io::Error> for ManifestError {
    fn from(e: io::Error) -> Self {
        ManifestError::Io(e)
    }
}

/// [`Manifest::parse`]'s result.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Parsed {
    pub manifest: Manifest,
    pub warnings: Vec<ManifestWarning>,
}

/// [`Manifest::load`]'s result: `found` is the file that was read, `None`
/// when there was no manifest (and `manifest` is the default).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Loaded {
    pub found: Option<PathBuf>,
    pub manifest: Manifest,
    pub warnings: Vec<ManifestWarning>,
}

/// Where one `[project] texinputs` entry points, decided lexically against
/// the manifest's directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TexInputLocation {
    /// Under the project root: the project-relative directory (`styles`,
    /// `lib/tex`). Its files keep their real project-relative paths.
    Inside(String),
    /// Above the project root, reached through leading `..` segments the
    /// user wrote explicitly (`../shared-macros`): the lexically joined
    /// directory. The reader mounts its files at
    /// [`outside_virtual_dir`]`(index)` and opens the directory through its
    /// own rooted handle, so a symlink inside it is still refused.
    Outside(PathBuf),
    /// Not a directory FlashTeX will look in — absolute, `~`, a forbidden
    /// character, the project directory itself, or above the filesystem
    /// root — with the reason. A diagnostic, never a read.
    Invalid(String),
}

/// One `[project] texinputs` entry, classified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TexInput {
    /// Position in `[project] texinputs`.
    pub index: usize,
    /// The entry as written.
    pub raw: String,
    pub location: TexInputLocation,
}

impl TexInput {
    /// The project-relative directory this entry's files are listed under:
    /// the real one for [`TexInputLocation::Inside`], `texinputs/<index>`
    /// for [`TexInputLocation::Outside`], `None` when invalid.
    pub fn project_dir(&self) -> Option<String> {
        match &self.location {
            TexInputLocation::Inside(dir) => Some(dir.clone()),
            TexInputLocation::Outside(_) => Some(outside_virtual_dir(self.index)),
            TexInputLocation::Invalid(_) => None,
        }
    }
}

/// Classifies a `texinputs` entry against `manifest_dir` (which should be
/// absolute: an `Outside` result pops it lexically). Mirrors
/// `flashtex_project_files::ProjectPath::resolve_in` for what is accepted,
/// and turns the one thing `ProjectPath` refuses — `..` past the root —
/// into `Outside` because the manifest is the user's explicit configuration.
pub fn classify_texinput(raw: &str, manifest_dir: &Path) -> TexInputLocation {
    if let Some(c) = raw.chars().find(|c| matches!(c, '\\' | ':' | '\0') || c.is_control()) {
        return TexInputLocation::Invalid(format!("contains forbidden character {c:?}"));
    }
    if raw.starts_with('/') || raw.starts_with('~') {
        return TexInputLocation::Invalid("must be relative to the manifest's directory, not absolute".into());
    }
    let mut up = 0usize;
    let mut segments: Vec<&str> = Vec::new();
    for seg in raw.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if segments.pop().is_none() {
                    up += 1;
                }
            }
            s => segments.push(s),
        }
    }
    if up == 0 {
        return if segments.is_empty() {
            TexInputLocation::Invalid("is the project directory itself, which is always searched".into())
        } else {
            TexInputLocation::Inside(segments.join("/"))
        };
    }
    let mut dir = manifest_dir.to_path_buf();
    for _ in 0..up {
        if !dir.pop() {
            return TexInputLocation::Invalid("climbs above the filesystem root".into());
        }
    }
    for s in segments {
        dir.push(s);
    }
    TexInputLocation::Outside(dir)
}

impl Manifest {
    /// Finds the manifest that governs `start_dir` (the entry's directory):
    /// `start_dir/flashtex.toml`, else the parent's, and so on. The walk
    /// stops, without looking further up, at the first directory that has
    /// a `.git` entry (a repository is the outermost plausible project, and
    /// a stray `~/flashtex.toml` must never govern a checkout beneath it)
    /// and at the filesystem root. The `.git` directory's own level is still
    /// checked, so a manifest at a repository root is found.
    pub fn locate(start_dir: &Path) -> Option<PathBuf> {
        let mut dir = start_dir.to_path_buf();
        loop {
            let candidate = dir.join(FILE_NAME);
            if candidate.is_file() {
                return Some(candidate);
            }
            // `.git` is a directory in a checkout and a file in a worktree.
            if dir.join(".git").exists() || !dir.pop() {
                return None;
            }
        }
    }

    /// Reads and parses `path`. A missing file is the default manifest
    /// with `found == None`; any other read failure is an error.
    pub fn load(path: &Path) -> Result<Loaded, ManifestError> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Loaded::default()),
            Err(e) => return Err(e.into()),
        };
        let parsed = Manifest::parse(&text)?;
        Ok(Loaded { found: Some(path.to_path_buf()), manifest: parsed.manifest, warnings: parsed.warnings })
    }

    /// Parses manifest text. Unknown keys and sections, and known keys of
    /// the wrong type, become warnings and take their defaults.
    pub fn parse(text: &str) -> Result<Parsed, ManifestError> {
        let table: Table = text.parse().map_err(|e: toml::de::Error| ManifestError::Syntax(e.message().to_string()))?;
        let mut w = Walker { warnings: Vec::new() };
        let mut m = Manifest::default();
        w.unknown_keys(&table, "", &["project", "fonts", "packages", "library"]);
        if let Some(project) = w.table(&table, "project", "project") {
            w.unknown_keys(project, "project", &["entry", "texinputs", "output"]);
            m.project.entry = w.string(project, "entry", "project.entry");
            m.project.texinputs = w.string_list(project, "texinputs", "project.texinputs");
            m.project.output = w.string(project, "output", "project.output");
        }
        if let Some(fonts) = w.table(&table, "fonts", "fonts") {
            w.unknown_keys(fonts, "fonts", &["text", "math", "mono", "sans"]);
            m.fonts.text = w.string(fonts, "text", "fonts.text");
            m.fonts.math = w.string(fonts, "math", "fonts.math");
            m.fonts.mono = w.string(fonts, "mono", "fonts.mono");
            m.fonts.sans = w.string(fonts, "sans", "fonts.sans");
        }
        if let Some(packages) = w.table(&table, "packages", "packages") {
            w.unknown_keys(packages, "packages", &["source", "fetch", "pin", "path"]);
            if let Some(s) = w.string(packages, "source", "packages.source") {
                m.packages.source = match s.as_str() {
                    "ctan" => PackageSource::Ctan,
                    "none" => PackageSource::None,
                    url if url.contains("://") => PackageSource::Url(url.to_string()),
                    other => {
                        w.warn("packages.source", format!("expected \"ctan\", \"none\" or a URL, got {other:?}; using \"ctan\""));
                        PackageSource::Ctan
                    }
                };
            }
            if let Some(s) = w.string(packages, "fetch", "packages.fetch") {
                m.packages.fetch = match s.as_str() {
                    "ask" => FetchPolicy::Ask,
                    "always" => FetchPolicy::Always,
                    "never" => FetchPolicy::Never,
                    other => {
                        w.warn("packages.fetch", format!("expected \"ask\", \"always\" or \"never\", got {other:?}; using \"ask\""));
                        FetchPolicy::Ask
                    }
                };
            }
            m.packages.pin = w.string_map(packages, "pin", "packages.pin");
            m.packages.path = w.string_map(packages, "path", "packages.path");
        }
        if let Some(library) = w.table(&table, "library", "library") {
            w.unknown_keys(library, "library", &["name"]);
            match w.string(library, "name", "library.name") {
                Some(name) => m.library = Some(Library { name }),
                None => w.warn("library", "a [library] section needs `name`; ignoring the section".into()),
            }
        }
        Ok(Parsed { manifest: m, warnings: w.warnings })
    }

    /// The entry's path, `manifest_dir/<entry>`, when the manifest names one.
    pub fn entry_path(&self, manifest_dir: &Path) -> Option<PathBuf> {
        self.project.entry.as_deref().map(|e| manifest_dir.join(e))
    }

    /// The output directory, `manifest_dir/<output>`, when the manifest sets one.
    pub fn output_dir(&self, manifest_dir: &Path) -> Option<PathBuf> {
        self.project.output.as_deref().map(|o| manifest_dir.join(o))
    }

    /// Every `[project] texinputs` entry classified against `manifest_dir`
    /// (see [`classify_texinput`]), in manifest order.
    pub fn texinputs(&self, manifest_dir: &Path) -> Vec<TexInput> {
        self.project
            .texinputs
            .iter()
            .enumerate()
            .map(|(index, raw)| TexInput { index, raw: raw.clone(), location: classify_texinput(raw, manifest_dir) })
            .collect()
    }

    /// The manifest as TOML: every section this version knows, with the
    /// effective value of every key that has one (defaults included for
    /// `texinputs`, `source` and `fetch`, so `manifest show` is explicit).
    /// `parse(m.to_toml())` reproduces `m` exactly.
    pub fn to_toml(&self) -> String {
        let mut out = String::new();
        out.push_str("[project]\n");
        if let Some(e) = &self.project.entry {
            out.push_str(&format!("entry = {}\n", quote(e)));
        }
        out.push_str(&format!(
            "texinputs = [{}]\n",
            self.project.texinputs.iter().map(|t| quote(t)).collect::<Vec<_>>().join(", ")
        ));
        if let Some(o) = &self.project.output {
            out.push_str(&format!("output = {}\n", quote(o)));
        }
        if !self.fonts.is_empty() {
            out.push_str("\n[fonts]\n");
            for (key, value) in [("text", &self.fonts.text), ("math", &self.fonts.math), ("mono", &self.fonts.mono), ("sans", &self.fonts.sans)] {
                if let Some(v) = value {
                    out.push_str(&format!("{key} = {}\n", quote(v)));
                }
            }
        }
        out.push_str("\n[packages]\n");
        out.push_str(&format!("source = {}\n", quote(self.packages.source.as_str())));
        out.push_str(&format!("fetch = {}\n", quote(self.packages.fetch.as_str())));
        for (key, map) in [("pin", &self.packages.pin), ("path", &self.packages.path)] {
            if !map.is_empty() {
                out.push_str(&format!(
                    "{key} = {{ {} }}\n",
                    map.iter().map(|(k, v)| format!("{} = {}", quote_key(k), quote(v))).collect::<Vec<_>>().join(", ")
                ));
            }
        }
        if let Some(l) = &self.library {
            out.push_str(&format!("\n[library]\nname = {}\n", quote(&l.name)));
        }
        out
    }

    /// The commented template `manifest init` / the editor's "Create
    /// flashtex.toml…" write: every key, its default, one line each.
    /// `entry` is the project's actual entry document.
    pub fn template(entry: &str) -> String {
        format!(
            "\
# flashtex.toml — the FlashTeX project manifest (docs/user/project-manifest.md).
# Every key is optional. Without this file FlashTeX behaves exactly as the
# defaults below describe; a key it does not know is a warning, never an error.

[project]
entry = {entry}   # the document to compile (default: the file you open)
texinputs = []    # extra directories searched after the project directory for .sty/.cls/.tex/.bib/.def/.clo, e.g. [\"styles\", \"../shared-macros\"]
# output = \"build\"  # where the PDF and auxiliary files go, relative to this file (default: next to the entry)

[fonts]   # font families by role; overrides nothing the document itself sets (\\setmainfont, \\usepackage{{lmodern}}, …)
# text = \"Latin Modern Roman\"
# math = \"Latin Modern Math\"
# mono = \"Latin Modern Mono\"
# sans = \"Latin Modern Sans\"

[packages]
source = \"ctan\"   # where a package that is not in the project or the cache may be fetched from: \"ctan\", \"none\" (never fetch) or a registry URL
fetch = \"ask\"     # when \\usepackage names such a package: \"ask\" (once per project), \"always\" or \"never\"
pin = {{}}          # exact versions, e.g. {{ siunitx = \"3.3.24\" }}
path = {{}}         # local libraries, e.g. {{ mylib = \"../mylib\" }} — a directory with its own [library] manifest

# [library]        # only when this directory *is* a library other projects reference
# name = \"mylib\"
",
            entry = quote(entry)
        )
    }

    /// `text` with its `[fonts]` table replaced by `fonts` -- the one writer
    /// the editor's Fonts sheet and the project-files helper's `set_fonts`
    /// use, so nothing else composes TOML. Everything outside the table is
    /// kept byte for byte (comments, unknown sections, key order). Inside
    /// it, the four known keys and their commented-out template
    /// placeholders (`# text = "…"`) are replaced by the keys `fonts` sets,
    /// in `text`, `math`, `mono`, `sans` order; any other line of the table
    /// (a comment, a key this version does not know) stays, after them.
    /// The header line itself is kept (with its trailing comment). Without
    /// a `[fonts]` table the new one is appended; without one and with
    /// nothing to set the text is returned unchanged. A table naming
    /// nothing is left as a bare header, which parses as the defaults.
    /// `parse(with_fonts(t, f)).fonts == f` for every `t` that parses.
    pub fn with_fonts(text: &str, fonts: &Fonts) -> String {
        const KEYS: [&str; 4] = ["text", "math", "mono", "sans"];
        let is_header = |line: &str| line.trim_start().starts_with('[');
        let is_fonts_header = |line: &str| {
            let t = line.trim_start();
            t.strip_prefix("[fonts]").is_some_and(|rest| rest.trim_start().is_empty() || rest.trim_start().starts_with('#'))
        };
        // A `text = …` line, or the template's `# text = …` placeholder.
        let is_known_key = |line: &str| {
            let t = line.trim_start();
            let t = t.strip_prefix('#').map_or(t, str::trim_start);
            KEYS.iter().any(|k| t.strip_prefix(k).is_some_and(|rest| rest.trim_start().starts_with('=')))
        };
        let mut new_keys = String::new();
        for (key, value) in KEYS.iter().zip([&fonts.text, &fonts.math, &fonts.mono, &fonts.sans]) {
            if let Some(v) = value {
                new_keys.push_str(&format!("{key} = {}\n", quote(v)));
            }
        }
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        let Some(start) = lines.iter().position(|l| is_fonts_header(l)) else {
            if fonts.is_empty() {
                return text.to_string();
            }
            let mut out = text.to_string();
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            if !out.is_empty() && !out.ends_with("\n\n") {
                out.push('\n');
            }
            out.push_str("[fonts]\n");
            out.push_str(&new_keys);
            return out;
        };
        let end = lines[start + 1..].iter().position(|l| is_header(l)).map_or(lines.len(), |i| start + 1 + i);
        let mut out = String::new();
        for l in &lines[..=start] {
            out.push_str(l);
        }
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&new_keys);
        let body = &lines[start + 1..end];
        let kept: Vec<&str> = body.iter().copied().filter(|l| !is_known_key(l) && !l.trim().is_empty()).collect();
        for l in &kept {
            out.push_str(l);
        }
        if !out.ends_with('\n') {
            out.push('\n');
        }
        // One blank line before the next table when the original had one.
        if end < lines.len() && body.last().is_some_and(|l| l.trim().is_empty()) {
            out.push('\n');
        }
        for l in &lines[end..] {
            out.push_str(l);
        }
        out
    }

    /// Writes [`Manifest::template`] for `entry` at `path`, refusing to
    /// overwrite an existing file (`io::ErrorKind::AlreadyExists`).
    pub fn write_template(path: &Path, entry: &str) -> io::Result<()> {
        let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
        f.write_all(Self::template(entry).as_bytes())?;
        f.flush()
    }

    /// [`Manifest::write_template`] with the conventional `main.tex` entry.
    pub fn write_default(path: &Path) -> io::Result<()> {
        Self::write_template(path, "main.tex")
    }
}

/// A TOML basic string literal for `s` (quotes and escapes as TOML needs).
fn quote(s: &str) -> String {
    Value::String(s.to_string()).to_string()
}

/// A TOML key: bare when it can be, quoted otherwise.
fn quote_key(k: &str) -> String {
    if !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        k.to_string()
    } else {
        quote(k)
    }
}

/// Reads typed values out of the parsed table, recording a warning for
/// every key that is present but not what this version expects.
struct Walker {
    warnings: Vec<ManifestWarning>,
}

impl Walker {
    fn warn(&mut self, key: &str, message: String) {
        self.warnings.push(ManifestWarning { key: key.to_string(), message });
    }

    fn dotted(prefix: &str, key: &str) -> String {
        if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{prefix}.{key}")
        }
    }

    fn unknown_keys(&mut self, t: &Table, prefix: &str, known: &[&str]) {
        for key in t.keys() {
            if !known.contains(&key.as_str()) {
                let what = if prefix.is_empty() && t[key].is_table() { "section" } else { "key" };
                self.warn(&Self::dotted(prefix, key), format!("unknown {what}; ignored"));
            }
        }
    }

    fn table<'t>(&mut self, t: &'t Table, key: &str, path: &str) -> Option<&'t Table> {
        match t.get(key) {
            None => None,
            Some(Value::Table(inner)) => Some(inner),
            Some(other) => {
                self.warn(path, format!("expected a table, got {}; ignored", other.type_str()));
                None
            }
        }
    }

    fn string(&mut self, t: &Table, key: &str, path: &str) -> Option<String> {
        match t.get(key) {
            None => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(other) => {
                self.warn(path, format!("expected a string, got {}; ignored", other.type_str()));
                None
            }
        }
    }

    fn string_list(&mut self, t: &Table, key: &str, path: &str) -> Vec<String> {
        match t.get(key) {
            None => Vec::new(),
            Some(Value::Array(items)) => {
                let mut out = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    match item {
                        Value::String(s) => out.push(s.clone()),
                        other => self.warn(&format!("{path}[{i}]"), format!("expected a string, got {}; ignored", other.type_str())),
                    }
                }
                out
            }
            Some(other) => {
                self.warn(path, format!("expected an array of strings, got {}; ignored", other.type_str()));
                Vec::new()
            }
        }
    }

    fn string_map(&mut self, t: &Table, key: &str, path: &str) -> BTreeMap<String, String> {
        match t.get(key) {
            None => BTreeMap::new(),
            Some(Value::Table(inner)) => {
                let mut out = BTreeMap::new();
                for (k, v) in inner {
                    match v {
                        Value::String(s) => {
                            out.insert(k.clone(), s.clone());
                        }
                        other => self.warn(&Self::dotted(path, k), format!("expected a string, got {}; ignored", other.type_str())),
                    }
                }
                out
            }
            Some(other) => {
                self.warn(path, format!("expected a table of strings, got {}; ignored", other.type_str()));
                BTreeMap::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: &str = r#"
[project]
entry = "paper/main.tex"
texinputs = ["styles", "../shared-macros"]
output = "build"

[fonts]
text = "Libertinus Serif"
math = "Libertinus Math"
mono = "JetBrains Mono"
sans = "Inter"

[packages]
source = "ctan"
fetch = "never"
pin = { siunitx = "3.3.24", tikz = "3.1.10" }
path = { mylib = "../mylib" }

[library]
name = "mylib"
"#;

    fn tmp(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("flashtex-manifest-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn parses_every_section() {
        let p = Manifest::parse(FULL).unwrap();
        assert!(p.warnings.is_empty(), "{:?}", p.warnings);
        let m = p.manifest;
        assert_eq!(m.project.entry.as_deref(), Some("paper/main.tex"));
        assert_eq!(m.project.texinputs, vec!["styles", "../shared-macros"]);
        assert_eq!(m.project.output.as_deref(), Some("build"));
        assert_eq!(m.fonts.text.as_deref(), Some("Libertinus Serif"));
        assert_eq!(m.fonts.sans.as_deref(), Some("Inter"));
        assert_eq!(m.packages.source, PackageSource::Ctan);
        assert_eq!(m.packages.fetch, FetchPolicy::Never);
        assert_eq!(m.packages.pin.get("siunitx").map(String::as_str), Some("3.3.24"));
        assert_eq!(m.packages.pin.len(), 2);
        assert_eq!(m.packages.path.get("mylib").map(String::as_str), Some("../mylib"));
        assert_eq!(m.library, Some(Library { name: "mylib".into() }));
    }

    #[test]
    fn empty_text_is_the_default_and_so_is_a_missing_file() {
        assert_eq!(Manifest::parse("").unwrap(), Parsed::default());
        let d = tmp("missing");
        let loaded = Manifest::load(&d.join(FILE_NAME)).unwrap();
        assert_eq!(loaded, Loaded::default());
        assert_eq!(loaded.manifest, Manifest::default());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn unknown_keys_and_sections_warn_with_their_path_and_never_fail() {
        let p = Manifest::parse(
            "top = 1\n[project]\nentry = \"m.tex\"\ntexinput = [\"s\"]\n[fonts]\nserif = \"x\"\n[build]\njobs = 3\n[packages]\npin = { a = 1, b = \"2\" }\n",
        )
        .unwrap();
        let keys: Vec<&str> = p.warnings.iter().map(|w| w.key.as_str()).collect();
        // Top-level unknowns first, in key order (the table is sorted), then
        // each known section's in the order the sections are read.
        assert_eq!(keys, vec!["build", "top", "project.texinput", "fonts.serif", "packages.pin.a"], "{:?}", p.warnings);
        assert!(p.warnings[0].message.contains("unknown section"));
        assert!(p.warnings[1].message.contains("unknown key"));
        assert!(p.warnings[2].message.contains("unknown key"));
        assert_eq!(p.manifest.project.entry.as_deref(), Some("m.tex"));
        assert_eq!(p.manifest.packages.pin.get("b").map(String::as_str), Some("2"));
        assert!(!p.manifest.packages.pin.contains_key("a"));
    }

    #[test]
    fn wrong_types_and_bad_enum_values_warn_and_take_defaults() {
        let p = Manifest::parse(
            "[project]\nentry = 3\ntexinputs = \"styles\"\n[packages]\nsource = \"ftp\"\nfetch = \"maybe\"\n[library]\nname = 1\n",
        )
        .unwrap();
        let keys: Vec<&str> = p.warnings.iter().map(|w| w.key.as_str()).collect();
        assert_eq!(keys, vec!["project.entry", "project.texinputs", "packages.source", "packages.fetch", "library.name", "library"]);
        assert_eq!(p.manifest, Manifest::default());
        let url = Manifest::parse("[packages]\nsource = \"https://mirror.example/tex-archive\"\n").unwrap();
        assert!(url.warnings.is_empty());
        assert_eq!(url.manifest.packages.source, PackageSource::Url("https://mirror.example/tex-archive".into()));
    }

    #[test]
    fn syntax_errors_are_errors() {
        match Manifest::parse("[project\nentry = ") {
            Err(ManifestError::Syntax(_)) => {}
            other => panic!("expected a syntax error, got {other:?}"),
        }
    }

    #[test]
    fn template_parses_clean_and_round_trips() {
        let t = Manifest::template("paper.tex");
        let p = Manifest::parse(&t).unwrap();
        assert!(p.warnings.is_empty(), "{:?}", p.warnings);
        let mut expected = Manifest::default();
        expected.project.entry = Some("paper.tex".into());
        assert_eq!(p.manifest, expected);
        // Every key of every section is in the template, set or commented.
        for key in ["entry", "texinputs", "output", "text", "math", "mono", "sans", "source", "fetch", "pin", "path", "name"] {
            assert!(t.lines().any(|l| l.trim_start_matches("# ").starts_with(&format!("{key} = "))), "{key} missing from the template");
        }

        let full = Manifest::parse(FULL).unwrap().manifest;
        let again = Manifest::parse(&full.to_toml()).unwrap();
        assert!(again.warnings.is_empty(), "{:?}", again.warnings);
        assert_eq!(again.manifest, full);
        let dflt = Manifest::parse(&Manifest::default().to_toml()).unwrap();
        assert!(dflt.warnings.is_empty());
        assert_eq!(dflt.manifest, Manifest::default());
        // Names that need quoting survive too.
        let mut odd = Manifest::default();
        odd.packages.pin.insert("a b".into(), "1\"2".into());
        odd.fonts.text = Some("Quote \" Back\\slash".into());
        assert_eq!(Manifest::parse(&odd.to_toml()).unwrap().manifest, odd);
    }

    #[test]
    fn write_template_refuses_to_overwrite() {
        let d = tmp("write");
        let path = d.join(FILE_NAME);
        Manifest::write_default(&path).unwrap();
        let loaded = Manifest::load(&path).unwrap();
        assert_eq!(loaded.found.as_deref(), Some(path.as_path()));
        assert_eq!(loaded.manifest.project.entry.as_deref(), Some("main.tex"));
        assert_eq!(Manifest::write_template(&path, "x.tex").unwrap_err().kind(), io::ErrorKind::AlreadyExists);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn texinputs_are_classified_and_an_escape_is_a_diagnostic_not_a_read() {
        let root = Path::new("/home/u/proj");
        let inside = |r: &str| classify_texinput(r, root);
        assert_eq!(inside("styles"), TexInputLocation::Inside("styles".into()));
        assert_eq!(inside("./lib//tex/"), TexInputLocation::Inside("lib/tex".into()));
        assert_eq!(inside("a/../styles"), TexInputLocation::Inside("styles".into()));
        assert_eq!(inside("../shared-macros"), TexInputLocation::Outside(PathBuf::from("/home/u/shared-macros")));
        assert_eq!(inside("../../x"), TexInputLocation::Outside(PathBuf::from("/home/x")));
        for (raw, why) in [
            ("/usr/share/texmf", "absolute"),
            ("~/texmf", "absolute"),
            ("a\\b", "forbidden character"),
            ("C:/texmf", "forbidden character"),
            (".", "project directory itself"),
            ("../../../../../x", "filesystem root"),
        ] {
            match inside(raw) {
                TexInputLocation::Invalid(m) => assert!(m.contains(why), "{raw}: {m}"),
                other => panic!("{raw}: expected Invalid, got {other:?}"),
            }
        }
        let mut m = Manifest::default();
        m.project.texinputs = vec!["styles".into(), "/abs".into(), "../shared".into()];
        let t = m.texinputs(root);
        assert_eq!(t.iter().map(|t| t.project_dir()).collect::<Vec<_>>(), vec![Some("styles".into()), None, Some("texinputs/2".into())]);
        assert_eq!(t[2].index, 2);
        assert!(is_texinput_file("a.sty") && is_texinput_file("b.cls") && is_texinput_file("c.def") && is_texinput_file("d.clo"));
        assert!(is_texinput_file("e.tex") && is_texinput_file("f.bib"));
        assert!(!is_texinput_file("README.md") && !is_texinput_file(".sty") && !is_texinput_file("sty"));
    }

    #[test]
    fn locate_walks_up_and_stops_at_a_git_boundary() {
        let d = tmp("locate");
        std::fs::create_dir_all(d.join("repo/paper/chapters")).unwrap();
        std::fs::create_dir_all(d.join("repo/.git")).unwrap();
        std::fs::write(d.join(FILE_NAME), "").unwrap(); // above the repo: never seen
        assert_eq!(Manifest::locate(&d.join("repo/paper/chapters")), None, "a manifest above .git must not govern the checkout");
        std::fs::write(d.join("repo").join(FILE_NAME), "").unwrap();
        assert_eq!(Manifest::locate(&d.join("repo/paper/chapters")), Some(d.join("repo").join(FILE_NAME)), "the .git level itself is checked");
        std::fs::write(d.join("repo/paper").join(FILE_NAME), "").unwrap();
        assert_eq!(Manifest::locate(&d.join("repo/paper/chapters")), Some(d.join("repo/paper").join(FILE_NAME)), "the nearest wins");
        assert_eq!(Manifest::locate(&d.join("repo/paper")), Some(d.join("repo/paper").join(FILE_NAME)));
        // A worktree's `.git` is a file; it bounds the walk the same way.
        std::fs::create_dir_all(d.join("wt/sub")).unwrap();
        std::fs::write(d.join("wt/.git"), "gitdir: elsewhere\n").unwrap();
        assert_eq!(Manifest::locate(&d.join("wt/sub")), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn with_fonts_rewrites_only_the_fonts_table_and_round_trips() {
        let fonts = Fonts { text: Some("Libertinus Serif".into()), math: None, mono: Some("JetBrains Mono".into()), sans: None };
        // The template: placeholders replaced, comments elsewhere untouched.
        let t = Manifest::with_fonts(&Manifest::template("main.tex"), &fonts);
        assert_eq!(Manifest::parse(&t).unwrap().manifest.fonts, fonts);
        assert!(t.contains("[fonts]   # font families by role"), "the header keeps its comment:\n{t}");
        assert!(t.contains("[fonts]   # font families by role; overrides nothing the document itself sets (\\setmainfont, \\usepackage{lmodern}, …)\ntext = \"Libertinus Serif\"\nmono = \"JetBrains Mono\"\n\n[packages]"), "{t}");
        assert!(!t.contains("# text = ") && !t.contains("# sans = "), "placeholders go:\n{t}");
        assert!(t.starts_with("# flashtex.toml — the FlashTeX project manifest") && t.contains("entry = \"main.tex\"   # the document") && t.contains("# [library]"), "{t}");
        // A full manifest: other sections byte-identical, unknown keys of the
        // table kept after the new ones, an existing key dropped when unset.
        let full = FULL.replace("[fonts]\n", "[fonts]\nserif = \"keep me\"   # unknown to this version\n");
        let t = Manifest::with_fonts(&full, &fonts);
        let p = Manifest::parse(&t).unwrap();
        assert_eq!(p.manifest.fonts, fonts);
        assert_eq!(p.warnings.iter().map(|w| w.key.as_str()).collect::<Vec<_>>(), vec!["fonts.serif"]);
        assert!(t.contains("[fonts]\ntext = \"Libertinus Serif\"\nmono = \"JetBrains Mono\"\nserif = \"keep me\"   # unknown to this version\n\n[packages]"), "{t}");
        let before = full.split("[fonts]").next().unwrap();
        assert!(t.starts_with(before), "everything before the table is untouched");
        let after = full.split("[packages]").nth(1).unwrap();
        assert!(t.ends_with(after), "everything after the table is untouched");
        // Nothing set: the table is emptied (a bare header) and still parses.
        let t = Manifest::with_fonts(&full, &Fonts::default());
        assert!(Manifest::parse(&t).unwrap().manifest.fonts.is_empty());
        assert!(t.contains("[fonts]\nserif = \"keep me\""), "{t}");
        // No table: appended; nothing to set and no table: unchanged.
        let bare = "[project]\nentry = \"m.tex\"";
        assert_eq!(Manifest::with_fonts(bare, &Fonts::default()), bare);
        let t = Manifest::with_fonts(bare, &fonts);
        assert_eq!(t, "[project]\nentry = \"m.tex\"\n\n[fonts]\ntext = \"Libertinus Serif\"\nmono = \"JetBrains Mono\"\n");
        assert_eq!(Manifest::parse(&t).unwrap().manifest.fonts, fonts);
        assert_eq!(Manifest::with_fonts("", &fonts), "[fonts]\ntext = \"Libertinus Serif\"\nmono = \"JetBrains Mono\"\n");
        // A name needing escapes is quoted like `to_toml` quotes it.
        let odd = Fonts { text: Some("Quote \" Back\\slash".into()), ..Fonts::default() };
        assert_eq!(Manifest::parse(&Manifest::with_fonts("", &odd)).unwrap().manifest.fonts, odd);
    }
}
