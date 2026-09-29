//! The engine's driver, with web2c's options for how a run starts.
//!
//! `flashtex-initex '\relax\end'` or `flashtex-initex story.tex`; with no
//! argument it prompts on the terminal exactly as TeX does. Leading options
//! (web2c's names, one or two dashes, `=` or a separate value):
//!
//! | option | meaning |
//! |---|---|
//! | `-ini` | INITEX (the default when neither `-fmt` nor `-progname` is given) |
//! | `-fmt=NAME` | a production run whose default format is `NAME.fmt` |
//! | `-progname=NAME` | kpathsea's program name (search paths); without `-fmt` also the default format, as in web2c |
//! | `-etex` | enter extended mode without a `*` on the first line |
//!
//! The rest of the command line is the first line of input.

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let (mut ini, mut etex) = (false, false);
    let (mut fmt, mut progname): (Option<String>, Option<String>) = (None, None);
    while let Some(a) = args.first().cloned() {
        let opt = match a.strip_prefix("--").or_else(|| a.strip_prefix('-')) {
            Some(o) if !a.starts_with("-\\") => o.to_string(),
            _ => break,
        };
        args.remove(0);
        let (name, value) = match opt.split_once('=') {
            Some((n, v)) => (n.to_string(), Some(v.to_string())),
            None => (opt, None),
        };
        let mut value_of = |v: Option<String>| -> String {
            v.or_else(|| (!args.is_empty()).then(|| args.remove(0)))
                .unwrap_or_default()
        };
        match name.as_str() {
            "ini" => ini = true,
            "etex" => etex = true,
            "fmt" => fmt = Some(value_of(value)),
            "progname" => progname = Some(value_of(value)),
            other => {
                eprintln!("flashtex-initex: unknown option -{other}");
                std::process::exit(1);
            }
        }
    }
    if let Some(p) = &progname {
        // kpathsea's program name, read by system.rs's resolver.
        std::env::set_var("FLASHTEX_PROGNAME", p);
    }
    let production = !ini && (fmt.is_some() || progname.is_some());
    flashtex_engine::system::set_run_mode(!production, etex, fmt.or(progname));
    if !args.is_empty() {
        flashtex_engine::system::set_command_line(vec![args.join(" ").into_bytes()]);
    }
    let mut g = flashtex_engine::Globals::new();
    g.tex_body();
}
