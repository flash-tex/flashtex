//! File lookup (DESIGN.md §4.4).
//!
//! `tex.web` opens files through a system-dependent `name_of_file`, and its
//! only notion of a search path is the device prefixes of §§514-520
//! (`TeXinputs:`, `TeXfonts:`, `TeXformats:`). This module is what replaces
//! them. The decision record is docs/evidence/file-resolver-2026-09-29/:
//! TeX Live's own kpathsea is linked (feature `kpathsea`, the default) rather
//! than reimplemented, because it *is* the reference for "identical results to
//! kpsewhich" and nothing we could write would be measurably faster.
//!
//! Resolvers are chosen once per process by [`default_resolver`]:
//!
//! | `FLASHTEX_RESOLVER` | resolver |
//! |---|---|
//! | `cwd` | [`CwdResolver`]: working directory, then `FLASHTEX_*` path variables |
//! | `bundle` | the fetched bundle `FLASHTEX_BUNDLE_DIGEST` (src/bundle/: `FLASHTEX_BUNDLE_URL`, `FLASHTEX_BUNDLE_OFFLINE`), else kpathsea over the flat directory in `FLASHTEX_BUNDLE` |
//! | `kpathsea-self` | kpathsea set up from this program's own path, as web2c sets it up from `argv[0]`: texmf.cnf comes from `TEXMFCNF` (pdfTeX's regression tests, `scripts/pdftex-regression.sh`) |
//! | unset or `kpathsea` | kpathsea over the TeX Live found by [`discover_texlive`], else the fetched bundle if `FLASHTEX_BUNDLE_DIGEST` is set, else `cwd` |
//!
//! kpathsea keeps its configuration in the process environment
//! (`kpathsea_xputenv` is `putenv`), so there is one resolver per process.

use std::path::{Path, PathBuf};

/// The kinds of file the engine asks for. The names are kpathsea's format
/// names, as `kpsewhich --help-formats` prints them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Tex,
    Tfm,
    Fmt,
    Type1,
    Enc,
    Map,
    Vf,
    Bib,
    Bst,
    Cnf,
    Pk,
    /// `web2c files`: TCX files and the like.
    Web2c,
    /// `misc fonts`: pdfTeX's `.pgc` Type 3 glyph files.
    MiscFonts,
    /// `truetype fonts`.
    TrueType,
    /// `opentype fonts`.
    OpenType,
    /// `subfont definition files` (`.sfd`).
    Sfd,
}

impl Format {
    pub fn kpse_name(self) -> &'static str {
        match self {
            Format::Tex => "tex",
            Format::Tfm => "tfm",
            Format::Fmt => "fmt",
            Format::Type1 => "type1 fonts",
            Format::Enc => "enc files",
            Format::Map => "map",
            Format::Vf => "vf",
            Format::Bib => "bib",
            Format::Bst => "bst",
            Format::Cnf => "cnf",
            Format::Pk => "pk",
            Format::Web2c => "web2c files",
            Format::MiscFonts => "misc fonts",
            Format::TrueType => "truetype fonts",
            Format::OpenType => "opentype fonts",
            Format::Sfd => "subfont definition files",
        }
    }

    pub fn all() -> &'static [Format] {
        &[
            Format::Tex,
            Format::Tfm,
            Format::Fmt,
            Format::Type1,
            Format::Enc,
            Format::Map,
            Format::Vf,
            Format::Bib,
            Format::Bst,
            Format::Cnf,
            Format::Pk,
            Format::Web2c,
            Format::MiscFonts,
            Format::TrueType,
            Format::OpenType,
            Format::Sfd,
        ]
    }

    pub fn from_kpse_name(s: &str) -> Option<Format> {
        Format::all().iter().copied().find(|f| f.kpse_name() == s)
    }
}

/// Finds input files. The contract is `kpsewhich -format=FORMAT NAME`:
/// the path of an existing file, or `None`; never creates anything.
pub trait FileResolver: Send {
    fn find(&mut self, name: &str, format: Format) -> Option<PathBuf>;
    /// `kpse_find_file(name, format, must_exist)` as web2c's `open_input`
    /// calls it: with `must_exist`, kpathsea also searches the disk beyond
    /// `ls-R` and may run an mktex script (`mktextfm`). The flag says
    /// whether a script made the file.
    fn find_ex(
        &mut self,
        name: &str,
        format: Format,
        _must_exist: bool,
    ) -> (Option<PathBuf>, bool) {
        (self.find(name, format), false)
    }
    /// `kpsewhich -all NAME`: every match in search order (fmtutil reads
    /// every `fmtutil.cnf` this way). By default, the one `find` returns.
    fn find_all(&mut self, name: &str, format: Format) -> Vec<PathBuf> {
        self.find(name, format).into_iter().collect()
    }
    /// One line for logs and error messages.
    fn describe(&self) -> String;
    /// A texmf.cnf variable, expanded (`kpsewhich -var-value`), for the
    /// few settings the engine itself reads (`log_openout`). None where
    /// there is no texmf.cnf.
    fn config_var(&mut self, _var: &str) -> Option<String> {
        None
    }
    /// kpathsea's `kpse_in_name_ok` (`write` false) or `kpse_out_name_ok`
    /// (`write` true): may the file be opened, under texmf.cnf's
    /// `openin_any` and `openout_any`? Without texmf.cnf, yes.
    fn name_ok(&mut self, _name: &str, _write: bool) -> bool {
        true
    }
    /// pdftex.web's `kpse_init_prog(prefix, dpi, mode, nil)` and
    /// `kpse_set_program_enabled(kpse_pk_format, 1, kpse_src_compile)`, at
    /// the start of PDF output: the resolution and mode mktexpk makes
    /// bitmap fonts at. Nothing where there is no kpathsea.
    fn init_pk(&mut self, _prefix: &str, _dpi: u32, _mode: Option<&[u8]>) {}
    /// writet3.c's `kpse_find_pk(name, dpi, &font_ret)`: the PK file of
    /// font `name` at `dpi` (or an alias or a fallback resolution), which
    /// mktexpk may make if `make` (and kpathsea's settings allow it). None
    /// where there is no kpathsea.
    fn find_pk(&mut self, _name: &str, _dpi: u32, _make: bool) -> Option<PkGlyph> {
        None
    }
}

/// What `kpse_find_pk` found: the file, `font_ret.name` and `font_ret.dpi`
/// (the font and resolution the file is for), and whether mktexpk made it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PkGlyph {
    pub path: PathBuf,
    pub name: Vec<u8>,
    pub dpi: u32,
    pub made: bool,
}

// ---------------------------------------------------------------------------

/// `tex.web`'s own behaviour: the name as given, relative to the working
/// directory, then each directory of a colon-separated variable for the format
/// (`FLASHTEX_INPUTS`, `FLASHTEX_TFM_PATH`, `FLASHTEX_FORMATS`). The trip test
/// uses this: tripman.tex defines it on files in the current area.
///
/// A name kpathsea takes as absolute or explicitly relative (`/`, `./`,
/// `../`, and on Windows also `\`, `.\`, `..\`, `X:`; see
/// `kpse_absolute_p`) is opened as given, never along the path variable.
/// With `dot` set, any other name found in the working directory comes back
/// as `./name` (`./sub/name` for `sub/name`), which is what kpathsea returns
/// for a search path of `.` (the e-trip test's texmf.cnf), and so what
/// pdfTeX's log shows.
#[derive(Default)]
pub struct CwdResolver {
    pub dot: bool,
}

impl FileResolver for CwdResolver {
    fn find(&mut self, name: &str, format: Format) -> Option<PathBuf> {
        // kpathsea's suffix rule for TeX input: `name.tex` first, then
        // `name` (web2c's `\input` and `\openin` do not add `.tex`).
        if format == Format::Tex && !name.ends_with(".tex") {
            if let Some(p) = self.find_one(&format!("{name}.tex"), format) {
                return Some(p);
            }
        }
        // TFM files, which tex.ch packs without `.tfm` (tex.ch [30.563]:
        // "kpse_find_file will append the .tfm"): kpathsea's TFM format is
        // suffix-only (`suffix_search_only`), so a name not ending in `.tfm`
        // is looked for as `name.tfm` and never as given. A file `zzbare` is
        // not the font `zzbare`, in any directory (`kpsewhich -format=tfm
        // zzbare` finds nothing; pdfTeX's `\font` gives nullfont).
        if format == Format::Tfm && !name.ends_with(".tfm") {
            return self.find_one(&format!("{name}.tfm"), format);
        }
        self.find_one(name, format)
    }
    fn describe(&self) -> String {
        "working directory (CwdResolver)".into()
    }
}

impl CwdResolver {
    fn find_one(&self, name: &str, format: Format) -> Option<PathBuf> {
        let p = Path::new(name);
        // kpathsea opens a name that is absolute or explicitly relative
        // (`kpse_absolute_p`: `/x`, `./x`, `../x`; on Windows also `\x`,
        // `.\x`, `..\x`, `C:x`) as given and never along the path; it
        // searches any other name along the path, so one found through `.`
        // is `./name`, also `./sub/name`, written with `/` on every OS
        // (DIR_SEP_STRING is `/` on Windows too), not `Path::join`'s `.\NAME`.
        let as_given = kpse_absolute(name);
        if p.is_file() {
            if self.dot && !as_given {
                return Some(PathBuf::from(format!("./{name}")));
            }
            return Some(p.to_path_buf());
        }
        if as_given {
            return None;
        }
        let var = match format {
            Format::Tfm => "FLASHTEX_TFM_PATH",
            Format::Fmt => "FLASHTEX_FORMATS",
            _ => "FLASHTEX_INPUTS",
        };
        let path = std::env::var(var).ok()?;
        // kpathsea's ENV_SEP: `;` on Windows, where `:` follows a drive.
        path.split(if cfg!(windows) { ';' } else { ':' })
            .filter(|d| !d.is_empty())
            .map(|d| Path::new(d).join(name))
            .find(|c| c.is_file())
    }
}

/// kpathsea's `kpathsea_absolute_p (kpse, name, true)`
/// (third_party/kpathsea/absolute.c): whether `name` is absolute or
/// explicitly relative, which kpathsea opens as given and never looks for
/// along a search path (pathsearch.c `search`). `dosish` is kpathsea's
/// `DOSISH` (config.h 37-42: Windows), a parameter so that both branches are
/// compiled and tested on every host. Bytes, as the C does:
/// - absolute.c 38: a leading directory separator, `/`, or under DOSISH also
///   `\` (c-pathch.h 38); this covers WIN32's UNC names (absolute.c 43-47);
/// - absolute.c 41 (DOSISH): any first byte, then the device separator `:`
///   (c-pathch.h 35), so `C:x` as well as `C:\x`;
/// - absolute.c 53-62: `.` then a separator, or `..` then a separator.
fn kpse_absolute_p(name: &str, dosish: bool) -> bool {
    let b = name.as_bytes();
    let sep = |i: usize| matches!(b.get(i), Some(b'/')) || (dosish && b.get(i) == Some(&b'\\'));
    let absolute = sep(0) || (dosish && !b.is_empty() && b.get(1) == Some(&b':'));
    let explicit_relative =
        b.first() == Some(&b'.') && (sep(1) || (b.get(1) == Some(&b'.') && sep(2)));
    absolute || explicit_relative
}

/// [`kpse_absolute_p`] for the host: `DOSISH` exactly on Windows.
fn kpse_absolute(name: &str) -> bool {
    kpse_absolute_p(name, cfg!(windows))
}

// ---------------------------------------------------------------------------

/// A TeX Live found without a shell environment (DESIGN.md 4.4): a GUI app
/// inherits launchd's environment, whose PATH is `/usr/bin:/bin:/usr/sbin:/sbin`,
/// so the TeX Live a user's terminal would run is looked for explicitly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TexLiveInstall {
    /// The directory holding `kpsewhich` (and `pdftex`), as found.
    pub bin: PathBuf,
    /// Why this one: which rule of [`discover_texlive`] found it.
    pub how: String,
}

impl TexLiveInstall {
    /// One line for reports: `/Library/TeX/texbin (MacTeX...) -> /usr/local/texlive/2026/bin/universal-darwin`.
    pub fn describe(&self) -> String {
        let real = std::fs::canonicalize(self.bin.join(crate::os::exe_name("kpsewhich")))
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf));
        match real {
            Some(r) if r != self.bin => {
                format!("{} ({}) -> {}", self.bin.display(), self.how, r.display())
            }
            _ => format!("{} ({})", self.bin.display(), self.how),
        }
    }
}

/// The candidate `bin` directories, in the order [`discover_texlive`] tries
/// them, each with the rule that proposes it:
///
/// 1. `FLASHTEX_TEXLIVE_BIN`, if set (and only it);
/// 2. the process's own `PATH`, which is the login shell's when the engine
///    is started from a terminal;
/// 3. on macOS, the PATH a login shell starts from: `/etc/paths`, then each
///    file of `/etc/paths.d` in name order, as `path_helper(8)` builds it
///    (MacTeX installs `/etc/paths.d/TeX` with `/Library/TeX/texbin`);
/// 4. MacTeX's `/Library/TeX/texbin`;
/// 5. `install-tl`'s default and common `TEXDIR`s, newest year first:
///    `/usr/local/texlive/<year>/bin/<arch>`, `~/texlive/<year>/bin/<arch>`,
///    `/opt/texlive/<year>/bin/<arch>`;
/// 6. Homebrew and distribution directories: `/opt/homebrew/bin`,
///    `/usr/local/bin`, `/usr/bin`.
///
/// On Windows, 3, 4 and 6 do not apply and 5 is `install-tl`'s Windows
/// default, `%SystemDrive%\texlive\<year>\bin\<arch>`; `kpsewhich` is
/// `kpsewhich.exe`.
pub fn texlive_candidates() -> Vec<(PathBuf, String)> {
    let mut c: Vec<(PathBuf, String)> = vec![];
    if let Some(d) = std::env::var_os("FLASHTEX_TEXLIVE_BIN") {
        c.push((PathBuf::from(d), "FLASHTEX_TEXLIVE_BIN".into()));
        return c;
    }
    if let Some(path) = std::env::var_os("PATH") {
        for d in std::env::split_paths(&path) {
            if d.is_absolute() {
                c.push((d, "PATH".into()));
            }
        }
    }
    if cfg!(target_os = "macos") {
        let mut files = vec![PathBuf::from("/etc/paths")];
        if let Ok(rd) = std::fs::read_dir("/etc/paths.d") {
            let mut v: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
            // path_helper reads the directory in byte order of the names.
            v.sort();
            files.extend(v);
        }
        for f in files {
            if let Ok(t) = std::fs::read_to_string(&f) {
                for l in t.lines().map(str::trim).filter(|l| l.starts_with('/')) {
                    c.push((
                        PathBuf::from(l),
                        format!("login-shell PATH ({})", f.display()),
                    ));
                }
            }
        }
    }
    let mut roots = vec![];
    if cfg!(windows) {
        // install-tl's Windows default, `%SystemDrive%\texlive\<year>`
        // (bin\windows since 2023, bin\win32 before).
        let drive = std::env::var_os("SystemDrive").unwrap_or_else(|| "C:".into());
        let mut r = drive;
        r.push("\\texlive");
        roots.push(PathBuf::from(r));
    } else {
        c.push((PathBuf::from("/Library/TeX/texbin"), "MacTeX".into()));
        roots.push(PathBuf::from("/usr/local/texlive"));
        if let Some(h) = std::env::var_os("HOME") {
            roots.push(PathBuf::from(h).join("texlive"));
        }
        roots.push(PathBuf::from("/opt/texlive"));
    }
    for root in roots {
        let Ok(rd) = std::fs::read_dir(&root) else {
            continue;
        };
        let mut years: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.len() == 4 && n.bytes().all(|b| b.is_ascii_digit()))
            })
            .collect();
        years.sort();
        for y in years.into_iter().rev() {
            if let Ok(rd) = std::fs::read_dir(y.join("bin")) {
                let mut arches: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
                arches.sort();
                for a in arches {
                    c.push((a, format!("TeX Live installer TEXDIR ({})", y.display())));
                }
            }
        }
    }
    if !cfg!(windows) {
        for d in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"] {
            c.push((PathBuf::from(d), "system directory".into()));
        }
    }
    c
}

/// The user's TeX Live: the first of [`texlive_candidates`] that holds
/// `kpsewhich`, which is the `kpsewhich` a login shell would run unless
/// the user's shell start-up files change PATH.
pub fn discover_texlive() -> Option<TexLiveInstall> {
    texlive_candidates()
        .into_iter()
        .find(|(d, _)| d.join(crate::os::exe_name("kpsewhich")).is_file())
        .map(|(bin, how)| TexLiveInstall { bin, how })
}

/// The `bin` directory of [`discover_texlive`].
pub fn find_texlive_bin() -> Option<PathBuf> {
    discover_texlive().map(|t| t.bin)
}

#[cfg(feature = "kpathsea")]
pub use kpse::KpathseaResolver;

/// kpathsea's `kpathsea_version_string` (`kpathsea version 6.4.2`), from the
/// vendored library; empty without it.
pub fn kpathsea_version() -> String {
    #[cfg(feature = "kpathsea")]
    {
        extern "C" {
            static kpathsea_version_string: *const std::ffi::c_char;
        }
        // SAFETY: a NUL-terminated string constant of the linked kpathsea.
        unsafe { std::ffi::CStr::from_ptr(kpathsea_version_string) }
            .to_string_lossy()
            .into_owned()
    }
    #[cfg(not(feature = "kpathsea"))]
    String::new()
}

#[cfg(feature = "kpathsea")]
mod kpse {
    use super::{FileResolver, Format};
    use std::collections::HashMap;
    use std::ffi::{c_char, c_int, c_void, CStr, CString};
    use std::path::{Path, PathBuf};

    extern "C" {
        fn flashtex_kpse_new(
            argv0: *const c_char,
            progname: *const c_char,
            engine: *const c_char,
            env: *const *const c_char,
            mktextfm: c_int,
        ) -> *mut c_void;
        fn flashtex_kpse_find_ex(
            k: *mut c_void,
            name: *const c_char,
            format: c_int,
            must_exist: c_int,
            made: *mut c_int,
        ) -> *mut c_char;
        fn flashtex_kpse_format(k: *mut c_void, name: *const c_char) -> c_int;
        fn flashtex_kpse_find(k: *mut c_void, name: *const c_char, format: c_int) -> *mut c_char;
        fn flashtex_kpse_var_value(k: *mut c_void, var: *const c_char) -> *mut c_char;
        fn flashtex_kpse_free(p: *mut c_void);
        fn flashtex_kpse_name_ok(k: *mut c_void, name: *const c_char, write: c_int) -> c_int;
        fn flashtex_kpse_find_all(
            k: *mut c_void,
            name: *const c_char,
            format: c_int,
        ) -> *mut *mut c_char;
        fn flashtex_kpse_free_list(list: *mut *mut c_char);
        fn flashtex_kpse_init_pk(
            k: *mut c_void,
            prefix: *const c_char,
            dpi: std::ffi::c_uint,
            mode: *const c_char,
        );
        fn flashtex_kpse_find_pk(
            k: *mut c_void,
            name: *const c_char,
            dpi: std::ffi::c_uint,
            make: c_int,
            ret_name: *mut *mut c_char,
            ret_dpi: *mut std::ffi::c_uint,
            made: *mut c_int,
        ) -> *mut c_char;
    }

    /// TeX Live's kpathsea, vendored and linked (third_party/kpathsea).
    pub struct KpathseaResolver {
        k: *mut c_void,
        formats: HashMap<Format, c_int>,
        what: String,
    }

    // One kpathsea instance, used from one thread at a time.
    unsafe impl Send for KpathseaResolver {}

    fn take(p: *mut c_char) -> Option<String> {
        if p.is_null() {
            return None;
        }
        let s = unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
        unsafe { flashtex_kpse_free(p as *mut c_void) };
        Some(s)
    }

    impl KpathseaResolver {
        /// kpathsea configured exactly as `kpsewhich -progname=PROGNAME
        /// -engine=ENGINE` run from `bin_dir` would be. `bin_dir` is what
        /// kpathsea derives SELFAUTOLOC and friends from, and through them
        /// where texmf.cnf is, so no environment variable is needed.
        pub fn for_texlive(bin_dir: &Path, progname: &str, engine: &str) -> KpathseaResolver {
            let argv0 = bin_dir.join(crate::os::exe_name("kpsewhich"));
            // Not on Windows: canonical paths there are `\\?\` verbatim
            // paths, which kpathsea's SELFAUTO* parsing does not expect, and
            // TeX Live's Windows bin directory has no links to resolve.
            #[cfg(not(windows))]
            let argv0 = std::fs::canonicalize(&argv0).unwrap_or(argv0);
            Self::new(
                &argv0,
                progname,
                engine,
                &[],
                format!("kpathsea ({})", bin_dir.display()),
                true,
            )
        }

        /// kpathsea set up from this program's own path, as web2c's
        /// `kpse_set_program_name(argv[0], ...)` sets it up: SELFAUTOLOC and
        /// friends are this executable's directories, so texmf.cnf is found
        /// through `TEXMFCNF`.
        pub fn for_self(progname: &str, engine: &str) -> KpathseaResolver {
            let argv0 = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("pdftex"));
            Self::new(
                &argv0,
                progname,
                engine,
                &[],
                "kpathsea (this program)".into(),
                true,
            )
        }

        /// A Tectonic-style bundle: one flat directory of files, no texmf.cnf,
        /// no ls-R. Every search path is set to that directory, so lookups keep
        /// kpathsea's suffix rules (`\input foo` finds `foo.tex`) exactly.
        pub fn for_bundle(dir: &Path, progname: &str, engine: &str) -> KpathseaResolver {
            let d = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
            let d = d.to_string_lossy().into_owned();
            Self::for_bundle_paths(&d, &format!(".:{d}"), &d, &[], progname, engine)
        }

        /// A fetched bundle's tree (src/bundle/): a TeX Live root
        /// (`texmf-dist/`, `texmf-var/`) under `root`, each tree with an
        /// `ls-R` listing every file of the bundle in it.
        ///
        /// With TeX Live's own `texmf-dist/web2c/texmf.cnf` there, kpathsea
        /// reads it, so every search path and setting is TeX Live's, and
        /// only the tree variables are set, as an installation's own
        /// `SELFAUTOPARENT` would set them: `TEXMFROOT` is `root`,
        /// `TEXMFDIST` and `TEXMFSYSVAR` its trees, `TEXMFSYSCONFIG` its
        /// `texmf-config` (what the installation configured, e.g. the paper
        /// size in `pdftexconfig.tex`, when the bundle carries it), and the
        /// user and site trees (`TEXMFHOME`, `TEXMFVAR`, `TEXMFCONFIG`,
        /// `TEXMFLOCAL`) point at empty directories inside `root`, since a
        /// machine without TeX Live has none. Without a texmf.cnf, every
        /// search path is `.` and each tree's database (`!!tree//`).
        pub fn for_bundle_tree(root: &Path, progname: &str, engine: &str) -> KpathseaResolver {
            let d = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
            let d = d.to_string_lossy().into_owned();
            let cnf = format!("{d}/texmf-dist/web2c");
            let _ = std::fs::create_dir_all(&cnf);
            if Path::new(&cnf).join("texmf.cnf").is_file() {
                let mut env: Vec<(String, String)> = vec![
                    ("TEXMFCNF".into(), cnf.clone()),
                    ("TEXMFROOT".into(), d.clone()),
                    ("TEXMFDIST".into(), format!("{d}/texmf-dist")),
                    ("TEXMFSYSVAR".into(), format!("{d}/texmf-var")),
                ];
                for (v, sub) in [
                    ("TEXMFSYSCONFIG", "texmf-config"),
                    ("TEXMFLOCAL", "texmf-local"),
                    ("TEXMFHOME", "texmf-home"),
                    ("TEXMFVAR", "texmf-user-var"),
                    ("TEXMFCONFIG", "texmf-user-config"),
                ] {
                    env.push((v.into(), format!("{d}/{sub}")));
                }
                let argv0 =
                    std::env::current_exe().unwrap_or_else(|_| PathBuf::from(&d).join("flashtex"));
                return Self::new(
                    &argv0,
                    progname,
                    engine,
                    &env,
                    format!("kpathsea bundle tree ({d}, TeX Live's texmf.cnf)"),
                    false,
                );
            }
            let mut trees: Vec<String> = std::fs::read_dir(&d)
                .map(|rd| {
                    rd.flatten()
                        .filter(|e| e.path().join("ls-R").is_file())
                        .map(|e| e.path().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
            trees.sort();
            let search: String = std::iter::once(".".to_string())
                .chain(trees.iter().map(|t| format!("!!{t}//")))
                .collect::<Vec<_>>()
                .join(":");
            Self::for_bundle_paths(
                &d,
                &search,
                &cnf,
                &[("TEXMFDBS".into(), trees.join(":"))],
                progname,
                engine,
            )
        }

        fn for_bundle_paths(
            d: &str,
            search: &str,
            cnf: &str,
            extra: &[(String, String)],
            progname: &str,
            engine: &str,
        ) -> KpathseaResolver {
            let d = d.to_string();
            let mut env: Vec<(String, String)> = vec![
                // An existing directory with no texmf.cnf in it would make
                // kpathsea warn; point it at the bundle and set everything.
                ("TEXMFCNF".into(), cnf.to_string()),
                ("TEXMF".into(), d.clone()),
                ("TEXMFDOTDIR".into(), ".".into()),
                // TeX Live 2026's texmf.cnf: restricted \write18 and its list
                // of allowed commands (DESIGN.md 4.5), which the engine
                // reads as `kpse_var_value` there.
                ("shell_escape".into(), "p".into()),
                (
                    "shell_escape_commands".into(),
                    super::TEXLIVE_SHELL_ESCAPE_COMMANDS.into(),
                ),
            ];
            for v in [
                "TEXINPUTS",
                "TFMFONTS",
                "TEXFORMATS",
                "T1FONTS",
                "ENCFONTS",
                "TEXFONTMAPS",
                "VFFONTS",
                "BIBINPUTS",
                "BSTINPUTS",
                "PKFONTS",
                "AFMFONTS",
                "TTFONTS",
                "OPENTYPEFONTS",
                "MISCFONTS",
                "SFDFONTS",
                "TEXPOOL",
                "MFINPUTS",
                "TEXCONFIG",
            ] {
                env.push((v.into(), search.to_string()));
            }
            env.extend(extra.iter().cloned());
            // kpathsea exits (!) if it cannot find the directory of argv[0], so
            // give it a real one; with TEXMFCNF set, SELFAUTO* are not used.
            let argv0 =
                std::env::current_exe().unwrap_or_else(|_| PathBuf::from(&d).join("flashtex"));
            Self::new(
                &argv0,
                progname,
                engine,
                &env,
                format!("kpathsea bundle ({d})"),
                false,
            )
        }

        fn new(
            argv0: &Path,
            progname: &str,
            engine: &str,
            env: &[(String, String)],
            what: String,
            mktextfm: bool,
        ) -> KpathseaResolver {
            let a = CString::new(argv0.to_string_lossy().as_bytes()).unwrap();
            let p = CString::new(progname).unwrap();
            let e = CString::new(engine).unwrap();
            let kv: Vec<CString> = env
                .iter()
                .flat_map(|(k, v)| {
                    [
                        CString::new(k.as_str()).unwrap(),
                        CString::new(v.as_str()).unwrap(),
                    ]
                })
                .collect();
            let mut ptrs: Vec<*const c_char> = kv.iter().map(|c| c.as_ptr()).collect();
            ptrs.push(std::ptr::null());
            let k = unsafe {
                flashtex_kpse_new(
                    a.as_ptr(),
                    p.as_ptr(),
                    e.as_ptr(),
                    ptrs.as_ptr(),
                    mktextfm as c_int,
                )
            };
            let mut formats = HashMap::new();
            for f in Format::all() {
                let n = CString::new(f.kpse_name()).unwrap();
                let id = unsafe { flashtex_kpse_format(k, n.as_ptr()) };
                if id >= 0 {
                    formats.insert(*f, id);
                }
            }
            KpathseaResolver { k, formats, what }
        }

        /// A texmf.cnf variable, expanded (`kpsewhich -var-value`).
        pub fn var_value(&mut self, var: &str) -> Option<String> {
            let v = CString::new(var).ok()?;
            take(unsafe { flashtex_kpse_var_value(self.k, v.as_ptr()) })
        }
    }

    impl FileResolver for KpathseaResolver {
        fn find(&mut self, name: &str, format: Format) -> Option<PathBuf> {
            let f = *self.formats.get(&format)?;
            let n = CString::new(name).ok()?;
            take(unsafe { flashtex_kpse_find(self.k, n.as_ptr(), f) }).map(PathBuf::from)
        }
        fn find_ex(
            &mut self,
            name: &str,
            format: Format,
            must_exist: bool,
        ) -> (Option<PathBuf>, bool) {
            let (Some(&f), Ok(n)) = (self.formats.get(&format), CString::new(name)) else {
                return (None, false);
            };
            let mut made: c_int = 0;
            let p = take(unsafe {
                flashtex_kpse_find_ex(self.k, n.as_ptr(), f, must_exist as c_int, &mut made)
            });
            (p.map(PathBuf::from), made != 0)
        }
        fn find_all(&mut self, name: &str, format: Format) -> Vec<PathBuf> {
            let (Some(&f), Ok(n)) = (self.formats.get(&format), CString::new(name)) else {
                return vec![];
            };
            let list = unsafe { flashtex_kpse_find_all(self.k, n.as_ptr(), f) };
            let mut out = vec![];
            if list.is_null() {
                return out;
            }
            // SAFETY: a NULL-terminated array of NUL-terminated strings,
            // freed once below.
            unsafe {
                let mut p = list;
                while !(*p).is_null() {
                    out.push(PathBuf::from(
                        CStr::from_ptr(*p).to_string_lossy().into_owned(),
                    ));
                    p = p.add(1);
                }
                flashtex_kpse_free_list(list);
            }
            out
        }
        fn describe(&self) -> String {
            self.what.clone()
        }
        fn config_var(&mut self, var: &str) -> Option<String> {
            self.var_value(var)
        }
        fn name_ok(&mut self, name: &str, write: bool) -> bool {
            let Ok(n) = CString::new(name) else {
                return false;
            };
            unsafe { flashtex_kpse_name_ok(self.k, n.as_ptr(), write as c_int) != 0 }
        }
        fn init_pk(&mut self, prefix: &str, dpi: u32, mode: Option<&[u8]>) {
            let p = CString::new(prefix).unwrap_or_default();
            let m = mode.map(|m| CString::new(m.split(|&b| b == 0).next().unwrap_or(b"")).unwrap());
            // SAFETY: NUL-terminated strings (or NULL for no mode) that
            // outlive the call; kpathsea copies what it keeps.
            unsafe {
                flashtex_kpse_init_pk(
                    self.k,
                    p.as_ptr(),
                    dpi,
                    m.as_ref().map_or(std::ptr::null(), |m| m.as_ptr()),
                )
            }
        }
        fn find_pk(&mut self, name: &str, dpi: u32, make: bool) -> Option<super::PkGlyph> {
            let n = CString::new(name).ok()?;
            let (mut rn, mut rd, mut made): (*mut c_char, std::ffi::c_uint, c_int) =
                (std::ptr::null_mut(), 0, 0);
            // SAFETY: the out-pointers are valid; the returned strings are
            // malloc'd (or NULL) and freed by `take`.
            let p = take(unsafe {
                flashtex_kpse_find_pk(
                    self.k,
                    n.as_ptr(),
                    dpi,
                    make as c_int,
                    &mut rn,
                    &mut rd,
                    &mut made,
                )
            })?;
            let rname = if rn.is_null() {
                vec![]
            } else {
                let b = unsafe { CStr::from_ptr(rn) }.to_bytes().to_vec();
                unsafe { flashtex_kpse_free(rn as *mut c_void) };
                b
            };
            Some(super::PkGlyph {
                path: PathBuf::from(p),
                name: rname,
                dpi: rd,
                made: made != 0,
            })
        }
    }
}

/// `shell_escape_commands` of TeX Live 2026's texmf.cnf
/// (third_party/pdftex/regression/texk/kpathsea/texmf.cnf), for resolvers
/// without a texmf.cnf of their own.
pub const TEXLIVE_SHELL_ESCAPE_COMMANDS: &str = "bibtex,bibtex8,extractbb,gregorio,kpsewhich,\
l3sys-query,latexminted,makeindex,memoize-extract.pl,memoize-extract.py,repstopdf,r-mpost,\
texosquery-jre8,";

/// The fetched bundle of `FLASHTEX_BUNDLE_DIGEST` (src/bundle/), if one is
/// configured; a bundle that cannot be opened is reported, and then there
/// is none.
#[cfg(feature = "kpathsea")]
fn fetched_bundle(progname: &str, engine: &str) -> Option<Box<dyn FileResolver>> {
    #[cfg(all(feature = "distribution", not(feature = "tex82")))]
    match crate::bundle::BundleResolver::from_env(progname, engine)? {
        Ok(r) => return Some(Box::new(r)),
        Err(e) => {
            eprintln!("flashtex: bundle: {e}");
            return None;
        }
    }
    #[allow(unreachable_code)]
    {
        let _ = (progname, engine);
        None
    }
}

/// The process's resolver, per the table in the module documentation.
/// `progname` is kpathsea's program name (`tex` for the TeX82 engine,
/// `pdflatex` once pdftex.web is in).
pub fn default_resolver(progname: &str, engine: &str) -> Box<dyn FileResolver> {
    let which = std::env::var("FLASHTEX_RESOLVER").unwrap_or_default();
    #[cfg(feature = "kpathsea")]
    {
        if which == "kpathsea-self" {
            return Box::new(KpathseaResolver::for_self(progname, engine));
        }
        if which == "bundle" {
            if let Some(r) = fetched_bundle(progname, engine) {
                return r;
            }
            if let Some(d) = std::env::var_os("FLASHTEX_BUNDLE") {
                return Box::new(KpathseaResolver::for_bundle(
                    Path::new(&d),
                    progname,
                    engine,
                ));
            }
        }
        if which.is_empty() || which == "kpathsea" {
            if let Some(bin) = find_texlive_bin() {
                return Box::new(KpathseaResolver::for_texlive(&bin, progname, engine));
            }
            // No TeX Live: the bundle, if one is configured (DESIGN.md 4.4).
            if let Some(r) = fetched_bundle(progname, engine) {
                return r;
            }
        }
    }
    let _ = (progname, engine);
    Box::new(CwdResolver {
        dot: which == "cwd-kpse",
    })
}

#[cfg(test)]
mod tests {
    use super::kpse_absolute_p;

    /// kpathsea's `kpathsea_absolute_p (kpse, name, true)` with `DOSISH`
    /// defined (Windows; config.h lines 37-42), every expectation read off
    /// third_party/kpathsea/absolute.c and c-pathch.h:
    /// - absolute.c 38: `IS_DIR_SEP (*filename)`, and c-pathch.h 38 makes
    ///   `IS_DIR_SEP` both `/` and `\` under DOSISH;
    /// - absolute.c 41: `*filename && IS_DEVICE_SEP (filename[1])`, any
    ///   first byte ("Novell allows non-alphanumeric drive letters"), with
    ///   c-pathch.h 35 `IS_DEVICE_SEP(ch) ((ch) == ':')`;
    /// - absolute.c 43-47 (WIN32): UNC names, which line 38 already covers;
    /// - absolute.c 53-62: explicitly relative, `.` then a separator, or `..`
    ///   then a separator (either separator under DOSISH).
    #[test]
    fn dosish_names_as_absolute_c_classifies_them() {
        let yes = [
            "/x",            // 38
            "\\x",           // 38: `\` is IS_DIR_SEP under DOSISH
            "C:x",           // 41: drive-relative
            "C:\\x",         // 41
            "c:/x",          // 41
            "1:x",           // 41: any first byte
            "x:",            // 41: filename[1] is `:`
            "\\\\server\\x", // 38 (and 45)
            "//server/x",    // 38 (and 46)
            "./x",           // 61
            ".\\x",          // 61: `\` is IS_DIR_SEP
            "../x",          // 62
            "..\\x",         // 62
            ".\\sub\\x",     // 61
        ];
        let no = [
            "",         // 38: *filename is NUL; 41: *filename is false
            "x",        // nothing matches
            "sub\\x",   // only the first bytes are looked at
            "sub/x",    // likewise
            ".x",       // 61: filename[1] is not a separator
            "..x",      // 62: filename[2] is not a separator
            ".",        // 61: filename[1] is NUL
            "..",       // 62: filename[2] is NUL
            "...\\x",   // 62: filename[2] is `.`
            ":x",       // 41: filename[1] is `x`
            "\u{e9}:x", // 41 looks at bytes: filename[1] is 0xA9, not `:`
        ];
        for n in yes {
            assert!(
                kpse_absolute_p(n, true),
                "DOSISH {n:?}: absolute.c: not searched"
            );
        }
        for n in no {
            assert!(
                !kpse_absolute_p(n, true),
                "DOSISH {n:?}: absolute.c: searched"
            );
        }
    }

    /// The same function without DOSISH (Unix): `IS_DIR_SEP` is `/` only
    /// (c-pathch.h 55-61) and `IS_DEVICE_SEP` is 0 (c-pathch.h 66-67). What
    /// kpsewhich does with such names is checked in tests/resolver_cwd.rs.
    #[test]
    fn unix_names_as_absolute_c_classifies_them() {
        for n in ["/x", "//x", "./x", "../x", ".//x"] {
            assert!(kpse_absolute_p(n, false), "Unix {n:?}: not searched");
        }
        for n in [
            "", "x", "sub/x", "\\x", ".\\x", "..\\x", "C:x", "C:\\x", ".x", "..x", ".",
        ] {
            assert!(!kpse_absolute_p(n, false), "Unix {n:?}: searched");
        }
    }
}
