//! `flashtex-xetex`: the XeTeX port as a program (`flashtex_xetex::driver`).

fn main() {
    // Arguments that are not UTF-8 are read with replacement characters.
    let argv: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    flashtex_xetex::driver::run(argv)
}
