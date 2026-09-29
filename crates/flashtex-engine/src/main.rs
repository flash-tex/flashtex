//! INITEX driver.
//!
//! `flashtex-initex '\relax\end'` or `flashtex-initex story.tex`; with no
//! argument it prompts on the terminal exactly as TeX does.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        flashtex_engine::system::set_command_line(vec![args.join(" ").into_bytes()]);
    }
    let mut g = flashtex_engine::Globals::new();
    g.tex_body();
}
