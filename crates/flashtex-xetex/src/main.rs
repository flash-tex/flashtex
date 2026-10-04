//! The XeTeX port's driver: XeTeX's command line (the pdfTeX engine's
//! parser, after XeTeX's own options are taken out), then one run of the
//! program, then the process exit status.

use flashtex_xetex::system;

fn main() {
    // Arguments that are not UTF-8 are read with replacement characters.
    let mut argv: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    // XeTeX's own options (xetexextra.c). Phase S0 always writes XDV, as
    // `-no-pdf` asks; an output driver is phase 3.
    let mut keep = vec![];
    for (i, a) in argv.iter().enumerate() {
        let opt = a.trim_start_matches('-');
        let is_opt = i > 0 && a.starts_with('-');
        if is_opt
            && (opt == "no-pdf"
                || opt.starts_with("output-driver=")
                || opt.starts_with("papersize="))
        {
            continue;
        }
        if is_opt && opt == "version" {
            print!("{}", system::version_text());
            return;
        }
        keep.push(a.clone());
    }
    argv = keep;
    system::set_no_pdf(true);
    let mut o = flashtex_engine::cli::parse(&argv);
    // Invoked under its own name, the program is xetex (kpathsea's program
    // name selects the search paths).
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
    if !args.is_empty() {
        system::set_first_line(system::command_line(&args));
    }
    flashtex_engine::system::configure(o);
    let mut g = flashtex_xetex::Globals::new();
    g.tex_body();
    system::final_end(&mut g)
}
