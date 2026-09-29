//! The engine's driver: web2c's command line (texmfmp.c's `long_options`,
//! `parse_options` and `maininit`), so that the engine can stand in for
//! `pdftex`/`pdflatex` wherever a harness or a user runs them.
//!
//! Options are parsed as `getopt_long_only` parses them: one or two dashes,
//! the value after `=` or as the next argument, any unambiguous prefix of an
//! option name, and the first non-option argument ends the options. What
//! remains is the first line of input (texmfmp.c's `topenin`), as in
//! `flashtex-initex -ini '\relax\end'` or `flashtex-initex -fmt=pdflatex
//! story.tex`; with nothing left, TeX prompts on the terminal with `**`.
//!
//! | option | web2c's meaning |
//! |---|---|
//! | `-ini` | INITEX (dump formats); also implied by the program name `inipdftex` |
//! | `-fmt=NAME` | the format to load (`NAME.fmt`), also kpathsea's program name unless `-progname` is given |
//! | `-progname=NAME` | kpathsea's program name (search paths); the default format when neither `-fmt` nor a `%&` line names one |
//! | `-etex` | extended mode without a `*` on the first line (INITEX only) |
//! | `-interaction=MODE` | `batchmode`, `nonstopmode`, `scrollmode` or `errorstopmode` from the start |
//! | `-halt-on-error` | stop at the first error |
//! | `-file-line-error`, `-no-file-line-error` (and the `-style` spellings) | `file:line:error` messages; default from texmf.cnf's `file_line_error_style` |
//! | `-parse-first-line`, `-no-parse-first-line` | honour a `%&format` first line; default from texmf.cnf's `parse_first_line` |
//! | `-jobname=NAME` | the job name |
//! | `-output-directory=DIR` | where output files go (also read from `TEXMF_OUTPUT_DIRECTORY`) |
//! | `-translate-file=TCX`, `-default-translate-file=TCX`, `-8bit` | character translation and printability |
//! | `-shell-escape`, `-no-shell-escape`, `-shell-restricted` (`-enable-write18`, `-disable-write18`) | `\write18` on, off, or restricted to texmf.cnf's `shell_escape_commands`; by default texmf.cnf's `shell_escape` (`p` in TeX Live: restricted, DESIGN.md 4.5) |
//! | `-cnf-line=LINE` | a texmf.cnf line that overrides the files |
//! | `-output-format=dvi\|pdf`, `-draftmode` | pdfTeX's `\pdfoutput` and `\pdfdraftmode` from the start |
//! | `-output-comment=TEXT` | the DVI comment |
//! | `-recorder` | list every file opened in `JOBNAME.fls` |
//! | `-version`, `-help` | print and exit |
//!
//! Options pdfTeX has that this engine does not implement yet (`-synctex`,
//! `-src-specials`, `-mltex`, `-enc`, `-ipc`, `-ipc-start`, `-mktex`,
//! `-no-mktex`, `-kpathsea-debug`, `-debug-format`) are refused with an
//! error instead of being ignored, so that no run silently differs from
//! pdfTeX's.

use flashtex_engine::system::{self, Interaction, RunOptions, Shell};

/// `long_options[]` of texmfmp.c for pdfTeX: name and whether it takes a
/// value (`1`), none (`0`) or an optional one given only with `=` (`2`).
const OPTIONS: &[(&str, u8)] = &[
    ("fmt", 1),
    ("efmt", 1),
    ("cnf-line", 1),
    ("help", 0),
    ("ini", 0),
    ("interaction", 1),
    ("halt-on-error", 0),
    ("kpathsea-debug", 1),
    ("progname", 1),
    ("recorder", 0),
    ("version", 0),
    ("ipc", 0),
    ("ipc-start", 0),
    ("mltex", 0),
    ("enc", 0),
    ("etex", 0),
    ("output-comment", 1),
    ("draftmode", 0),
    ("output-format", 1),
    ("shell-escape", 0),
    ("no-shell-escape", 0),
    ("enable-write18", 0),
    ("disable-write18", 0),
    ("shell-restricted", 0),
    ("debug-format", 0),
    ("src-specials", 2),
    ("synctex", 1),
    ("file-line-error-style", 0),
    ("no-file-line-error-style", 0),
    ("file-line-error", 0),
    ("no-file-line-error", 0),
    ("jobname", 1),
    ("output-directory", 1),
    ("parse-first-line", 0),
    ("no-parse-first-line", 0),
    ("translate-file", 1),
    ("default-translate-file", 1),
    ("8bit", 0),
    ("mktex", 1),
    ("no-mktex", 1),
];

/// Options this engine refuses (see the module documentation).
const NOT_IMPLEMENTED: &[&str] = &[
    "kpathsea-debug",
    "ipc",
    "ipc-start",
    "mltex",
    "enc",
    "debug-format",
    "src-specials",
    "synctex",
    "mktex",
    "no-mktex",
];

/// `getopt_long_only`'s lookup: an exact name, else the one option the
/// argument is a prefix of.
fn lookup(name: &str) -> Result<(&'static str, u8), String> {
    if let Some(&(n, a)) = OPTIONS.iter().find(|(n, _)| *n == name) {
        return Ok((n, a));
    }
    let hits: Vec<_> = OPTIONS
        .iter()
        .filter(|(n, _)| n.starts_with(name))
        .collect();
    match hits.as_slice() {
        [(n, a)] => Ok((n, *a)),
        [] => Err(format!("unrecognized option '-{name}'")),
        _ => Err(format!("option '-{name}' is ambiguous")),
    }
}

fn main() {
    // Arguments that are not UTF-8 are read with replacement characters.
    let argv: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let mut o = RunOptions::new(argv.first().map(String::as_str).unwrap_or("pdftex"));
    let mut i = 1;
    while i < argv.len() {
        let a = &argv[i];
        if a == "--" {
            i += 1;
            break;
        }
        // An argument that starts with `-` (but is not `-` alone) is an
        // option; `+` in getopt's option string stops at the first operand.
        let Some(body) = a.strip_prefix("--").or_else(|| a.strip_prefix('-')) else {
            break;
        };
        if body.is_empty() {
            break;
        }
        i += 1;
        let (name, inline) = match body.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (body, None),
        };
        let (name, arity) = match lookup(name) {
            Ok(x) => x,
            Err(e) => {
                // getopt reports and carries on.
                eprintln!("{}: {e}", o.invocation_name);
                continue;
            }
        };
        let value = match (arity, inline) {
            (0, Some(_)) => {
                eprintln!(
                    "{}: option '-{name}' doesn't allow an argument",
                    o.invocation_name
                );
                continue;
            }
            (1, None) => match argv.get(i) {
                Some(v) => {
                    i += 1;
                    Some(v.clone())
                }
                None => {
                    eprintln!(
                        "{}: option '-{name}' requires an argument",
                        o.invocation_name
                    );
                    continue;
                }
            },
            (_, v) => v,
        };
        if NOT_IMPLEMENTED.contains(&name) {
            eprintln!(
                "{}: option -{name} is not implemented by this engine yet",
                o.invocation_name
            );
            std::process::exit(1);
        }
        let v = value.unwrap_or_default();
        match name {
            "fmt" | "efmt" => o.dump_name = Some(v),
            "cnf-line" => o.cnf_lines.push(v),
            "help" => {
                print!("{}", system::HELP);
                std::process::exit(0);
            }
            "ini" => o.ini = true,
            "interaction" => match v.as_str() {
                "batchmode" => o.interaction = Interaction::Batch,
                "nonstopmode" => o.interaction = Interaction::Nonstop,
                "scrollmode" => o.interaction = Interaction::Scroll,
                "errorstopmode" => o.interaction = Interaction::ErrorStop,
                _ => eprintln!(
                    "{}: Ignoring unknown argument `{v}' to --interaction",
                    o.invocation_name
                ),
            },
            "halt-on-error" => o.halt_on_error = true,
            "progname" => o.user_progname = Some(v),
            "recorder" => o.recorder = true,
            "version" => {
                print!("{}", system::version_text());
                std::process::exit(0);
            }
            "etex" => o.etex = true,
            "output-comment" => {
                // texmfmp.c truncates to 255 characters with a warning.
                let mut v = v;
                if v.len() >= 256 {
                    eprintln!(
                        "{}: Comment truncated to 255 characters from {}. ({v})",
                        o.invocation_name,
                        v.len()
                    );
                    v.truncate(255);
                }
                o.output_comment = Some(v);
            }
            "draftmode" => o.draftmode = true,
            "output-format" => match v.as_str() {
                "dvi" => o.output_format = Some(0),
                "pdf" => o.output_format = Some(2),
                _ => eprintln!(
                    "{}: Ignoring unknown value `{v}' for --output-format",
                    o.invocation_name
                ),
            },
            "shell-escape" | "enable-write18" => o.shell = Shell::On,
            "no-shell-escape" | "disable-write18" => o.shell = Shell::Off,
            "shell-restricted" => o.shell = Shell::Restricted,
            "file-line-error-style" | "file-line-error" => o.file_line_error = 1,
            "no-file-line-error-style" | "no-file-line-error" => o.file_line_error = -1,
            "jobname" => match system::normalize_quotes(&v) {
                Some(j) => o.job_name = Some(j),
                None => {
                    eprintln!("! Unbalanced quotes in jobname {v}");
                    std::process::exit(1);
                }
            },
            "output-directory" => o.output_directory = Some(v),
            "parse-first-line" => o.parse_first_line = 1,
            "no-parse-first-line" => o.parse_first_line = -1,
            "translate-file" => o.translate_filename = Some(v),
            "default-translate-file" => o.default_translate_filename = Some(v),
            "8bit" => o.eight_bit = true,
            _ => unreachable!("option table and match disagree on -{name}"),
        }
    }
    o.args = argv[i.min(argv.len())..].to_vec();
    o.first_arg = argv.get(1).cloned();
    system::configure(o);
    // The preview's display list, when the engine host asked for one.
    #[cfg(not(feature = "tex82"))]
    flashtex_engine::displaylist::init_from_env();
    let mut g = flashtex_engine::Globals::new();
    g.tex_body();
    #[cfg(not(feature = "tex82"))]
    flashtex_engine::displaylist::finish();
    // The end of the main program: tex.ch's `do_final_end`, whose exit
    // status says whether there was an error.
    system::final_end(&mut g)
}
