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
}

// ---------------------------------------------------------------------------

/// `tex.web`'s own behaviour: the name as given, relative to the working
/// directory, then each directory of a colon-separated variable for the format
/// (`FLASHTEX_INPUTS`, `FLASHTEX_TFM_PATH`, `FLASHTEX_FORMATS`). The trip test
/// uses this: tripman.tex defines it on files in the current area.
///
/// With `dot` set, a name without a directory that is found in the working
/// directory comes back as `./name`, which is what kpathsea returns for a
/// search path of `.` (the e-trip test's texmf.cnf), and so what pdfTeX's log
/// shows.
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
        self.find_one(name, format)
    }
    fn describe(&self) -> String {
        "working directory (CwdResolver)".into()
    }
}

impl CwdResolver {
    fn find_one(&self, name: &str, format: Format) -> Option<PathBuf> {
        let p = Path::new(name);
        if p.is_file() {
            if self.dot && p.parent().is_some_and(|d| d.as_os_str().is_empty()) {
                return Some(Path::new(".").join(p));
            }
            return Some(p.to_path_buf());
        }
        if p.is_absolute() {
            return None;
        }
        let var = match format {
            Format::Tfm => "FLASHTEX_TFM_PATH",
            Format::Fmt => "FLASHTEX_FORMATS",
            _ => "FLASHTEX_INPUTS",
        };
        let path = std::env::var(var).ok()?;
        path.split(':')
            .filter(|d| !d.is_empty())
            .map(|d| Path::new(d).join(name))
            .find(|c| c.is_file())
    }
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
        let real = std::fs::canonicalize(self.bin.join("kpsewhich"))
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
    c.push((PathBuf::from("/Library/TeX/texbin"), "MacTeX".into()));
    let mut roots = vec![PathBuf::from("/usr/local/texlive")];
    if let Some(h) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(h).join("texlive"));
    }
    roots.push(PathBuf::from("/opt/texlive"));
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
    for d in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"] {
        c.push((PathBuf::from(d), "system directory".into()));
    }
    c
}

/// The user's TeX Live: the first of [`texlive_candidates`] that holds
/// `kpsewhich`, which is the `kpsewhich` a login shell would run unless
/// the user's shell start-up files change PATH.
pub fn discover_texlive() -> Option<TexLiveInstall> {
    texlive_candidates()
        .into_iter()
        .find(|(d, _)| d.join("kpsewhich").is_file())
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
            let argv0 = bin_dir.join("kpsewhich");
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
            let mut env: Vec<(String, String)> = vec![
                // An existing directory with no texmf.cnf in it would make
                // kpathsea warn; point it at the bundle and set everything.
                ("TEXMFCNF".into(), d.clone()),
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
                "TEXPOOL",
                "MFINPUTS",
                "TEXCONFIG",
            ] {
                env.push((v.into(), format!(".:{d}")));
            }
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
