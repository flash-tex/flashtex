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
//! | `bundle` | kpathsea over the flat directory in `FLASHTEX_BUNDLE` |
//! | unset or `kpathsea` | kpathsea over the TeX Live found by [`find_texlive_bin`], else `cwd` |
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
    /// One line for logs and error messages.
    fn describe(&self) -> String;
}

// ---------------------------------------------------------------------------

/// `tex.web`'s own behaviour: the name as given, relative to the working
/// directory, then each directory of a colon-separated variable for the format
/// (`FLASHTEX_INPUTS`, `FLASHTEX_TFM_PATH`, `FLASHTEX_FORMATS`). The trip test
/// uses this: tripman.tex defines it on files in the current area.
pub struct CwdResolver;

impl FileResolver for CwdResolver {
    fn find(&mut self, name: &str, format: Format) -> Option<PathBuf> {
        let p = Path::new(name);
        if p.is_file() {
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
    fn describe(&self) -> String {
        "working directory (CwdResolver)".into()
    }
}

// ---------------------------------------------------------------------------

/// Directories to probe for a TeX Live `bin` directory when there is no shell
/// environment to consult (a GUI app inherits launchd's, with a bare PATH).
/// `FLASHTEX_TEXLIVE_BIN` overrides.
pub fn find_texlive_bin() -> Option<PathBuf> {
    let has_kpsewhich = |d: &Path| d.join("kpsewhich").is_file();
    if let Some(d) = std::env::var_os("FLASHTEX_TEXLIVE_BIN") {
        let d = PathBuf::from(d);
        return has_kpsewhich(&d).then_some(d);
    }
    let mut cands: Vec<PathBuf> = vec![PathBuf::from("/Library/TeX/texbin")];
    // /usr/local/texlive/<year>/bin/<arch>, newest year first.
    if let Ok(rd) = std::fs::read_dir("/usr/local/texlive") {
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
                cands.extend(arches);
            }
        }
    }
    cands.extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"].map(PathBuf::from));
    cands.into_iter().find(|d| has_kpsewhich(d))
}

#[cfg(feature = "kpathsea")]
pub use kpse::KpathseaResolver;

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
        ) -> *mut c_void;
        fn flashtex_kpse_format(k: *mut c_void, name: *const c_char) -> c_int;
        fn flashtex_kpse_find(k: *mut c_void, name: *const c_char, format: c_int) -> *mut c_char;
        fn flashtex_kpse_var_value(k: *mut c_void, var: *const c_char) -> *mut c_char;
        fn flashtex_kpse_free(p: *mut c_void);
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
            )
        }

        fn new(
            argv0: &Path,
            progname: &str,
            engine: &str,
            env: &[(String, String)],
            what: String,
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
            let k = unsafe { flashtex_kpse_new(a.as_ptr(), p.as_ptr(), e.as_ptr(), ptrs.as_ptr()) };
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
        fn describe(&self) -> String {
            self.what.clone()
        }
    }
}

/// The process's resolver, per the table in the module documentation.
/// `progname` is kpathsea's program name (`tex` for the TeX82 engine,
/// `pdflatex` once pdftex.web is in).
pub fn default_resolver(progname: &str, engine: &str) -> Box<dyn FileResolver> {
    let which = std::env::var("FLASHTEX_RESOLVER").unwrap_or_default();
    #[cfg(feature = "kpathsea")]
    {
        if which == "bundle" {
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
        }
    }
    let _ = (which, progname, engine);
    Box::new(CwdResolver)
}
