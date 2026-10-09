//! The engine's driver: XeTeX's command line (the pdfTeX engine's parser,
//! after XeTeX's own options are taken out), then one run of the program,
//! then the process exit status. `flashtex-xetex` is this program;
//! `flashtex-host-unicode` runs it when it is invoked as `xetex` or
//! `xelatex` (the format cache's INITEX runs and each compile).

use crate::system;

/// Run the engine with `argv` (as the process got it) and exit.
pub fn run(argv: Vec<String>) -> ! {
    // A child of `flashtex-host-unicode` ends with it (`host::proc`).
    #[cfg(unix)]
    crate::host::proc::watch_lifeline();
    // XeTeX's own options (xetexextra.c). `-no-pdf` writes XDV, as xetex
    // does; without it the PDF is FlashTeX's own (`crate::out`), never
    // xdvipdfmx's, so `-output-driver` has nothing to name.
    let mut keep = vec![];
    let mut no_pdf = false;
    for (i, a) in argv.iter().enumerate() {
        let opt = a.trim_start_matches('-');
        let is_opt = i > 0 && a.starts_with('-');
        if is_opt && opt == "no-pdf" {
            no_pdf = true;
            continue;
        }
        if is_opt && (opt.starts_with("output-driver=") || opt.starts_with("papersize=")) {
            continue;
        }
        if is_opt && opt == "version" {
            print!("{}", system::version_text());
            std::process::exit(0);
        }
        keep.push(a.clone());
    }
    let mut o = flashtex_engine::cli::parse(&keep);
    // Invoked under its own name (which kpathsea's rule reads as `pdftex`),
    // the program is xetex: kpathsea's program name selects the search
    // paths. As `xelatex` it is xelatex.
    if o.argv0_program == "pdftex" {
        o.argv0_program = "xetex".into();
    }
    // texmfmp.c's `topenin`: the arguments after the options are the first
    // line (a first file name with a space is quoted, as `get_input_file_name`
    // makes it).
    let mut args = o.args.clone();
    if let Some(a) = args.first().cloned() {
        if !a.starts_with('&') && !a.starts_with('\\') {
            if let Some(q) = flashtex_engine::system::normalize_quotes(&a) {
                args[0] = q;
            }
        }
    }
    flashtex_engine::system::configure(o);
    let mut g = crate::Globals::new();
    system::set_no_pdf(&mut g, no_pdf);
    if !no_pdf {
        g.host.out = Some(Box::new(crate::out::Output::from_env()));
    }
    if !args.is_empty() {
        system::set_first_line(&mut g, system::command_line(&args));
    }
    g.tex_body();
    system::final_end(&mut g)
}
